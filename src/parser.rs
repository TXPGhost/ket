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
    Call,
    Group,
    Func,
    Block,
    Proj,
    Struct,
    Tuple,
    Array,
    Field,
    Bind,
    BindMut,
    Assign,
    If,
    Infix(InfixKind),

    #[default]
    Error,
}

impl AstKind {
    pub fn has_string_data(self) -> bool {
        matches!(
            self,
            Self::LIdent
                | Self::UIdent
                | Self::String
                | Self::Character
                | Self::Integer
                | Self::Float
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InfixKind {
    Add,
    Sub,
    Mul,
    Div,
}

impl InfixKind {
    pub fn ast(self) -> AstKind {
        AstKind::Infix(self)
    }

    pub fn tok(self) -> TokenKind {
        match self {
            InfixKind::Add => Plus,
            InfixKind::Sub => Minus,
            InfixKind::Mul => Times,
            InfixKind::Div => Divide,
        }
    }
}

pub struct PrattGroup<'a> {
    kinds: &'a [InfixKind],
    assoc: Assoc,
}

impl<'a> PrattGroup<'a> {
    pub fn left(kinds: &'a [InfixKind]) -> Self {
        Self {
            kinds,
            assoc: Assoc::Left,
        }
    }

    pub fn right(kinds: &'a [InfixKind]) -> Self {
        Self {
            kinds,
            assoc: Assoc::Right,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Assoc {
    Left,
    Right,
}

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
            && id.get(&self.kinds).has_string_data()
        {
            let slice = &location.file.get(&files.sources)
                [location.start as usize..location.end as usize]
                .trim();
            len += slice.len() + 1;
            print!("{} ", slice.bold());
        }
        print!("{} ", " ".repeat(36_usize.saturating_sub(len)));
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
    context: Context,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Context {
    Struct,
    Block,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a Tokens, ast: &'a mut Ast, errors: &'a mut Errors) -> Self {
        Self {
            tokens,
            cursor: TokenId::new(0),
            ast,
            errors,
            context: Context::Struct,
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

    fn matches(&self, kinds: &[TokenKind]) -> bool {
        for kind in kinds {
            if self.cursor.get(&self.tokens.kinds) == kind {
                return true;
            }
        }
        false
    }

    fn matches_ahead(&mut self, kinds: &[TokenKind], lookahead: usize) -> bool {
        for _ in 0..lookahead {
            self.cursor = self.cursor.next().unwrap();
        }
        let result = self.matches(kinds);
        for _ in 0..lookahead {
            self.cursor = self.cursor.prev().unwrap();
        }
        result
    }

    fn try_eat(&mut self, kinds: &[TokenKind]) -> Option<TokenKind> {
        assert!(!kinds.is_empty());
        for kind in kinds {
            if self.cursor.get(&self.tokens.kinds) == kind {
                return Some(self.eat());
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
        None
    }

    fn eat_many(&mut self, kinds: &[TokenKind]) -> usize {
        let mut count = 0;
        'outer: loop {
            for kind in kinds {
                if self.cursor.get(&self.tokens.kinds) == kind {
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
        while !self.eof() && !self.matches(&[RParen, RSquare, RCurl]) {
            if !first {
                self.merge_loc(id, self.cur_loc());
                self.try_eat(&[Comma, Newline, Semicolon]);
            }
            self.eat_many(&[Newline]);
            if self.eof() || self.matches(&[RParen, RSquare, RCurl]) {
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

    fn parse_stmt(&mut self) -> AstId {
        if self.matches_ahead(&[Equals, ColonEquals, DotEquals], 1) {
            let ident = self.parse_ident();
            let id = match self.try_eat(&[Equals, ColonEquals, DotEquals]) {
                Some(Equals) => self.node(AstKind::Bind),
                Some(ColonEquals) => self.node(AstKind::BindMut),
                Some(DotEquals) => self.node(AstKind::Assign),
                _ => unreachable!(),
            };
            let expr = self.parse_expr();
            self.push_child(id, ident);
            self.push_child(id, expr);
            return id;
        }
        self.parse_expr()
    }

    fn parse_expr(&mut self) -> AstId {
        self.parse_pratt(&[
            PrattGroup::left(&[InfixKind::Add, InfixKind::Sub]),
            PrattGroup::left(&[InfixKind::Mul, InfixKind::Div]),
        ])
    }

    fn parse_base_expr(&mut self) -> AstId {
        let base = match (self.cur(), self.context) {
            (QuestionMark, _) => self.parse_if(),
            (LParen, Context::Struct) => self.parse_struct(),
            (LParen, Context::Block) => self.parse_tuple(),
            (LCurl, _) => self.parse_block(),
            _ => self.parse_atom(),
        };
        if self.matches(&[LParen]) {
            let id = self.node(AstKind::Call);
            let args = self.parse_tuple();
            self.push_child(id, base);
            self.push_child(id, args);
            return id;
        } else if *base.get(&self.ast.kinds) == AstKind::Struct
            && !self.matches(&[Newline, Comma, RParen, RCurl, RSquare])
        {
            let id = self.node(AstKind::Func);
            let body = self.parse_expr();
            self.push_child(id, base);
            self.push_child(id, body);
            return id;
        }
        base
    }

    fn parse_pratt(&mut self, groups: &[PrattGroup]) -> AstId {
        match groups {
            [] => self.parse_base_expr(),
            _ => {
                let group = &groups[0];
                match group.assoc {
                    Assoc::Left => self.parse_left_recursive_expr(group.kinds, &groups[1..]),
                    Assoc::Right => self.parse_right_recursive_expr(group.kinds, &groups[1..]),
                }
            }
        }
    }

    fn parse_left_recursive_expr(
        &mut self,
        kinds: &[InfixKind],
        next_groups: &[PrattGroup],
    ) -> AstId {
        let mut id = self.parse_pratt(next_groups);
        loop {
            let mut matched = false;
            for kind in kinds {
                if self.matches(&[kind.tok()]) {
                    self.eat();
                    let infix = self.node(kind.ast());
                    let next = self.parse_pratt(next_groups);
                    self.push_child(infix, id);
                    self.push_child(infix, next);
                    id = infix;
                    matched = true;
                    break;
                }
            }
            if !matched {
                break;
            }
        }
        id
    }

    fn parse_right_recursive_expr(
        &mut self,
        kinds: &[InfixKind],
        next_groups: &[PrattGroup],
    ) -> AstId {
        let id = self.parse_pratt(next_groups);
        for kind in kinds {
            if self.matches(&[kind.tok()]) {
                self.eat();
                let infix = self.node(kind.ast());
                let next = self.parse_right_recursive_expr(kinds, next_groups);
                self.push_child(infix, id);
                self.push_child(infix, next);
                return infix;
            }
        }
        id
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

    fn parse_delimited_list(
        &mut self,
        kind: AstKind,
        left: TokenKind,
        right: TokenKind,
        parser: fn(&mut Self) -> AstId,
    ) -> AstId {
        let left_loc = self.cur_loc();
        self.try_eat(&[left]);
        let id = self.parse_list(kind, parser);
        self.merge_loc(id, left_loc);
        self.merge_loc(id, self.cur_loc());
        self.try_eat(&[right]);
        id
    }

    fn parse_struct(&mut self) -> AstId {
        self.parse_delimited_list(AstKind::Struct, LParen, RParen, Self::parse_field)
    }

    fn parse_block(&mut self) -> AstId {
        let old_context = self.context;
        self.context = Context::Block;
        let id = self.parse_delimited_list(AstKind::Block, LCurl, RCurl, Self::parse_stmt);
        self.context = old_context;
        id
    }

    fn parse_tuple(&mut self) -> AstId {
        self.parse_delimited_list(AstKind::Tuple, LParen, RParen, Self::parse_expr)
    }

    fn parse_if(&mut self) -> AstId {
        let id = self.node(AstKind::If);

        self.merge_loc(id, self.cur_loc());
        self.try_eat(&[QuestionMark]);

        self.merge_loc(id, self.cur_loc());
        self.try_eat(&[LParen]);

        let cond = self.parse_expr();

        self.merge_loc(id, self.cur_loc());
        self.try_eat(&[RParen]);

        let if_body = self.parse_expr();

        self.push_child(id, cond);
        self.push_child(id, if_body);

        if self.matches(&[Colon]) {
            self.merge_loc(id, self.cur_loc());
            self.eat();
            let else_body = self.parse_expr();
            self.push_child(id, else_body);
        }

        id
    }
}
