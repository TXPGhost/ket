use smallvec::SmallVec;
use thiserror::Error;

use crate::{
    arena::{Arena, Id, World},
    error::{ErrorExt, ErrorKind, ErrorRef, Errors},
    lexer::{
        TokenId,
        TokenKind::{self, *},
        Tokens,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum AstKind {
    Ident,
    String,
    Integer,
    Float,
    Infix,
    Call,
    Func,
    Block,
    Proj,
    Struct,

    #[default]
    Error,
}

pub enum InfixKind {}

#[derive(Default, Debug)]
pub struct Ast {
    pub ids: World<Ast>,
    pub kinds: Arena<Ast, AstKind>,
    pub children: Arena<Ast, SmallVec<[AstId; 4]>>,
}
pub type AstId = Id<Ast>;

#[derive(Debug, Error)]
pub enum ParseError {}

impl TokenId {}

pub struct Parser<'a> {
    tokens: &'a Tokens,
    cursor: TokenId,
    ast: &'a mut Ast,
    errors: &'a mut Errors,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a Tokens, ast: &'a mut Ast, errors: &'a mut Errors) -> Self {
        Self {
            tokens,
            cursor: TokenId::new(0),
            ast,
            errors,
        }
    }

    pub fn parse(self) -> AstId {
        self.ast.ids.alloc() // TEMP
    }

    fn advance(&mut self) {
        self.cursor = self.cursor.next().unwrap();
    }

    fn matches<const N: usize>(&self, kinds: [TokenKind; N]) -> bool {
        for kind in kinds {
            if *self.cursor.get(&self.tokens.kinds) == kind {
                return true;
            }
        }
        false
    }

    fn try_consume<const N: usize>(&mut self, kinds: [TokenKind; N]) -> Result<(), ErrorRef> {
        for kind in kinds {
            if *self.cursor.get(&self.tokens.kinds) == kind {
                self.advance();
                return Ok(());
            }
        }
        Err(self.errors.log(
            ErrorKind::Parse,
            format!("Expected one of the following tokens: {:?}", kinds),
        ))
    }

    fn consume_many<const N: usize>(&mut self, kinds: [TokenKind; N]) -> usize {
        let mut count = 0;
        'outer: loop {
            for kind in kinds {
                if *self.cursor.get(&self.tokens.kinds) == kind {
                    self.advance();
                    count += 1;
                    continue 'outer;
                }
            }
            break;
        }
        count
    }

    fn parse_list<T>(
        &mut self,
        kind: AstKind,
        parser: impl Fn(&mut Self) -> Result<AstId, ErrorRef>,
    ) {
        let mut first = true;
        while self.matches([RParen, RSquare, RCurl]) {
            if !first {
                let _ = self
                    .try_consume([Comma, Newline, Semicolon])
                    .err_caused_by(ErrorKind::Parse, "missing list separator");
                self.consume_many([Newline]);
            }
        }
    }
}
