use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: Opcode,
    pub position: Position,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Line: {}, Column: {}", self.line, self.column)
    }
}

#[derive(Debug, Clone)]
pub enum Opcode {
    Reverse(usize),
    Forward(usize),
    AddValue(u8),
    SubValue(u8),
    Output,
    Input,
    LoopStart(usize),
    LoopEnd(usize),
    FastZeroLeft(usize),
    FastZeroRight(usize),
    ClearCell,
    MoveLeft { left: usize, factor: u8 },
    MoveRight { right: usize, factor: u8 },
}
