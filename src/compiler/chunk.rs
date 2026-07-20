use std::fmt::{self, Display};

use crate::{compiler::opcode::OpCode, lexer::types::Span, vm::value::Value};

#[derive(Clone)]
pub struct Chunk {
    pub code: Vec<OpCode>,
    pub consts: Vec<Value>,
    pub spans: Vec<Span>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: vec![],
            consts: vec![],
            spans: vec![],
        }
    }

    pub fn emit(&mut self, opcode: OpCode, span: Span) {
        self.code.push(opcode);
        self.spans.push(span);
    }

    pub fn add_const(&mut self, val: Value) -> usize {
        let idx = self.consts.len();
        self.consts.push(val);
        idx
    }

    pub fn emit_jump(&mut self, opcode: OpCode, span: Span) -> usize {
        let offset = self.code.len();
        self.emit(opcode, span);
        offset
    }

    pub fn patch_jump(&mut self, offset: usize) {
        let dest = self.code.len();
        self.code[offset] = match &self.code[offset] {
            OpCode::Jump(_) => OpCode::Jump(dest),
            OpCode::JumpIfFalse(_) => OpCode::JumpIfFalse(dest),
            _ => panic!("patch_jump in invalid opcode"),
        };
    }

    /// Imprime o disassembly para stdout.
    pub fn disassemble(&self) {
        for offset in 0..self.code.len() {
            self.disassemble_instruction(offset);
        }
    }

    fn disassemble_instruction(&self, offset: usize) {
        let op = &self.code[offset];
        let span = &self.spans[offset];

        print!("{:04}  ", offset);
        print!("[{}]  ", span);

        match op {
            OpCode::Const(idx) => println!("CONST {}    ({})", idx, self.consts[*idx]),
            OpCode::Nil => println!("NIL"),
            OpCode::True => println!("TRUE"),
            OpCode::False => println!("FALSE"),
            OpCode::GetLocal(slot) => println!("GET_LOCAL {}", slot),
            OpCode::SetLocal(slot) => println!("SET_LOCAL {}", slot),
            OpCode::Add => println!("ADD"),
            OpCode::Sub => println!("SUB"),
            OpCode::Mul => println!("MUL"),
            OpCode::Div => println!("DIV"),
            OpCode::Mod => println!("MOD"),
            OpCode::Pow => println!("POW"),
            OpCode::Neg => println!("NEG"),
            OpCode::Eq => println!("EQ"),
            OpCode::Neq => println!("NEQ"),
            OpCode::Lt => println!("LT"),
            OpCode::Le => println!("LE"),
            OpCode::Gt => println!("GT"),
            OpCode::Ge => println!("GE"),
            OpCode::And => println!("AND"),
            OpCode::Or => println!("OR"),
            OpCode::Not => println!("NOT"),
            OpCode::BitAnd => println!("BIT_AND"),
            OpCode::BitOr => println!("BIT_OR"),
            OpCode::BitXor => println!("BIT_XOR"),
            OpCode::Shl => println!("SHL"),
            OpCode::Shr => println!("SHR"),
            OpCode::AsInt => println!("AS_INT"),
            OpCode::AsFloat => println!("AS_FLOAT"),
            OpCode::Jump(addr) => println!("JUMP {}", addr),
            OpCode::JumpIfFalse(addr) => println!("JUMP_IF_FALSE {}", addr),
            OpCode::Call(argc) => println!("CALL argc={}", argc),
            OpCode::Closure(idx, n_upv) => println!("CLOSURE chunk={}, n_upv={}", idx, n_upv),
            OpCode::Return => println!("RETURN"),
            OpCode::GetUpvalue(idx) => println!("GET_UPVALUE {}", idx),
            OpCode::SetUpvalue(idx) => println!("SET_UPVALUE {}", idx),
            OpCode::GetProperty(name) => println!("GET_PROPERTY `{}`", name),
            OpCode::SetProperty(name) => println!("SET_PROPERTY `{}`", name),
            OpCode::IndexGet => println!("INDEX_GET"),
            OpCode::IndexSet => println!("INDEX_SET"),
            OpCode::Array(n) => println!("ARRAY ({} items)", n),
            OpCode::Struct(name, field_count) => {
                println!("STRUCT `{}`, field_count={}", name, field_count)
            }
            OpCode::Pop => println!("POP"),
            OpCode::Increment(slot) => println!("INCREMENT {}", slot),
            OpCode::Decrement(slot) => println!("DECREMENT {}", slot),
            OpCode::Rotate(n) => println!("ROTATE {}", n),
        }
    }
}

