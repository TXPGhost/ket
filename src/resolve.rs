use colored::Colorize;
use std::collections::HashMap;

use crate::{
    arena::Arena,
    ast::{Ast, AstId, AstKind},
    errors::{ErrorKind, Errors},
    file::Files,
    lexer::Location,
};

pub trait Prelude {
    fn apply(&self, resolver: &mut Resolver<'_>);
}

pub struct StandardPrelude;
impl Prelude for StandardPrelude {
    fn apply(&self, resolver: &mut Resolver<'_>) {
        let mut add_primitive = |s: &str, k: AstKind| {
            let id = resolver.ast.ids.alloc();
            id.put(&mut resolver.ast.kinds, k);
            resolver.definitions_map.insert(format!(".{s}"), id);
        };

        add_primitive("I32", AstKind::PrimitiveI32);
        add_primitive("F32", AstKind::PrimitiveF32);
        add_primitive("String", AstKind::PrimitiveString);
        add_primitive("Char", AstKind::PrimitiveChar);
        add_primitive("Bool", AstKind::PrimitiveBool);
        add_primitive("true", AstKind::PrimitiveTrue);
        add_primitive("false", AstKind::PrimitiveFalse);
    }
}

#[derive(Debug, Default)]
pub struct Symbols {
    pub qualified_idents: Arena<Ast, String>,
    pub definitions: Arena<Ast, Option<AstId>>,
}

pub struct Resolver<'a> {
    pub ast: &'a mut Ast,
    pub symbols: &'a mut Symbols,
    pub errors: &'a mut Errors,
    pub definitions_map: HashMap<String, AstId>,
    pub undefined_ids: Vec<AstId>,
}

impl<'a> Resolver<'a> {
    pub fn new(ast: &'a mut Ast, symbols: &'a mut Symbols, errors: &'a mut Errors) -> Self {
        Self {
            ast,
            symbols,
            errors,
            definitions_map: HashMap::new(),
            undefined_ids: Vec::new(),
        }
    }

    pub fn resolve(mut self, root: AstId, prelude: impl Prelude) {
        prelude.apply(&mut self);
        self.remove_grouping(root);
        self.qualify_idents(root, "");
        self.find_definitions();
    }
}

impl Resolver<'_> {
    pub fn qualify_and_define_self(&mut self, id: AstId, ident: &str) {
        id.put(&mut self.symbols.qualified_idents, ident.to_owned());
        id.put(&mut self.symbols.definitions, Some(id));
        self.definitions_map.insert(ident.to_owned(), id);
    }

    pub fn qualify_only(&mut self, id: AstId, ident: &str) {
        id.put(&mut self.symbols.qualified_idents, ident.to_owned());
        self.undefined_ids.push(id);
    }

    pub fn qualify_idents(&mut self, id: AstId, path: &str) {
        let kind = id.get(&self.ast.kinds);
        match kind {
            AstKind::VField | AstKind::TField => {
                let ident = id.get(&self.ast.idents);
                let path = format!("{path}.{ident}");
                self.qualify_and_define_self(id, &path);
                self.qualify_idents(id.get(&self.ast.children)[0], &path);
            }
            AstKind::VArg | AstKind::TArg => {
                self.qualify_idents(id.get(&self.ast.children)[0], path);
            }
            AstKind::Bind => {
                let ident = id.get(&self.ast.idents);
                let path = format!("{path}.{ident}");
                self.qualify_and_define_self(id, &path);
                self.qualify_idents(id.get(&self.ast.children)[0], &path);
            }
            AstKind::Proj => {
                self.qualify_idents(id.get(&self.ast.children)[0], path);
            }
            AstKind::VIdent | AstKind::TIdent => {
                let ident = id.get(&self.ast.idents);
                let path = format!("{path}.{ident}");
                self.qualify_only(id, &path);
            }
            AstKind::Block => {
                let idx = id.index();
                let path = format!("{path}.__blk{idx}");
                self.qualify_and_define_self(id, &path);
                for i in 0..id.get(&self.ast.children).len() {
                    let child = id.get(&self.ast.children)[i];
                    self.qualify_idents(child, &path);
                }
            }
            _ => {
                for i in 0..id.get(&self.ast.children).len() {
                    let child = id.get(&self.ast.children)[i];
                    self.qualify_idents(child, path);
                }
            }
        }
    }

    pub fn find_definitions(&mut self) {
        println!("map is {:?}", self.symbols.qualified_idents);
        println!("map is {:?}", self.definitions_map);
        for id in self.undefined_ids.iter() {
            // Try to resolve identifiers to their fully qualified names
            if matches!(id.get(&self.ast.kinds), AstKind::VIdent | AstKind::TIdent) {
                let mut ident = id.get(&self.symbols.qualified_idents).to_owned();
                assert!(
                    !ident.is_empty(),
                    "unqualified identifier \"{}\"",
                    id.get(&self.ast.idents)
                );
                loop {
                    // If we find an exact match, go with that
                    if let Some(def_id) = self.definitions_map.get(ident.as_str()) {
                        id.put(&mut self.symbols.qualified_idents, ident);
                        id.put(&mut self.symbols.definitions, Some(*def_id));
                        break;
                    }

                    // Go up a scope, e.g. ".Vector3.values.x" --> ".Vector3.x"
                    let rdot = ident.rfind('.').unwrap();
                    let Some(rrdot) = ident[..rdot].rfind('.') else {
                        id.put(&mut self.symbols.qualified_idents, "".to_owned());
                        self.errors
                            .log(
                                ErrorKind::Resolve,
                                format!("Unresolved identifier \"{}\"", id.get(&self.ast.idents)),
                            )
                            .location_opt(*id.get(&self.ast.locations));
                        break;
                    };
                    ident = format!("{}{}", &ident[..rrdot], &ident[rdot..]);
                }
            } else {
                unreachable!();
            }
        }
    }

    pub fn remove_grouping(&mut self, root: AstId) {
        for i in 0..root.get(&self.ast.children).len() {
            let child = root.get(&self.ast.children)[i];
            self.remove_grouping(child);
            if *child.get(&self.ast.kinds) == AstKind::Group {
                let loc = *child.get(&self.ast.children)[0].get(&self.ast.locations);
                root.get_mut(&mut self.ast.children)[i].put(&mut self.ast.locations, loc);
                root.get_mut(&mut self.ast.children)[i] = child.get(&self.ast.children)[0];
            }
        }
    }
}

impl Symbols {
    fn pretty_print_indented(&self, id: AstId, indent: usize, ast: &Ast, files: &Files) {
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
        let kind_str = format!("{:?}", id.get(&ast.kinds));
        let len = indent * 2 + kind_str.len();
        print!("{}{} ", "  ".repeat(indent), kind_str.bold().magenta());
        let location = id.get(&ast.locations);
        let ident = id.get(&self.qualified_idents);
        if !ident.is_empty() {
            print!("{} ", ident);
        }
        print!("{} ", " ".repeat(36_usize.saturating_sub(len)));
        Location::pretty_print_opt(location, files);
        for child in id.get(&ast.children) {
            self.pretty_print_indented(*child, indent + 1, ast, files);
        }
    }

    pub fn pretty_print(&self, id: AstId, ast: &Ast, files: &Files) {
        println!();
        self.pretty_print_indented(id, 0, ast, files);
    }
}
