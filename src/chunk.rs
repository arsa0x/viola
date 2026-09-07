use crate::native::{NativeId, Value};

/// A bytecode operation executed by the virtual machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    /// Pushes a constant from the chunk's constant pool onto the stack.
    Const(u16),

    /// Loads a local variable onto the stack.
    GetL(u16),

    /// Stores the top stack value into a local variable.
    SetL(u16),

    /// Adds the top two stack values.
    Add,

    /// Subtracts the top stack value from the next value on the stack.
    Sub,

    /// Multiplies the top two stack values.
    Mul,

    /// Divides the next stack value by the top stack value.
    Div,

    /// Compares the top two stack values for equality.
    Eq,

    /// Compares the top two stack values for inequality.
    Ne,

    /// Compares whether the next stack value is less than the top value.
    Lt,

    /// Compares whether the next stack value is less than or equal to the top value.
    Le,

    /// Compares whether the next stack value is greater than the top value.
    Gt,

    /// Compares whether the next stack value is greater than or equal to the top value.
    Ge,

    /// Negates the top stack value.
    Not,

    /// Negates the top numeric stack value.
    Neg,

    /// Unconditionally jumps by a relative offset.
    Jmp(i16),

    /// Jumps by a relative offset if the top stack value is falsy.
    JmpF(i16),

    /// Short-circuit jump for `and`: if the value now on top of the stack
    /// is falsy, jumps without popping it, since that falsy value is the
    /// result of the whole `and` expression. If truthy, falls through to
    /// an explicit `Pop` emitted immediately afterward, followed by the
    /// right-hand side's code.
    JmpFKeep(i16),

    /// Short-circuit jump for `or`: if the value now on top of the stack
    /// is truthy, jumps without popping it, since that truthy value is the
    /// result of the whole `or` expression. If falsy, falls through to
    /// an explicit `Pop` emitted immediately afterward, followed by the
    /// right-hand side's code.
    JmpTKeep(i16),

    /// Calls a native function with the given identifier and argument count.
    CallN { id: NativeId, argc: u8 },

    /// Creates an array from the given number of stack values.
    MkArr(u16),

    /// Creates an object using an object layout from the chunk's layout table.
    MkObj(u16),

    /// Gets a property from the object on top of the stack.
    GetP(u16),

    /// Sets a property on the object on top of the stack.
    SetP(u16),

    /// Calls an object method with the given name and argument count.
    CallM { name_idx: u16, argc: u8 },

    /// Removes the top value from the stack.
    Pop,

    /// Returns from the current function.
    Ret,
}

/// A compiled unit of bytecode and its associated metadata.
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
