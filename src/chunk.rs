use crate::native::{NativeId, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    Const(u16),
    GetL(u16),
    SetL(u16),

    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Not,
    Neg,

    Jmp(i16),
    JmpF(i16),

    /// Short-circuit jump for `and`: if the value now on top of the stack
    /// is falsy, jump — *without* popping it, since that falsy value is
    /// the result of the whole `and` expression. If truthy, falls through
    /// to an explicit `Pop` (emitted right after) followed by the
    /// right-hand side's code.
    JmpFKeep(i16),

    /// Short-circuit jump for `or`: mirror of `JmpFKeep` — jumps (keeping
    /// the value) when the top of the stack is truthy, falls through to
    /// `Pop` + the right-hand side otherwise.
    JmpTKeep(i16),

    CallN {
        id: NativeId,
        argc: u8,
    },

    MkArr(u16),

    MkObj(u16),
    GetP(u16),
    SetP(u16),
    CallM {
        name_idx: u16,
        argc: u8,
    },

    Pop,
    Ret,
}

#[derive(Debug, Default)]
pub struct Chunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>,
    pub lines: Vec<u32>,
    pub local_count: u16,
    pub locals_by_name: std::collections::HashMap<String, u16>,
    pub object_layouts: Vec<Box<[u16]>>,

    pub name: Option<String>,
    pub triggers: Vec<String>,
    pub category: Option<String>,
}

impl Chunk {
    pub fn emit(&mut self, op: OpCode, line: u32) -> usize {
        self.code.push(op);
        self.lines.push(line);

        self.code.len() - 1
    }

    pub fn add_object_layout(&mut self, layout: Box<[u16]>) -> u16 {
        if let Some(idx) = self
            .object_layouts
            .iter()
            .position(|x| x.as_ref() == layout.as_ref())
        {
            return idx as u16;
        }

        self.object_layouts.push(layout);

        (self.object_layouts.len() - 1) as u16
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
            OpCode::Jmp(j) | OpCode::JmpF(j) => *j = offset as i16,
            other => panic!("patch_jump is called for non-jump instructions: {other:?}"),
        }
    }
}
