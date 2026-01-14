use std::ops::Index;

use colored::Colorize;
use std::fmt::Write;

use super::{Instruction, Opcode, Position};
use crate::{Error, Result};

pub struct Program {
    instructions: Vec<Instruction>,
    source: String,
}

impl Program {
    pub fn new(code: &str) -> Result<Self> {
        Self {
            instructions: Vec::new(),
            source: code.to_string(),
        }
        .parse(code)
        .optimize()
        .resolve_clears()
        // .resolve_move_left()
        .resolve_move_right()
        .resolve_fz_right()
        .resolve_fz_left()
        .resolve_loops()
    }

    #[inline]
    #[must_use]
    pub fn get_instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    #[inline]
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&Instruction> {
        self.instructions.get(index)
    }

    #[must_use]
    pub fn get_line(&self, index: usize, surrounding: bool) -> Option<String> {
        let position = &self.instructions.get(index)?.position;
        let line_num = position.line.saturating_sub(1);

        let lines: Vec<_> = self.source.lines().collect();

        let mut output = String::new();

        if surrounding && line_num >= 1 {
            output.push_str(lines[line_num - 1]);
            output.push('\n');
        }

        if let Some(current) = lines.get(line_num) {
            output.push_str(current);
            output.push('\n');
        }

        if surrounding && let Some(next) = lines.get(line_num + 1) {
            output.push_str(next);
            output.push('\n');
        }

        Some(output.trim_end().to_string())
    }

    fn make_error_message(&self, message: &str, position: &Position) -> String {
        let lines: Vec<_> = self.source.lines().collect();
        let line_num = position.line;
        let col_num = position.column;

        // Get the actual source line (if available)
        let src_line = lines.get(line_num.saturating_sub(1)).unwrap_or(&"");

        // Line number formatting
        let gutter_width = line_num.to_string().len();

        // Build the visual message
        let mut output = String::new();
        _ = writeln!(output, "{}", message.red());
        _ = writeln!(output, " {} {}", "-->".blue(), position.to_string().blue());
        _ = writeln!(output, "{line_num:>gutter_width$} | {}", src_line.white());
        _ = writeln!(
            output,
            "{:>gutter_width$} | {:>col$}{}",
            "",
            "",
            "^".red(),
            col = col_num - 1
        );

        output
    }

    fn resolve_loops(mut self) -> Result<Self> {
        let mut stack: Vec<usize> = Vec::new();

        for i in 0..self.instructions.len() {
            match self.instructions[i].opcode {
                Opcode::LoopStart(_) => {
                    stack.push(i);
                }

                Opcode::LoopEnd(_) => {
                    if let Some(start_index) = stack.pop() {
                        // Update both instructions
                        self.instructions[start_index].opcode = Opcode::LoopStart(i);
                        self.instructions[i].opcode = Opcode::LoopEnd(start_index);
                    } else {
                        return Err(Error::Program(self.make_error_message(
                            "Unmatched ']' found",
                            &self.instructions[i].position,
                        )));
                    }
                }
                _ => {}
            }
        }

        if !stack.is_empty() {
            let mut total_message: String = String::new();

            while let Some(index) = stack.pop() {
                let instruction = &self.instructions[index];
                let message = self.make_error_message("Unmatched '[' found", &instruction.position);
                total_message.push_str(&message);
            }

            return Err(Error::Program(total_message));
        }

        Ok(self)
    }

    fn resolve_move_right(mut self) -> Self {
        let mut optimized = Vec::with_capacity(self.instructions.len());
        let mut i = 0;

        while i < self.instructions.len() {
            // Look for loop: [SubValue(1), MovePtr(r), AddValue(n), MovePtr(-r)]
            if i + 5 < self.instructions.len()
                && let Opcode::LoopStart(_) = self.instructions[i].opcode
            {
                let a = &self.instructions[i + 1];
                let b = &self.instructions[i + 2];
                let c = &self.instructions[i + 3];
                let d = &self.instructions[i + 4];
                let e = &self.instructions[i + 5];

                if let (
                    Opcode::SubValue(1),
                    Opcode::Forward(r),
                    Opcode::AddValue(n),
                    Opcode::Reverse(l),
                    Opcode::LoopEnd(_),
                ) = (&a.opcode, &b.opcode, &c.opcode, &d.opcode, &e.opcode)
                    && l == r
                {
                    optimized.push(Instruction {
                        opcode: Opcode::MoveRight {
                            right: *r,
                            factor: *n,
                        },
                        position: self.instructions[i].position.clone(),
                    });

                    i += 6; // skip entire loop
                    continue;
                }

                if let (
                    Opcode::Forward(r),
                    Opcode::AddValue(n),
                    Opcode::Reverse(l),
                    Opcode::SubValue(1),
                    Opcode::LoopEnd(_),
                ) = (&a.opcode, &b.opcode, &c.opcode, &d.opcode, &e.opcode)
                    && l == r
                {
                    optimized.push(Instruction {
                        opcode: Opcode::MoveRight {
                            right: *r,
                            factor: *n,
                        },
                        position: self.instructions[i].position.clone(),
                    });

                    i += 6; // skip entire loop
                    continue;
                }
            }

            optimized.push(self.instructions[i].clone());
            i += 1;
        }

        self.instructions = optimized;
        self
    }

