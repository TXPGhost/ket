use colored::Colorize;
use smallvec::SmallVec;

use crate::{
    arena::{Arena, Id, World},
    error::{ErrorKind, Errors},
    file::Files,
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
    Character,
    None,
    Integer,
    Float,
    Infix,
    Call,
    Func,
    Block,
    Proj,
    Struct,
    Field,

    #[default]
    Error,
}

pub enum InfixKind {}

#[derive(Default, Debug)]
pub struct Ast {
    pub ids: World<Ast>,
    pub kinds: Arena<Ast, AstKind>,
    pub children: Arena<Ast, SmallVec<[AstId; 4]>>,
    pub locations: Arena<Ast, Option<Location>>,
}
pub type AstId = Id<Ast>;

impl Ast {
    fn compute_locations(&mut self, root: AstId) {
        fn helper(
            id: AstId,
            locations: &mut Arena<Ast, Option<Location>>,
            children: &Arena<Ast, SmallVec<[AstId; 4]>>,
        ) {
            let mut location = *id.get(locations);
            for child in id.get(children) {
                helper(*child, locations, children);
                location = Location::merge(&location, child.get(locations));
            }
            id.put(locations, location);
        }
        helper(root, &mut self.locations, &self.children)
    }

    fn pretty_print_indented(&self, id: AstId, indent: usize, files: &Files) {
        let kind_str = format!("{:?}", id.get(&self.kinds));
        let mut len = indent * 2 + kind_str.len();
        print!("{}{} ", "  ".repeat(indent), kind_str.bold());
        let location = id.get(&self.locations);
        if let Some(location) = location
            && id.get(&self.children).is_empty()
        {
            let slice = &location.file.get(&files.sources)
                [location.start as usize..location.end as usize]
                .trim();
            len += slice.len();
            print!("{} ", slice.bold());
        }
        print!("{} ", " ".repeat(32_usize.saturating_sub(len)));
        Location::pretty_print_opt(location, files);
        for child in id.get(&self.children) {
            self.pretty_print_indented(*child, indent + 1, files);
        }
    }

    pub fn pretty_print(&mut self, id: AstId, files: &Files) {
        self.compute_locations(id);
        self.pretty_print_indented(id, 0, files);
    }
}

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
        if !self.eof() {
            self.error("Trailing tokens when parsing file");
        }
        ast
    }

    fn error(&mut self, message: impl Into<std::string::String>) {
        let location = self.cur_loc();
        let err = self.errors.log(ErrorKind::Parse, message.into());
        if let Some(location) = location {
            err.location(location);
        }
    }

    fn node(&mut self, kind: AstKind) -> AstId {
        self.ast.ids.alloc().put(&mut self.ast.kinds, kind)
    }

    fn push_child(&mut self, id: AstId, child: AstId) {
        id.get_mut(&mut self.ast.children).push(child);
    }

    fn merge_loc(&mut self, id: AstId, cur_loc: Option<Location>) {
        let loc = id.get_mut(&mut self.ast.locations);
        *loc = Location::merge(loc, &cur_loc);
    }

    fn cur(&self) -> TokenKind {
        *self.cursor.get(&self.tokens.kinds)
    }

    fn eof(&self) -> bool {
        self.cur() == EndOfFile
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
        assert!(!kinds.is_empty());
        for kind in kinds {
            if *self.cursor.get(&self.tokens.kinds) == kind {
                return self.eat();
            }
        }
        if kinds.len() == 1 {
            self.error(format!(
                "Expected {:?} but found {:?}",
                kinds[0],
                self.cur()
            ));
        } else {
            self.error(format!(
                "Expected {}, or {:?}, but found {:?}",
                &kinds[..kinds.len() - 1]
                    .iter()
                    .map(|kind| format!("{:?}", kind))
                    .collect::<Vec<std::string::String>>()
                    .join(", "),
                kinds[kinds.len() - 1],
                self.cur()
            ));
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
        while !self.eof() && !self.matches([RParen, RSquare, RCurl]) {
            if !first {
                self.merge_loc(id, self.cur_loc());
                self.try_eat([Comma, Newline, Semicolon]);
            }
            self.eat_many([Newline]);
            if self.eof() || self.matches([RParen, RSquare, RCurl]) {
                break;
            }
            let child = parser(self);
            self.push_child(id, child);
            first = false;
        }
        id
    }

    fn parse_field(&mut self) -> AstId {
        let id = self.node(AstKind::Field);
        let ident = self.parse_ident();
        let expr = self.parse_expr();
        self.push_child(id, ident);
        self.push_child(id, expr);
        id
    }

    fn parse_ident(&mut self) -> AstId {
        let id = match self.cur() {
            Underscore => self.node(AstKind::Void),
            LIdent => self.node(AstKind::LIdent),
            UIdent => self.node(AstKind::UIdent),
            _ => {
                self.error("Expected identifier");
                self.node(AstKind::Error)
            }
        };
        let location = self.cur_loc();
        id.put(&mut self.ast.locations, location);
        self.eat();
        id
    }

    fn parse_expr(&mut self) -> AstId {
        match self.cur() {
            LCurl => self.parse_struct(),
            _ => self.parse_atom(),
        }
    }

    fn parse_atom(&mut self) -> AstId {
        let id = match self.cur() {
            Integer => self.node(AstKind::Integer),
            Float => self.node(AstKind::Float),
            String => self.node(AstKind::String),
            Character => self.node(AstKind::Character),
            Underscore => self.node(AstKind::None),
            LIdent => self.node(AstKind::LIdent),
            UIdent => self.node(AstKind::UIdent),
            _ => {
                self.error("Expected expression");
                self.node(AstKind::Error)
            }
        };
        let location = self.cur_loc();
        id.put(&mut self.ast.locations, location);
        self.eat();
        id
    }

    fn parse_struct(&mut self) -> AstId {
        let lcurl_loc = self.cur_loc();
        self.try_eat([LCurl]);
        let id = self.parse_list(AstKind::Struct, Self::parse_field);
        self.merge_loc(id, lcurl_loc);
        self.merge_loc(id, self.cur_loc());
        self.try_eat([RCurl]);
        id
    }
}
