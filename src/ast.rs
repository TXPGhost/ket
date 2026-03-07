use std::collections::HashMap;

use colored::Colorize;
use smallvec::SmallVec;

use crate::{
    arena::{Arena, Id, World},
    error::{ErrorKind, Errors},
    file::Files,
    lexer::{Location, TokenKind},
    prelude::Prelude,
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

    PrimitiveI32,
    PrimitiveF32,
    PrimitiveString,
    PrimitiveChar,

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

#[derive(Debug, Clone)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Char(u8),
}

#[derive(Default, Debug)]
pub struct Ast {
    pub ids: World<Ast>,
    pub kinds: Arena<Ast, AstKind>,
    pub children: Arena<Ast, SmallVec<[AstId; 4]>>,
    pub locations: Arena<Ast, Option<Location>>,
    pub idents: Arena<Ast, String>,
    pub qualified_idents: Arena<Ast, String>,
    pub definitions: Arena<Ast, Option<AstId>>,
    pub literals: Arena<Ast, Option<Literal>>,
}
pub type AstId = Id<Ast>;

impl Ast {
    pub fn compute_locations(&mut self, root: AstId) {
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
    pub fn qualify_idents(
        id: AstId,
        path: &str,
        files: &Files,
        kinds: &Arena<Ast, AstKind>,
        children: &Arena<Ast, SmallVec<[AstId; 4]>>,
        locations: &Arena<Ast, Option<Location>>,
        idents: &mut Arena<Ast, String>,
        qualified_idents: &mut Arena<Ast, String>,
        definitions_map: &mut HashMap<String, AstId>,
        definitions: &mut Arena<Ast, Option<AstId>>,
        unresolved: &mut Vec<AstId>,
    ) {
        let kind = id.get(kinds);
        match kind {
            AstKind::Field => {
                let ident = id.get(children)[0];
                let location = ident
                    .get(locations)
                    .expect("must have file source to compute qualified idents");
                let slice = &location.file.get(&files.sources)
                    [location.start as usize..location.end as usize];
                ident.put(idents, slice.to_owned());
                let path = format!("{path}.{slice}");
                ident.put(qualified_idents, path.clone());
                definitions_map.insert(path.clone(), id);
                ident.put(definitions, Some(id));
                id.put(definitions, Some(id));
                Self::qualify_idents(
                    id.get(children)[1],
                    &path,
                    files,
                    kinds,
                    children,
                    locations,
                    idents,
                    qualified_idents,
                    definitions_map,
                    definitions,
                    unresolved,
                );
            }
            AstKind::Arg => {
                Self::qualify_idents(
                    id.get(children)[1],
                    path,
                    files,
                    kinds,
                    children,
                    locations,
                    idents,
                    qualified_idents,
                    definitions_map,
                    definitions,
                    unresolved,
                );
            }
            AstKind::Bind => {
                let lhs = id.get(children)[0];
                let location = lhs
                    .get(locations)
                    .expect("must have file source to compute qualified idents");
                let slice = &location.file.get(&files.sources)
                    [location.start as usize..location.end as usize];
                lhs.put(idents, slice.to_owned());
                let path = format!("{path}.{slice}");
                lhs.put(qualified_idents, path.clone());
                definitions_map.insert(path.clone(), id);
                lhs.put(definitions, Some(id));
                id.put(definitions, Some(id));
                Self::qualify_idents(
                    id.get(children)[1],
                    &path,
                    files,
                    kinds,
                    children,
                    locations,
                    idents,
                    qualified_idents,
                    definitions_map,
                    definitions,
                    unresolved,
                );
            }
            AstKind::Proj => {
                let tgt = id.get(children)[1];
                let location = tgt
                    .get(locations)
                    .expect("must have file source to compute qualified idents");
                let slice = &location.file.get(&files.sources)
                    [location.start as usize..location.end as usize];
                tgt.put(idents, slice.to_owned());
                Self::qualify_idents(
                    id.get(children)[0],
                    path,
                    files,
                    kinds,
                    children,
                    locations,
                    idents,
                    qualified_idents,
                    definitions_map,
                    definitions,
                    unresolved,
                );
            }
            AstKind::LIdent | AstKind::UIdent => {
                let location = id
                    .get(locations)
                    .expect("must have file source to compute qualified idents");
                let slice = &location.file.get(&files.sources)
                    [location.start as usize..location.end as usize];
                id.put(idents, slice.to_owned());
                let path = format!("{path}.{slice}");
                id.put(qualified_idents, path.clone());
                unresolved.push(id);
            }
            AstKind::Block => {
                let idx = id.index();
                let path = format!("{path}.__blk{idx}");
                id.put(qualified_idents, path.clone());
                for child in id.get(children) {
                    Self::qualify_idents(
                        *child,
                        &path,
                        files,
                        kinds,
                        children,
                        locations,
                        idents,
                        qualified_idents,
                        definitions_map,
                        definitions,
                        unresolved,
                    );
                }
            }
            _ => {
                for child in id.get(children) {
                    Self::qualify_idents(
                        *child,
                        path,
                        files,
                        kinds,
                        children,
                        locations,
                        idents,
                        qualified_idents,
                        definitions_map,
                        definitions,
                        unresolved,
                    );
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn find_definitions(
        kinds: &Arena<Ast, AstKind>,
        idents: &Arena<Ast, String>,
        qualified_idents: &mut Arena<Ast, String>,
        definitions_map: &mut HashMap<String, AstId>,
        definitions: &mut Arena<Ast, Option<AstId>>,
        errors: &mut Errors,
        locations: &Arena<Ast, Option<Location>>,
        unresolved: &Vec<AstId>,
    ) {
        for id in unresolved {
            // Try to resolve identifiers to their fully qualified names
            if matches!(id.get(kinds), AstKind::LIdent | AstKind::UIdent) {
                let mut ident = id.get(qualified_idents).to_owned();
                loop {
                    // If we find an exact match, go with that
                    if let Some(def_id) = definitions_map.get(ident.as_str()) {
                        id.put(qualified_idents, ident);
                        id.put(definitions, Some(*def_id));
                        break;
                    }

                    // Go up a scope, e.g. ".Vector3.values.x" --> ".Vector3.x"
                    let rdot = ident.rfind('.').unwrap();
                    let Some(rrdot) = ident[..rdot].rfind('.') else {
                        id.put(qualified_idents, "".to_owned());
                        errors
                            .log(
                                ErrorKind::Resolve,
                                format!("Unresolved identifier \"{}\"", id.get(idents)),
                            )
                            .location_opt(*id.get(locations));
                        break;
                    };
                    ident = format!("{}{}", &ident[..rrdot], &ident[rdot..]);
                }
            } else {
                unreachable!();
            }
        }
    }

    pub fn resolve_idents(
        &mut self,
        files: &Files,
        root: AstId,
        errors: &mut Errors,
        prelude: impl Prelude,
    ) {
        let mut idents = Arena::new();
        let mut qualified_idents = Arena::new();
        let mut qualified_idents_map = HashMap::new();
        let mut definitions = Arena::new();
        let mut unresolved = Vec::new();
        prelude.apply(&mut self.ids, &mut self.kinds, &mut qualified_idents_map);
        Self::qualify_idents(
            root,
            "",
            files,
            &self.kinds,
            &self.children,
            &self.locations,
            &mut idents,
            &mut qualified_idents,
            &mut qualified_idents_map,
            &mut definitions,
            &mut unresolved,
        );
        self.idents = idents;
        self.qualified_idents = qualified_idents;
        Self::find_definitions(
            &self.kinds,
            &self.idents,
            &mut self.qualified_idents,
            &mut qualified_idents_map,
            &mut definitions,
            errors,
            &self.locations,
            &unresolved,
        );
        self.definitions = definitions;
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
                    AstKind::Float => match slice.parse::<f64>() {
                        Ok(v) => {
                            id.put(&mut self.literals, Some(Literal::Float(v)));
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