impl Display for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for offset in 0..self.code.len() {
            let op = &self.code[offset];
            write!(f, "{:04}  ", offset)?;

            match op {
                OpCode::Const(idx) => writeln!(f, "CONST {}    ({})", idx, self.consts[*idx])?,
                OpCode::Nil => writeln!(f, "NIL")?,
                OpCode::True => writeln!(f, "TRUE")?,
                OpCode::False => writeln!(f, "FALSE")?,
                OpCode::GetLocal(slot) => writeln!(f, "GET_LOCAL {}", slot)?,
                OpCode::SetLocal(slot) => writeln!(f, "SET_LOCAL {}", slot)?,
                OpCode::Add => writeln!(f, "ADD")?,
                OpCode::Sub => writeln!(f, "SUB")?,
                OpCode::Mul => writeln!(f, "MUL")?,
                OpCode::Div => writeln!(f, "DIV")?,
                OpCode::Mod => writeln!(f, "MOD")?,
                OpCode::Pow => writeln!(f, "POW")?,
                OpCode::Neg => writeln!(f, "NEG")?,
                OpCode::Eq => writeln!(f, "EQ")?,
                OpCode::Neq => writeln!(f, "NEQ")?,
                OpCode::Lt => writeln!(f, "LT")?,
                OpCode::Le => writeln!(f, "LE")?,
                OpCode::Gt => writeln!(f, "GT")?,
                OpCode::Ge => writeln!(f, "GE")?,
                OpCode::And => writeln!(f, "AND")?,
                OpCode::Or => writeln!(f, "OR")?,
                OpCode::Not => writeln!(f, "NOT")?,
                OpCode::BitAnd => writeln!(f, "BIT_AND")?,
                OpCode::BitOr => writeln!(f, "BIT_OR")?,
                OpCode::BitXor => writeln!(f, "BIT_XOR")?,
                OpCode::Shl => writeln!(f, "SHL")?,
                OpCode::Shr => writeln!(f, "SHR")?,
                OpCode::AsInt => writeln!(f, "AS_INT")?,
                OpCode::AsFloat => writeln!(f, "AS_FLOAT")?,
                OpCode::Jump(addr) => writeln!(f, "JUMP {}", addr)?,
                OpCode::JumpIfFalse(addr) => writeln!(f, "JUMP_IF_FALSE {}", addr)?,
                OpCode::Call(argc) => writeln!(f, "CALL argc={}", argc)?,
                OpCode::Closure(idx, n_upv) => {
                    writeln!(f, "CLOSURE chunk={}, n_upv={}", idx, n_upv)?
                }
                OpCode::Return => writeln!(f, "RETURN")?,
                OpCode::GetUpvalue(idx) => writeln!(f, "GET_UPVALUE {}", idx)?,
                OpCode::SetUpvalue(idx) => writeln!(f, "SET_UPVALUE {}", idx)?,
                OpCode::GetProperty(name) => writeln!(f, "GET_PROPERTY `{}`", name)?,
                OpCode::SetProperty(name) => writeln!(f, "SET_PROPERTY `{}`", name)?,
                OpCode::IndexGet => writeln!(f, "INDEX_GET")?,
                OpCode::IndexSet => writeln!(f, "INDEX_SET")?,
                OpCode::Array(n) => writeln!(f, "ARRAY ({} items)", n)?,
                OpCode::Struct(name, field_count) => {
                    writeln!(f, "STRUCT `{}`, field_count={}", name, field_count)?
                }
                OpCode::Pop => writeln!(f, "POP")?,
                OpCode::Increment(slot) => writeln!(f, "INCREMENT {}", slot)?,
                OpCode::Decrement(slot) => writeln!(f, "DECREMENT {}", slot)?,
                OpCode::Rotate(n) => writeln!(f, "ROTATE {}", n)?,
            }
        }
        Ok(())
    }
}