    fn resolve_fz_right(mut self) -> Self {
        let mut optimized = Vec::with_capacity(self.instructions.len());
        let mut i = 0;

        while i < self.instructions.len() {
            // Look for loop: [SubValue(1), MovePtr(r), AddValue(n), MovePtr(-r)]
            if i + 3 < self.instructions.len()
                && let Opcode::LoopStart(_) = self.instructions[i].opcode
            {
                let a = &self.instructions[i + 1];
                let b = &self.instructions[i + 2];

                if let (Opcode::Forward(n), Opcode::LoopEnd(_)) = (&a.opcode, &b.opcode) {
                    optimized.push(Instruction {
                        opcode: Opcode::FastZeroRight(*n),
                        position: self.instructions[i].position.clone(),
                    });

                    i += 3; // skip entire loop
                    continue;
                }
            }

            optimized.push(self.instructions[i].clone());
            i += 1;
        }

        self.instructions = optimized;
        self
    }

    fn resolve_fz_left(mut self) -> Self {
        let mut optimized = Vec::with_capacity(self.instructions.len());
        let mut i = 0;

        while i < self.instructions.len() {
            // Look for loop: [SubValue(1), MovePtr(r), AddValue(n), MovePtr(-r)]
            if i + 3 < self.instructions.len()
                && let Opcode::LoopStart(_) = self.instructions[i].opcode
            {
                let a = &self.instructions[i + 1];
                let b = &self.instructions[i + 2];

                if let (Opcode::Reverse(n), Opcode::LoopEnd(_)) = (&a.opcode, &b.opcode) {
                    optimized.push(Instruction {
                        opcode: Opcode::FastZeroLeft(*n),
                        position: self.instructions[i].position.clone(),
                    });

                    i += 3; // skip entire loop
                    continue;
                }
            }

            optimized.push(self.instructions[i].clone());
            i += 1;
        }

        self.instructions = optimized;
        self
    }

    fn resolve_move_left(mut self) -> Self {
        let mut optimized = Vec::with_capacity(self.instructions.len());
        let mut i = 0;

        while i < self.instructions.len() {
            if i + 5 < self.instructions.len()
                && let Opcode::LoopStart(_) = self.instructions[i].opcode
            {
                let a = &self.instructions[i + 1];
                let b = &self.instructions[i + 2];
                let c = &self.instructions[i + 3];
                let d = &self.instructions[i + 4];
                let e = &self.instructions[i + 5];

                if let (
                    Opcode::SubValue(1),
                    Opcode::Reverse(l),
                    Opcode::AddValue(n),
                    Opcode::Forward(r),
                    Opcode::LoopEnd(_),
                ) = (&a.opcode, &b.opcode, &c.opcode, &d.opcode, &e.opcode)
                    && l == r
                {
                    optimized.push(Instruction {
                        opcode: Opcode::MoveLeft {
                            left: *l,
                            factor: *n,
                        },
                        position: self.instructions[i].position.clone(),
                    });

                    i += 6; // skip entire loop
                    continue;
                }

                if let (
                    Opcode::Reverse(l),
                    Opcode::AddValue(n),
                    Opcode::Forward(r),
                    Opcode::SubValue(1),
                    Opcode::LoopEnd(_),
                ) = (&a.opcode, &b.opcode, &c.opcode, &d.opcode, &e.opcode)
                    && l == r
                {
                    optimized.push(Instruction {
                        opcode: Opcode::MoveLeft {
                            left: *l,
                            factor: *n,
                        },
                        position: self.instructions[i].position.clone(),
                    });

                    i += 6; // skip entire loop
                    continue;
                }
            }

            optimized.push(self.instructions[i].clone());
            i += 1;
        }

        self.instructions = optimized;
        self
    }

