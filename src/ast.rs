use colored::Colorize;
use smallvec::SmallVec;

use crate::{
    arena::{Arena, Id, World},
    file::Files,
    lexer::{
        Location,
        TokenKind::{self, *},
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
    Index,
    Struct,
    Tuple,
    Array,
    Vector,
    Field,
    Optional,
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
