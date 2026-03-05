use crate::{
    ast::{Ast, AstId, AstKind, InfixKind},
    error::{ErrorKind, Errors},
    lexer::{
        Location, TokenId,
        TokenKind::{self, *},
        Tokens,
    },
};

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

impl TokenId {}

pub struct Parser<'a> {
    tokens: &'a Tokens,
    cursor: TokenId,
    ast: &'a mut Ast,
    errors: &'a mut Errors,
    ctx: ParserCtx,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParserCtx {
    Type,
    Value,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a Tokens, ast: &'a mut Ast, errors: &'a mut Errors) -> Self {
        Self {
            tokens,
            cursor: TokenId::new(0),
            ast,
            errors,
            ctx: ParserCtx::Type,
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

    fn parse_proj_name(&mut self) -> AstId {
        let id = match self.cur() {
            Underscore => self.node(AstKind::Void),
            LIdent => self.node(AstKind::LIdent),
            UIdent => self.node(AstKind::UIdent),
            Integer => self.node(AstKind::Integer),
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

    fn parse_arg(&mut self) -> AstId {
        if self.matches_ahead(&[TokenKind::Equals], 1) {
            let id = self.node(AstKind::Arg);
            let ident = self.parse_ident();
            self.eat();
            let expr = self.parse_expr();
            self.push_child(id, ident);
            self.push_child(id, expr);
            return id;
        }
        self.parse_expr()
    }

    fn parse_stmt(&mut self) -> AstId {
        let lhs = self.parse_expr();
        if self.matches(&[Equals, ColonEquals, DotEquals]) {
            let id = match self.try_eat(&[Equals, ColonEquals, DotEquals]) {
                Some(Equals) => self.node(AstKind::Bind),
                Some(ColonEquals) => self.node(AstKind::BindMut),
                Some(DotEquals) => self.node(AstKind::Assign),
                _ => unreachable!(),
            };
            let rhs = self.parse_expr();
            self.push_child(id, lhs);
            self.push_child(id, rhs);
            return id;
        }
        lhs
    }

    fn parse_expr(&mut self) -> AstId {
        self.parse_pratt(&[
            PrattGroup::left(&[InfixKind::Add, InfixKind::Sub]),
            PrattGroup::left(&[InfixKind::Mul, InfixKind::Div]),
            PrattGroup::left(&[InfixKind::Gt, InfixKind::Lt, InfixKind::Ge, InfixKind::Le]),
            PrattGroup::left(&[InfixKind::Eq, InfixKind::Ne]),
        ])
    }

    fn parse_base_expr(&mut self) -> AstId {
        let mut base = match (self.cur(), self.ctx) {
            (QuestionMark, _) => self.parse_if(),
            (LParen, ParserCtx::Type) => self.parse_struct(),
            (LParen, ParserCtx::Value) => self.parse_tuple(),
            (LSquare, _) => self.parse_array(),
            (LCurl, _) => self.parse_block(),
            _ => self.parse_atom(),
        };
        loop {
            if self.matches(&[LParen]) {
                let id = self.node(AstKind::Call);
                let args = self.parse_tuple();
                self.push_child(id, base);
                self.push_child(id, args);
                base = id;
                continue;
            }
            if self.matches(&[LSquare]) {
                let id = self.node(AstKind::Index);
                let args = self.parse_index();
                self.push_child(id, base);
                self.push_child(id, args);
                base = id;
                continue;
            }
            if self.matches(&[Dot]) {
                let id = self.node(AstKind::Proj);
                self.eat();
                let ident = self.parse_proj_name();
                self.push_child(id, base);
                self.push_child(id, ident);
                base = id;
                continue;
            }
            if self.matches(&[Colon])
                && self.matches_ahead(&[LIdent, UIdent], 1)
                && self.matches_ahead(&[LParen], 2)
            {
                let id = self.node(AstKind::Method);
                self.eat();
                let ident = self.parse_ident();
                let args = self.parse_tuple();
                self.push_child(id, base);
                self.push_child(id, ident);
                self.push_child(id, args);
                base = id;
                continue;
            }
            if self.matches(&[QuestionMark]) {
                let id = self.node(AstKind::Optional);
                self.push_child(id, base);
                self.eat();
                if !self.matches(&[
                    Newline,
                    Comma,
                    RParen,
                    RCurl,
                    RSquare,
                    EndOfFile,
                    QuestionMark,
                ]) {
                    let err = self.parse_expr();
                    self.push_child(id, err);
                }
                base = id;
                continue;
            }
            if *base.get(&self.ast.kinds) == AstKind::Struct
                && !self.matches(&[Newline, Comma, RParen, RCurl, RSquare])
            {
                base.put(&mut self.ast.kinds, AstKind::Tuple);
                let id = self.node(AstKind::Func);
                let body = self.parse_expr();
                self.push_child(id, base);
                self.push_child(id, body);
                base = id;
                continue;
            }
            break;
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
            Char => self.node(AstKind::Char),
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
        let old_context = self.ctx;
        self.ctx = ParserCtx::Value;
        let id = self.parse_delimited_list(AstKind::Block, LCurl, RCurl, Self::parse_stmt);
        self.ctx = old_context;
        id
    }

    fn parse_tuple(&mut self) -> AstId {
        let id = self.parse_delimited_list(AstKind::Tuple, LParen, RParen, Self::parse_arg);
        if self.num_children(id) == 1 {
            *id.get_mut(&mut self.ast.kinds) = AstKind::Group;
        }
        id
    }

    fn parse_array(&mut self) -> AstId {
        let id = self.parse_delimited_list(AstKind::Array, LSquare, RSquare, Self::parse_expr);
        if self.matches(&[
            LIdent, UIdent, Underscore, String, Char, Integer, Float, LParen,
        ]) {
            let expr = self.parse_expr();
            if self.num_children(id) == 0 {
                *id.get_mut(&mut self.ast.kinds) = AstKind::Vector;
                self.push_child(id, expr);
            } else {
                if self.num_children(id) > 1 {
                    self.error("Repeat expression may not have multiple lengths");
                    id.get_mut(&mut self.ast.children).truncate(1);
                }
                *id.get_mut(&mut self.ast.kinds) = AstKind::Repeat;
                self.push_child(id, expr);
            }
        }
        id
    }

    fn parse_index(&mut self) -> AstId {
        let left_lsquare = self.cur_loc();
        self.try_eat(&[LSquare]);
        let id = self.parse_expr();
        self.merge_loc(id, left_lsquare);
        self.merge_loc(id, self.cur_loc());
        self.try_eat(&[RSquare]);
        id
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
            id.put(&mut self.ast.kinds, AstKind::IfElse);
            self.merge_loc(id, self.cur_loc());
            self.eat();
            let else_body = self.parse_expr();
            self.push_child(id, else_body);
        }

        id
    }
}
