use std::thread;

use parse_frequency::Frequency;
use std::time::Instant;

use super::{Instruction, Limb, Opcode, Program};

pub struct ExecutionContext {
    pub stack: Vec<usize>,
    pub instruction_pointer: usize,
    pub pointer: usize,
    pub memory: Vec<u8>,
}

pub struct Engine {
    limbs: Vec<Limb>,
    frequency: Frequency,
    context: ExecutionContext,
    program: Program,
    commands_executed: u64,
}

impl Engine {
    pub fn new(program: Program, memory: usize, frequency: Frequency) -> Self {
        Engine {
            limbs: Vec::new(),
            frequency,
            context: ExecutionContext {
                stack: Vec::new(),
                instruction_pointer: 0,
                pointer: 0,
                memory: vec![0; memory],
            },
            program,
            commands_executed: 0,
        }
    }

    pub fn run(&mut self) {
        let target_dt = self.frequency.as_duration();

        loop {
            let start = Instant::now();
            if self.context.instruction_pointer >= self.program.len() {
                break;
            }

            if !self.tick() {
                break;
            }

            let elapsed = start.elapsed();

            if elapsed < target_dt {
                thread::sleep(target_dt - elapsed);
            }
        }
    }

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
        let opcode = match self.get_instruction() {
            Some(instr) => &instr.opcode,
            None => return false,
        };

        match opcode {
            Opcode::AddValue(n) => {
                let value = self.context.memory[self.context.pointer] as i16 + *n;
                self.context.memory[self.context.pointer] = value as u8;
            }
            Opcode::MovePtr(n) => {
                let new_pointer = self.context.pointer as isize + *n;
                if new_pointer < 0 || new_pointer >= self.context.memory.len() as isize {
                    return false; // Pointer out of bounds
                }
                self.context.pointer = new_pointer as usize;
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
            Opcode::LoopStart(end) => {
                if self.context.memory[self.context.pointer] == 0 {
                    self.context.instruction_pointer = *end;
                } else {
                    self.context.stack.push(*end);
                }
            }
            Opcode::LoopEnd(start) => {
                if self.context.memory[self.context.pointer] != 0 {
                    self.context.instruction_pointer = *start;
                } else {
                    self.context.stack.pop();
                }
            }
        }

        self.context.instruction_pointer += 1;
        true
    }

    #[must_use]
    pub fn get_instruction(&self) -> Option<&Instruction> {
        self.program
            .get_instructions()
            .get(self.context.instruction_pointer)
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
            stack: Vec::new(),
            instruction_pointer: 0,
            pointer: 0,
            memory: vec![0; self.context.memory.len()],
        };
    }

    pub fn set_context(&mut self, context: ExecutionContext) {
        self.context = context;
    }

    pub fn get_program(&self) -> &Program {
        &self.program
    }

    #[must_use]
    pub fn get_context(&self) -> &ExecutionContext {
        &self.context
    }
}
