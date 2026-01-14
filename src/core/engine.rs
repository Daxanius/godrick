use super::{Instruction, Limb, Opcode, Program};
use parse_frequency::Frequency;

pub struct ExecutionContext {
    pub instruction_pointer: usize,
    pub pointer: usize,
    pub memory: Box<[u8]>,
}

pub struct Engine {
    limbs: Vec<Limb>,
    frequency: Frequency,
    context: ExecutionContext,
    program: Program,
    commands_executed: u64,
}

impl Engine {
    #[must_use]
    pub fn new(program: Program, memory: usize, frequency: Frequency) -> Self {
        Engine {
            limbs: Vec::new(),
            frequency,
            context: ExecutionContext {
                // stack: Vec::with_capacity(stack),
                instruction_pointer: 0,
                pointer: 0,
                memory: vec![0; memory].into(),
            },
            program,
            commands_executed: 0,
        }
    }

    pub fn run(&mut self) {
        // let target_dt = self.frequency.as_duration();

        loop {
            // let start = Instant::now();
            if self.context.instruction_pointer >= self.program.len() {
                break;
            }

            if !self.tick() {
                break;
            }

            // let elapsed = start.elapsed();

            // if elapsed < target_dt {
            //     thread::sleep(target_dt - elapsed);
            // }
        }
    }

    #[must_use]
    pub fn get_commands_executed(&self) -> u64 {
        self.commands_executed
    }

    /// Ticks the engine, executing one instruction and updating the context.
    /// Returns true if an instruction was executed, false if the program has finished
    pub fn tick(&mut self) -> bool {
        for limb in &self.limbs {
            limb.tick(&mut self.context.memory, self.context.pointer);
        }

        self.execute_instruction()
    }

    fn execute_instruction(&mut self) -> bool {
        let instruction = &self.program.get_instructions()[self.context.instruction_pointer];

        match instruction.opcode {
            Opcode::AddValue(n) => {
                self.context.memory[self.context.pointer] =
                    self.context.memory[self.context.pointer].wrapping_add(n);
            }
            Opcode::SubValue(n) => {
                self.context.memory[self.context.pointer] =
                    self.context.memory[self.context.pointer].wrapping_sub(n);
            }
            Opcode::Forward(n) => {
                self.context.pointer = self.context.pointer.wrapping_add(n);
            }
            Opcode::Reverse(n) => {
                self.context.pointer = self.context.pointer.wrapping_sub(n);
            }
            Opcode::Output => {
                let value = self.context.memory[self.context.pointer];
                print!("{}", value as char);
            }
            Opcode::Input => {
                let mut input = String::new();
                std::io::stdin()
                    .read_line(&mut input)
                    .expect("Failed to read input");
                if let Some(first_char) = input.chars().next() {
                    self.context.memory[self.context.pointer] = first_char as u8;
                }
            }
            Opcode::LoopStart(jump_to) => {
                if self.context.memory[self.context.pointer] == 0 {
                    self.context.instruction_pointer = jump_to;
                }
            }
            Opcode::LoopEnd(jump_to) => {
                if self.context.memory[self.context.pointer] != 0 {
                    self.context.instruction_pointer = jump_to;
                }
            }
            Opcode::ClearCell => {
                self.context.memory[self.context.pointer] = 0;
            }
            Opcode::MoveRight { right, factor } => {
                let src_val = self.context.memory[self.context.pointer];
                let dest_index = self.context.pointer + right;
                self.context.memory[dest_index] =
                    self.context.memory[dest_index].wrapping_add(src_val.wrapping_mul(factor));
                self.context.memory[self.context.pointer] = 0;
            }
            Opcode::MoveLeft { left, factor } => {
                let src_val = self.context.memory[self.context.pointer];
                // println!("{left}, {}: {}", self.context.pointer, instruction.position);
                let dest_index = self.context.pointer - left;
                self.context.memory[dest_index] =
                    self.context.memory[dest_index].wrapping_add(src_val.wrapping_mul(factor));
                self.context.memory[self.context.pointer] = 0;
            }
            Opcode::FastZeroRight(n) => {
                while self.context.memory[self.context.pointer] != 0 {
                    self.context.pointer += n;
                }
            }
            Opcode::FastZeroLeft(n) => {
                while self.context.memory[self.context.pointer] != 0 {
                    self.context.pointer -= n;
                }
            }
        }

        self.context.instruction_pointer += 1;
        self.commands_executed += 1;
        true
    }

    #[inline]
    #[must_use]
    pub fn get_instruction(&self) -> Option<&Instruction> {
        self.program.at(self.context.instruction_pointer)
    }

    pub fn graft(&mut self, limb: Limb) {
        limb.init();
        self.limbs.push(limb);
    }

    pub fn load_program(&mut self, program: Program) {
        self.program = program;
    }

    pub fn reset_context(&mut self) {
        self.context = ExecutionContext {
            instruction_pointer: 0,
            pointer: 0,
            memory: vec![0; self.context.memory.len()].into(),
        };
    }

    pub fn set_context(&mut self, context: ExecutionContext) {
        self.context = context;
    }

    #[inline]
    #[must_use]
    pub fn get_program(&self) -> &Program {
        &self.program
    }

    #[inline]
    #[must_use]
    pub fn get_context(&self) -> &ExecutionContext {
        &self.context
    }
}
