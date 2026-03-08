use std::collections::HashMap;

use crate::{
    arena::Arena,
    ast::{Ast, AstId, AstKind},
    errors::{ErrorKind, Errors},
    file::Files,
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
            resolver.qualified_idents_map.insert(format!(".{s}"), id);
        };

        add_primitive("I32", AstKind::PrimitiveI32);
        add_primitive("F32", AstKind::PrimitiveF32);
        add_primitive("String", AstKind::PrimitiveString);
        add_primitive("Char", AstKind::PrimitiveChar);
    }
}

#[derive(Debug, Default)]
pub struct Symbols {
    pub qualified_idents: Arena<Ast, String>,
    pub definitions: Arena<Ast, Option<AstId>>,
}

pub struct Resolver<'a> {
    pub ast: &'a mut Ast,
    pub files: &'a Files,
    pub symbols: &'a mut Symbols,
    pub qualified_idents_map: &'a mut HashMap<String, AstId>,
    pub definitions_map: &'a mut HashMap<String, AstId>,
    pub unresolved: &'a mut Vec<AstId>,
}

pub fn resolve_symbols(
    ast: &mut Ast,
    files: &Files,
    root: AstId,
    errors: &mut Errors,
    prelude: impl Prelude,
) -> Symbols {
    let mut resolver = Resolver {
        files,
        ast,
        symbols: &mut Symbols::default(),
        qualified_idents_map: &mut HashMap::new(),
        definitions_map: &mut HashMap::new(),
        unresolved: &mut Vec::new(),
    };

    resolver.remove_grouping(root);
    prelude.apply(&mut resolver);
    resolver.qualify_idents(root, "");
    resolver.find_definitions(errors);
    std::mem::take(resolver.symbols)
}

impl Resolver<'_> {
    pub fn qualify_idents(&mut self, id: AstId, path: &str) {
        let kind = id.get(&self.ast.kinds);
        match kind {
            AstKind::Field => {
                let field_id = id.get(&self.ast.children)[0];
                let ident = field_id.get(&self.ast.idents);
                let path = format!("{path}.{ident}");
                field_id.put(&mut self.symbols.qualified_idents, path.clone());
                self.definitions_map.insert(path.clone(), id);
                field_id.put(&mut self.symbols.definitions, Some(id));
                id.put(&mut self.symbols.definitions, Some(id));
                self.qualify_idents(id.get(&self.ast.children)[1], &path);
            }
            AstKind::Arg => {
                self.qualify_idents(id.get(&self.ast.children)[1], path);
            }
            AstKind::Bind => {
                let lhs = id.get(&self.ast.children)[0];
                let ident = lhs.get(&self.ast.idents);
                let path = format!("{path}.{ident}");
                lhs.put(&mut self.symbols.qualified_idents, path.clone());
                self.definitions_map.insert(path.clone(), id);
                lhs.put(&mut self.symbols.definitions, Some(id));
                id.put(&mut self.symbols.definitions, Some(id));
                self.qualify_idents(id.get(&self.ast.children)[1], &path);
            }
            AstKind::Proj => {
                self.qualify_idents(id.get(&self.ast.children)[0], path);
            }
            AstKind::LIdent | AstKind::UIdent => {
                let ident = id.get(&self.ast.idents);
                let path = format!("{path}.{ident}");
                id.put(&mut self.symbols.qualified_idents, path.clone());
                self.unresolved.push(id);
            }
            AstKind::Block => {
                let idx = id.index();
                let path = format!("{path}.__blk{idx}");
                id.put(&mut self.symbols.qualified_idents, path.clone());
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

    pub fn find_definitions(&mut self, errors: &mut Errors) {
        for id in self.unresolved.iter() {
            // Try to resolve identifiers to their fully qualified names
            if matches!(id.get(&self.ast.kinds), AstKind::LIdent | AstKind::UIdent) {
                let mut ident = id.get(&self.symbols.qualified_idents).to_owned();
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
                        errors
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
