use colored::Colorize;
use smallvec::SmallVec;

use crate::{
    arena::{Arena, Id, World},
    file::Files,
    lexer::{Location, TokenKind},
};

#[derive(Default, Debug)]
pub struct Ast {
    pub ids: World<Ast>,
    pub kinds: Arena<Ast, AstKind>,
    pub children: Arena<Ast, SmallVec<[AstId; 4]>>,
    pub locations: Arena<Ast, Option<Location>>,
    pub idents: Arena<Ast, String>,
    pub literals: Arena<Ast, Option<Literal>>,
}
pub type AstId = Id<Ast>;

impl Ast {
    fn pretty_print_indented(&self, id: AstId, indent: usize, files: &Files) {
        let index_str = id.index().to_string();
        print!(
            "{} ",
            format!(
                "[{}{}]",
                " ".repeat(3_usize.saturating_sub(index_str.len())),
                id.index()
            )
            .bright_black()
        );
        let kind_str = format!("{:?}", id.get(&self.kinds));
        let len = indent * 2 + kind_str.len();
        print!("{}{} ", "  ".repeat(indent), kind_str.bold().magenta());
        let location = id.get(&self.locations);
        let ident = id.get(&self.idents);
        if !ident.is_empty() {
            print!("{} ", ident);
        }
        print!("{} ", " ".repeat(36_usize.saturating_sub(len)));
        Location::pretty_print_opt(location, files);
        for child in id.get(&self.children) {
            self.pretty_print_indented(*child, indent + 1, files);
        }
    }

    pub fn pretty_print(&self, id: AstId, files: &Files) {
        println!();
        self.pretty_print_indented(id, 0, files);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum AstKind {
    VIdent,
    TIdent,
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
    VField,
    TField,
    Arg,
    Optional,
    Bind,
    BindMut,
    Assign,
    If,
    IfElse,
    Infix(InfixKind),

    BuiltinI32,
    BuiltinF32,
    BuiltinString,
    BuiltinChar,
    BuiltinBool,
    BuiltinTrue,
    BuiltinFalse,

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
