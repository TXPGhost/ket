use smallvec::SmallVec;
use thiserror::Error;

use crate::{
    arena::{Arena, Ref},
    lexer::{Token, TokenKinds},
};

pub enum AstNode {
    Ident {
        tok: Ref<Token>,
    },
    String {
        tok: Ref<Token>,
    },
    Integer {
        tok: Ref<Token>,
    },
    Float {
        tok: Ref<Token>,
    },
    Infix {
        kind: InfixKind,
        tok: Ref<Token>,
    },
    Call {
        func: Ref<Ast>,
        args: SmallVec<[Ref<Ast>; 5]>,
    },
    Func {
        params: SmallVec<[Ref<Ast>; 5]>,
        ty: Option<Ref<Ast>>,
        body: Option<Ref<Ast>>,
    },
    Block {
        stmts: SmallVec<[Ref<Ast>; 6]>,
    },
}

pub enum InfixKind {}

pub struct Ast;
pub type AstNodes = Arena<Ast, AstNode>;

#[derive(Debug, Error)]
pub enum ParseError {}

pub fn parse(kinds: &TokenKinds) -> Result<(), ParseError> {
    Ok(())
}
