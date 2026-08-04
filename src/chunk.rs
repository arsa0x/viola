use crate::native::{NativeId, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    Constant(u16),
    GetLocal(u16),
    SetLocal(u16),

    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    Not,
    Neg,

    Jump(i16),
    JumpIfFalse(i16),

    CallNative { id: NativeId, argc: u8 },

    MakeArray(u16),

    Pop,
    Ret,
}

#[derive(Debug, Default)]
pub struct Chunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>,
    pub lines: Vec<u16>,
    pub local_count: u16,

    pub name: Option<String>,
    pub triggers: Vec<String>,
    pub category: Option<String>,
}

impl Chunk {
    pub fn emit(&mut self, op: OpCode, line: u16) -> usize {
        self.code.push(op);
        self.lines.push(line);

        self.code.len() - 1
    }

    pub fn add_constant(&mut self, val: Value) -> u16 {
        if let Some(idx) = self.constants.iter().position(|v| *v == val) {
            return idx as u16;
        }

        self.constants.push(val);

        (self.constants.len() - 1) as u16
    }

    pub fn patch_jump(&mut self, pos: usize, target: usize) {
        let offset = target as i32 - (pos as i32 + 1);

        debug_assert!(
            offset >= i16::MIN as i32 && offset <= i16::MAX as i32,
            "jump offset overflow"
        );

        match &mut self.code[pos] {
            OpCode::Jump(j) | OpCode::JumpIfFalse(j) => *j = offset as i16,
            other => panic!("patch_jump is called for non-jump instructions: {other:?}"),
        }
    }
}
