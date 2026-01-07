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
    Group,
    Func,
    Block,
    Proj,
    Struct,
    Constructor,
    Tuple,
    Array,
    Args,
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

    fn error_at(&mut self, location: Option<Location>, message: impl Into<std::string::String>) {
        let err = self.errors.log(ErrorKind::Parse, message.into());
        if let Some(location) = location {
            err.location(location);
        }
    }

    fn error(&mut self, message: impl Into<std::string::String>) {
        self.error_at(self.cur_loc(), message);
    }

    fn node(&mut self, kind: AstKind) -> AstId {
        self.ast.ids.alloc().put(&mut self.ast.kinds, kind)
    }

    fn push_child(&mut self, id: AstId, child: AstId) {
        id.get_mut(&mut self.ast.children).push(child);
    }

    fn num_children(&mut self, id: AstId) -> usize {
        id.get(&self.ast.children).len()
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

    fn parse_field_or_expr(&mut self) -> AstId {
        if !self.matches([LIdent, UIdent, Underscore]) {
            return self.parse_expr();
        }
        self.parse_field()
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
        let base = match self.cur() {
            LParen => self.parse_tuple_or_group(),
            LCurl => self.parse_struct(),
            _ => self.parse_atom(),
        };
        if self.matches([LParen]) {
            let id = self.node(AstKind::Call);
            let args = self.parse_args();
            self.push_child(id, base);
            self.push_child(id, args);
            return id;
        } else if *base.get(&self.ast.kinds) == AstKind::Tuple
            && !self.matches([Newline, Comma, RParen, RCurl, RSquare])
        {
            let id = self.node(AstKind::Func);
            let body = if self.matches([LCurl]) {
                self.parse_block()
            } else {
                self.parse_expr()
            };
            self.push_child(id, base);
            self.push_child(id, body);
            return id;
        } else if self.matches([LCurl]) {
            if *base.get(&self.ast.kinds) == AstKind::Tuple {
                let id = self.node(AstKind::Func);
                let body = if self.matches([LCurl]) {
                    self.parse_block()
                } else {
                    self.parse_expr()
                };
                self.push_child(id, base);
                self.push_child(id, body);
                return id;
            }
            let id = self.node(AstKind::Constructor);
            let body = self.parse_struct();
            self.push_child(id, base);
            self.push_child(id, body);
            return id;
        }
        base
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

    fn parse_block(&mut self) -> AstId {
        let lcurl_loc = self.cur_loc();
        self.try_eat([LCurl]);
        let id = self.parse_list(AstKind::Block, Self::parse_expr);
        self.merge_loc(id, lcurl_loc);
        self.merge_loc(id, self.cur_loc());
        self.try_eat([RCurl]);
        id
    }

    fn parse_args(&mut self) -> AstId {
        let lparen_loc = self.cur_loc();
        self.try_eat([LParen]);
        let id = self.parse_list(AstKind::Args, Self::parse_expr);
        self.merge_loc(id, lparen_loc);
        self.merge_loc(id, self.cur_loc());
        self.try_eat([RParen]);
        id
    }

    fn parse_tuple_or_group(&mut self) -> AstId {
        let lparen_loc = self.cur_loc();
        self.try_eat([LParen]);
        let id = self.parse_list(AstKind::Tuple, Self::parse_field_or_expr);
        self.merge_loc(id, lparen_loc);
        self.merge_loc(id, self.cur_loc());
        self.try_eat([RParen]);

        if self.num_children(id) == 1 {
            let field = id.get(&self.ast.children)[0];
            if *field.get(&self.ast.kinds) == AstKind::Group {
                *id.get_mut(&mut self.ast.kinds) = AstKind::Group;
            } else if *field.get(&self.ast.kinds) == AstKind::Field {
                let expr = field.get(&self.ast.children)[1];
                match expr.get(&self.ast.kinds) {
                    AstKind::Tuple => {
                        *field.get_mut(&mut self.ast.kinds) = AstKind::Call;
                        *id.get_mut(&mut self.ast.kinds) = AstKind::Group;
                    }
                    AstKind::Struct => {
                        *field.get_mut(&mut self.ast.kinds) = AstKind::Constructor;
                        *id.get_mut(&mut self.ast.kinds) = AstKind::Group;
                    }
                    _ => {}
                }
            }
        } else {
            for child in id.get(&self.ast.children).clone() {
                if *child.get(&self.ast.kinds) != AstKind::Field {
                    self.error_at(
                        *child.get(&self.ast.locations),
                        "Expected name for tuple field",
                    );
                }
            }
        }

        id
    }
}
