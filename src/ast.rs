use std::collections::HashMap;

use colored::Colorize;
use smallvec::SmallVec;

use crate::{
    arena::{Arena, Id, World},
    error::{ErrorKind, Errors},
    file::Files,
    lexer::{Location, TokenKind},
    prelude::Prelude,
    types::Type,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum AstKind {
    LIdent,
    UIdent,
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
    Field,
    Arg,
    Optional,
    Bind,
    BindMut,
    Assign,
    If,
    IfElse,
    Infix(InfixKind),
    PrimitiveType(Type),

    #[default]
    Error,
}

impl AstKind {
    pub fn has_atomic_data(self) -> bool {
        matches!(
            self,
            Self::LIdent | Self::UIdent | Self::String | Self::Char | Self::Integer | Self::Float
        )
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    Integer(i64),
    String(String),
    Char(u8),
}

#[derive(Default, Debug)]
pub struct Ast {
    pub ids: World<Ast>,
    pub kinds: Arena<Ast, AstKind>,
    pub children: Arena<Ast, SmallVec<[AstId; 4]>>,
    pub locations: Arena<Ast, Option<Location>>,
    pub qualified_idents: Arena<Ast, String>,
    pub resolved_idents: Arena<Ast, Option<AstId>>,
    pub literals: Arena<Ast, Option<Literal>>,
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
        resolved_idents: &mut Arena<Ast, Option<AstId>>,
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
            ident.put(resolved_idents, Some(id.get(children)[1]));
            Self::qualify_helper(
                id.get(children)[1],
                &path,
                files,
                kinds,
                children,
                locations,
                qualified_idents,
                qualified_idents_map,
                resolved_idents,
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
                    resolved_idents,
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
                resolved_idents,
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
            lhs.put(resolved_idents, Some(id.get(children)[1]));
            Self::qualify_helper(
                id.get(children)[1],
                &path,
                files,
                kinds,
                children,
                locations,
                qualified_idents,
                qualified_idents_map,
                resolved_idents,
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
                    resolved_idents,
                    unresolved,
                );
            }
        }
    }

    pub fn resolve_helper(
        kinds: &Arena<Ast, AstKind>,
        qualified_idents: &mut Arena<Ast, String>,
        qualified_idents_map: &mut HashMap<String, AstId>,
        resolved_idents: &mut Arena<Ast, Option<AstId>>,
        unresolved: &Vec<AstId>,
    ) {
        for id in unresolved {
            // Try to resolve identifiers to their fully qualified names
            if matches!(id.get(kinds), AstKind::LIdent | AstKind::UIdent) {
                let mut ident = id.get(qualified_idents).to_owned();
                loop {
                    // If we find an exact match, go with that
                    if let Some(rid) = qualified_idents_map.get(ident.as_str()) {
                        id.put(qualified_idents, ident);
                        id.put(resolved_idents, Some(*rid));
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

    pub fn qualify_and_resolve(&mut self, files: &Files, root: AstId, prelude: impl Prelude) {
        let mut qualified_idents = Arena::new();
        let mut qualified_idents_map = HashMap::new();
        let mut resolved_idents = Arena::new();
        let mut unresolved = Vec::new();
        prelude.apply(&mut self.ids, &mut self.kinds, &mut qualified_idents_map);
        Self::qualify_helper(
            root,
            "",
            files,
            &self.kinds,
            &self.children,
            &self.locations,
            &mut qualified_idents,
            &mut qualified_idents_map,
            &mut resolved_idents,
            &mut unresolved,
        );
        self.qualified_idents = qualified_idents;
        Self::resolve_helper(
            &self.kinds,
            &mut self.qualified_idents,
            &mut qualified_idents_map,
            &mut resolved_idents,
            &unresolved,
        );
        self.resolved_idents = resolved_idents;
    }

    pub fn simplify(&mut self, root: AstId) {
        for i in 0..root.get(&self.children).len() {
            let child = root.get(&self.children)[i];
            self.simplify(child);
            if *child.get(&self.kinds) == AstKind::Group {
                root.get_mut(&mut self.children)[i] = child.get(&self.children)[0];
            }
        }
    }

    pub fn parse_literals(&mut self, files: &Files, errors: &mut Errors) {
        for id in self.ids.iter() {
            if let Some(location) = id.get(&self.locations) {
                let slice = &location.file.get(&files.sources)
                    [location.start as usize..location.end as usize]
                    .trim();
                match id.get(&self.kinds) {
                    AstKind::Integer => match slice.parse::<i64>() {
                        Ok(v) => {
                            id.put(&mut self.literals, Some(Literal::Integer(v)));
                        }
                        Err(e) => {
                            errors.log(ErrorKind::Parse, format!("failed to parse integer: {e}"));
                        }
                    },
                    _ => {}
                }
            }
        }
    }

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
        let mut len = indent * 2 + kind_str.len();
        print!("{}{} ", "  ".repeat(indent), kind_str.bold().magenta());
        let location = id.get(&self.locations);
        let qualified = id.get(&self.qualified_idents);
        if !qualified.is_empty() {
            len += qualified.len() + 1;
            print!("{} ", qualified.green());
        } else if let Some(location) = location
            && id.get(&self.kinds).has_atomic_data()
        {
            let slice = &location.file.get(&files.sources)
                [location.start as usize..location.end as usize]
                .trim();
            len += slice.len() + 1;
            print!("{} ", slice);
        }
        print!("{} ", " ".repeat(36_usize.saturating_sub(len)));
        Location::pretty_print_opt(location, files);
        for child in id.get(&self.children) {
            self.pretty_print_indented(*child, indent + 1, files);
        }
    }

    pub fn pretty_print(&mut self, id: AstId, files: &Files) {
        println!();
        self.compute_locations(id);
        self.pretty_print_indented(id, 0, files);
    }
}
