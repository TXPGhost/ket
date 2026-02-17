use std::collections::HashMap;

use colored::Colorize;
use smallvec::SmallVec;

use crate::{
    arena::{Arena, Id, World},
    file::Files,
    lexer::{Location, TokenKind},
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
    Method,
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
    Arg,
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
    pub fn has_atomic_data(self) -> bool {
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
            InfixKind::Add => TokenKind::Plus,
            InfixKind::Sub => TokenKind::Minus,
            InfixKind::Mul => TokenKind::Times,
            InfixKind::Div => TokenKind::Divide,
        }
    }
}

#[derive(Default, Debug)]
pub struct Ast {
    pub ids: World<Ast>,
    pub kinds: Arena<Ast, AstKind>,
    pub children: Arena<Ast, SmallVec<[AstId; 4]>>,
    pub locations: Arena<Ast, Option<Location>>,
    pub qualified_idents: Arena<Ast, String>,
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

    #[allow(clippy::too_many_arguments)]
    pub fn qualify_helper(
        id: AstId,
        path: &str,
        files: &Files,
        kinds: &Arena<Ast, AstKind>,
        children: &Arena<Ast, SmallVec<[AstId; 4]>>,
        locations: &Arena<Ast, Option<Location>>,
        qualified_idents: &mut Arena<Ast, String>,
        qualified_idents_map: &mut HashMap<String, AstId>,
        unresolved: &mut Vec<AstId>,
    ) {
        if matches!(id.get(kinds), AstKind::Field) {
            let ident = id.get(children)[0];
            let location = ident
                .get(locations)
                .expect("must have file source to compute qualified idents");
            let slice =
                &location.file.get(&files.sources)[location.start as usize..location.end as usize];
            let path = format!("{path}.{slice}");
            ident.put(qualified_idents, path.clone());
            qualified_idents_map.insert(path.clone(), ident);
            Self::qualify_helper(
                id.get(children)[1],
                &path,
                files,
                kinds,
                children,
                locations,
                qualified_idents,
                qualified_idents_map,
                unresolved,
            );
        } else if matches!(id.get(kinds), AstKind::Block) {
            let idx = id.index();
            let path = format!("{path}.${idx}");
            id.put(qualified_idents, path.clone());
            for child in id.get(children) {
                Self::qualify_helper(
                    *child,
                    &path,
                    files,
                    kinds,
                    children,
                    locations,
                    qualified_idents,
                    qualified_idents_map,
                    unresolved,
                );
            }
        } else if matches!(id.get(kinds), AstKind::Arg) {
            Self::qualify_helper(
                id.get(children)[1],
                path,
                files,
                kinds,
                children,
                locations,
                qualified_idents,
                qualified_idents_map,
                unresolved,
            );
        } else if matches!(id.get(kinds), AstKind::Bind) {
            let lhs = id.get(children)[0];
            let location = lhs
                .get(locations)
                .expect("must have file source to compute qualified idents");
            let slice =
                &location.file.get(&files.sources)[location.start as usize..location.end as usize];
            let path = format!("{path}.{slice}");
            lhs.put(qualified_idents, path.clone());
            qualified_idents_map.insert(path.clone(), lhs);
            Self::qualify_helper(
                id.get(children)[1],
                &path,
                files,
                kinds,
                children,
                locations,
                qualified_idents,
                qualified_idents_map,
                unresolved,
            );
        } else if matches!(id.get(kinds), AstKind::LIdent | AstKind::UIdent) {
            let location = id
                .get(locations)
                .expect("must have file source to compute qualified idents");
            let slice =
                &location.file.get(&files.sources)[location.start as usize..location.end as usize];
            let path = format!("{path}.{slice}");
            id.put(qualified_idents, path.clone());
            unresolved.push(id);
        } else {
            for child in id.get(children) {
                Self::qualify_helper(
                    *child,
                    path,
                    files,
                    kinds,
                    children,
                    locations,
                    qualified_idents,
                    qualified_idents_map,
                    unresolved,
                );
            }
        }
    }

    pub fn resolve_helper(
        kinds: &Arena<Ast, AstKind>,
        qualified_idents: &mut Arena<Ast, String>,
        qualified_idents_map: &mut HashMap<String, AstId>,
        unresolved: &Vec<AstId>,
    ) {
        for id in unresolved {
            // Try to resolve identifiers to their fully qualified names
            if matches!(id.get(kinds), AstKind::LIdent | AstKind::UIdent) {
                let mut ident = id.get(qualified_idents).to_owned();
                loop {
                    // If we find an exact match, go with that
                    if qualified_idents_map.get(ident.as_str()).is_some() {
                        id.put(qualified_idents, ident);
                        break;
                    }

                    // Go up a scope, e.g. ".Vector3.values.x" --> ".Vector3.x"
                    let Some(rdot) = ident.rfind('.') else {
                        id.put(qualified_idents, "".to_owned());
                        break;
                    };
                    let Some(rrdot) = ident[..rdot].rfind('.') else {
                        id.put(qualified_idents, "".to_owned());
                        break;
                    };
                    ident = format!("{}{}", &ident[..rrdot], &ident[rdot..]);
                }
            }
        }
    }

    pub fn compute_qualified_idents(&mut self, files: &Files, root: AstId) {
        let mut qualified_idents = Arena::new();
        let mut qualified_idents_map = HashMap::new();
        let mut unresolved = Vec::new();
        Self::qualify_helper(
            root,
            "",
            files,
            &self.kinds,
            &self.children,
            &self.locations,
            &mut qualified_idents,
            &mut qualified_idents_map,
            &mut unresolved,
        );
        self.qualified_idents = qualified_idents;
        Self::resolve_helper(
            &self.kinds,
            &mut self.qualified_idents,
            &mut qualified_idents_map,
            &unresolved,
        );
    }

    fn pretty_print_indented(&self, id: AstId, indent: usize, files: &Files) {
        let kind_str = format!("{:?}", id.get(&self.kinds));
        let mut len = indent * 2 + kind_str.len();
        print!("{}{} ", "  ".repeat(indent), kind_str.bold());
        let location = id.get(&self.locations);
        let qualified = id.get(&self.qualified_idents);
        if !qualified.is_empty() {
            len += qualified.len() + 1;
            print!("{} ", qualified.bold());
        } else if let Some(location) = location
            && id.get(&self.kinds).has_atomic_data()
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
