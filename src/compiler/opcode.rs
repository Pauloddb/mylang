#[derive(Debug, Clone)]
pub enum OpCode {
    // === Constantes ===
    Const(usize),
    Nil,
    True,
    False,

    // === Variáveis locais ===
    GetLocal(u8),
    SetLocal(u8),

    // === Aritmética ===
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Neg,

    // === Comparação ===
    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,

    // === Lógico ===
    And,
    Or,
    Not,

    // === Bitwise ===
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,

    // === Cast ===
    AsInt,
    AsFloat,

    // === Controle de fluxo ===
    Jump(usize),
    JumpIfFalse(usize),

    // === Funções ===
    Call(u8),
    Closure(usize, u8), // idx, n_upvalues
    Return,
    GetUpvalue(u8),
    SetUpvalue(u8),

    // === Propriedades ===
    GetProperty(String),
    SetProperty(String),

    // === Index ===
    IndexGet,
    IndexSet,

    // === Arrays / Structs ===
    Array(usize),
    Struct(String, u8),

    // === Stack ===
    Pop,
    Rotate(u8),

    // === ++ / -- ===
    Increment(u8),
    Decrement(u8),
}
