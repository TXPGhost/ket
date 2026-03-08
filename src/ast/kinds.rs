use crate::lexer::TokenKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum AstKind {
    LIdent,
    UIdent,
    Void,
    String,
    Char,
    None,
    Integer,
    Float,
    Call,
    Method,
    Group,
    Func,
    Block,
    Proj,
    Index,
    Struct,
    Tuple,
    Array,
    Repeat,
    Vector,
    Field,
    Arg,
    Optional,
    Bind,
    BindMut,
    Assign,
    If,
    IfElse,
    Infix(InfixKind),

    PrimitiveI32,
    PrimitiveF32,
    PrimitiveString,
    PrimitiveChar,

    #[default]
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InfixKind {
    Add,
    Sub,
    Mul,
    Div,
    Gt,
    Lt,
    Ge,
    Le,
    Eq,
    Ne,
}

impl InfixKind {
    pub fn ast(self) -> AstKind {
        AstKind::Infix(self)
    }

    pub fn tok(self) -> TokenKind {
        match self {
            InfixKind::Add => TokenKind::Plus,
            InfixKind::Sub => TokenKind::Minus,
            InfixKind::Mul => TokenKind::Times,
            InfixKind::Div => TokenKind::Divide,

            InfixKind::Gt => TokenKind::RAngle,
            InfixKind::Lt => TokenKind::LAngle,
            InfixKind::Ge => TokenKind::RAngleEquals,
            InfixKind::Le => TokenKind::LAngleEquals,
            InfixKind::Eq => TokenKind::EqualsEquals,
            InfixKind::Ne => TokenKind::NotEquals,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Char(u8),
}
