use smallvec::SmallVec;
use thiserror::Error;

use crate::{
    arena::{Arena, Id, World},
    error::{ErrorId, ErrorKind, ErrorRef, Errors},
    lexer::{
        Location, TokenId,
        TokenKind::{self, *},
        Tokens,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum AstKind {
    LIdent,
    UIdent,
    Void,
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

    pub fn parse(mut self) -> AstId {
        let ast = self.parse_list(AstKind::Struct, Self::parse_field);
        // TODO: check for completeness
        ast
    }

    fn node(&mut self, kind: AstKind) -> AstId {
        self.ast.ids.alloc().put(&mut self.ast.kinds, kind)
    }

    fn push_child(&mut self, id: AstId, child: AstId) {
        id.get_mut(&mut self.ast.children).push(child);
    }

    fn cur(&self) -> TokenKind {
        *self.cursor.get(&self.tokens.kinds)
    }

    fn cur_loc(&self) -> Option<Location> {
        *self.cursor.get(&self.tokens.locations)
    }

    fn eat(&mut self) -> TokenKind {
        let cur = self.cur();
        self.cursor = self.cursor.next().unwrap();
        cur
    }

    fn matches<const N: usize>(&self, kinds: [TokenKind; N]) -> bool {
        for kind in kinds {
            if *self.cursor.get(&self.tokens.kinds) == kind {
                return true;
            }
        }
        false
    }

    fn try_eat<const N: usize>(&mut self, kinds: [TokenKind; N]) -> TokenKind {
        for kind in kinds {
            if *self.cursor.get(&self.tokens.kinds) == kind {
                return self.eat();
            }
        }
        let location = self.cur_loc();
        let err = self.errors.log(
            ErrorKind::Parse,
            format!("Expected one of the following tokens: {:?}", kinds),
        );
        if let Some(location) = location {
            err.location(location);
        }
        self.cur()
    }

    fn eat_many<const N: usize>(&mut self, kinds: [TokenKind; N]) -> usize {
        let mut count = 0;
        'outer: loop {
            for kind in kinds {
                if *self.cursor.get(&self.tokens.kinds) == kind {
                    self.eat();
                    count += 1;
                    continue 'outer;
                }
            }
            break;
        }
        count
    }

    fn parse_list(&mut self, kind: AstKind, parser: impl Fn(&mut Self) -> AstId) -> AstId {
        let id = self.node(kind);
        let mut first = true;
        while !self.matches([RParen, RSquare, RCurl]) {
            if !first {
                self.try_eat([Comma, Newline, Semicolon]);
            }
            self.eat_many([Newline]);
            if !first || self.matches([RParen, RSquare, RCurl]) {
                break;
            }
            let child = parser(self);
            self.push_child(id, child);
            first = false;
        }
        id
    }

    fn parse_field(&mut self) -> AstId {
        self.errors.log(ErrorKind::Parse, "unimplemented");
        self.node(AstKind::Error)
    }

    fn parse_ident(&mut self) -> AstId {
        let id = if self.matches([Underscore]) {
            self.node(AstKind::Void)
        } else if self.matches([LIdent]) {
            self.node(AstKind::LIdent)
        } else if self.matches([UIdent]) {
            self.node(AstKind::UIdent)
        } else {
            self.errors.log(ErrorKind::Parse, "Expected identifier");
            self.node(AstKind::Error)
        };
        self.eat();
        id
    }
}