    fn resolve_clears(mut self) -> Self {
        let mut optimized = Vec::with_capacity(self.instructions.len());
        let mut i = 0;

        while i < self.instructions.len() {
            // Check for [-] or [+]
            if i + 2 < self.instructions.len() {
                let a = &self.instructions[i];
                let b = &self.instructions[i + 1];
                let c = &self.instructions[i + 2];

                let is_clear = matches!(a.opcode, Opcode::LoopStart(_))
                    && matches!(b.opcode, Opcode::AddValue(_) | Opcode::SubValue(_))
                    && matches!(c.opcode, Opcode::LoopEnd(_));

                if is_clear {
                    optimized.push(Instruction {
                        opcode: Opcode::ClearCell,
                        position: a.position.clone(), // point to '['
                    });

                    i += 3;
                    continue;
                }
            }

            optimized.push(self.instructions[i].clone());
            i += 1;
        }

        self.instructions = optimized;
        self
    }

    fn optimize(mut self) -> Self {
        let mut optimized = Vec::new();
        let mut accumulator: Option<Instruction> = None;

        for instruction in self.instructions {
            match (&instruction.opcode, &accumulator) {
                (
                    Opcode::Forward(n),
                    Some(Instruction {
                        opcode: Opcode::Forward(acc_n),
                        position,
                    }),
                ) => {
                    // Accumulate the pointer movement
                    let new = Instruction {
                        opcode: Opcode::Forward(acc_n + n),
                        position: position.clone(),
                    };
                    accumulator = Some(new);
                }
                (
                    Opcode::Reverse(n),
                    Some(Instruction {
                        opcode: Opcode::Reverse(acc_n),
                        position,
                    }),
                ) => {
                    // Accumulate the pointer movement
                    let new = Instruction {
                        opcode: Opcode::Reverse(acc_n + n),
                        position: position.clone(),
                    };
                    accumulator = Some(new);
                }
                (
                    Opcode::AddValue(n),
                    Some(Instruction {
                        opcode: Opcode::AddValue(acc_n),
                        position,
                    }),
                ) => {
                    // Accumulate the value addition
                    let new = Instruction {
                        opcode: Opcode::AddValue(acc_n + n),
                        position: position.clone(),
                    };
                    accumulator = Some(new);
                }
                (
                    Opcode::SubValue(n),
                    Some(Instruction {
                        opcode: Opcode::SubValue(acc_n),
                        position,
                    }),
                ) => {
                    // Accumulate the value addition
                    let new = Instruction {
                        opcode: Opcode::SubValue(acc_n + n),
                        position: position.clone(),
                    };
                    accumulator = Some(new);
                }
                (
                    Opcode::Forward(_)
                    | Opcode::Reverse(_)
                    | Opcode::AddValue(_)
                    | Opcode::SubValue(_),
                    _,
                ) => {
                    // Start new accumulation
                    if let Some(prev) = accumulator.take() {
                        optimized.push(prev);
                    }

                    accumulator = Some(instruction.clone());
                }
                _ => {
                    // Flush any accumulated instruction
                    if let Some(prev) = accumulator.take() {
                        optimized.push(prev);
                    }

                    // Add non-optimizable instruction
                    optimized.push(instruction.clone());
                }
            }
        }

        // Final flush
        if let Some(prev) = accumulator.take() {
            optimized.push(prev);
        }

        self.instructions = optimized;
        self
    }

    fn parse(mut self, code: &str) -> Self {
        let mut line = 1;
        let mut column = 1;

        for c in code.chars() {
            if c == '\n' {
                line += 1;
                column = 1;
            } else {
                if let Some(opcode) = Self::decode_character(c) {
                    self.instructions.push(Instruction {
                        opcode,
                        position: Position { line, column },
                    });
                }
                column += 1;
            }
        }

        self
    }

    fn decode_character(c: char) -> Option<Opcode> {
        match c {
            '>' => Some(Opcode::Forward(1)),
            '<' => Some(Opcode::Reverse(1)),
            '+' => Some(Opcode::AddValue(1)),
            '-' => Some(Opcode::SubValue(1)),
            '.' => Some(Opcode::Output),
            ',' => Some(Opcode::Input),
            '[' => Some(Opcode::LoopStart(0)),
            ']' => Some(Opcode::LoopEnd(0)),
            _ => None,
        }
    }
}

impl Index<usize> for Program {
    type Output = Instruction;

    fn index(&self, index: usize) -> &Self::Output {
        &self.instructions[index]
    }
}

impl IntoIterator for Program {
    type Item = Instruction;
    type IntoIter = std::vec::IntoIter<Instruction>;

    fn into_iter(self) -> Self::IntoIter {
        self.instructions.into_iter()
    }
}

impl TryFrom<String> for Program {
    type Error = Error;

    fn try_from(code: String) -> Result<Self> {
        Program::new(&code)
    }
}

impl TryFrom<&String> for Program {
    type Error = Error;

    fn try_from(code: &String) -> Result<Self> {
        Program::new(code)
    }
}
