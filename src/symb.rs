use std::collections::HashMap;

use crate::{
    arena::Arena,
    ast::{Ast, AstId, AstKind},
    error::{ErrorKind, Errors},
    file::Files,
    prelude::Prelude,
};

#[derive(Debug, Default)]
pub struct Symbols {
    pub qualified_idents: Arena<Ast, String>,
    pub definitions: Arena<Ast, Option<AstId>>,
}

impl Ast {
    pub fn qualify_idents(
        &self,
        id: AstId,
        path: &str,
        files: &Files,
        qualified_idents: &mut Arena<Ast, String>,
        definitions_map: &mut HashMap<String, AstId>,
        definitions: &mut Arena<Ast, Option<AstId>>,
        unresolved: &mut Vec<AstId>,
    ) {
        let kind = id.get(&self.kinds);
        match kind {
            AstKind::Field => {
                let field_id = id.get(&self.children)[0];
                let ident = field_id.get(&self.idents);
                let path = format!("{path}.{ident}");
                field_id.put(qualified_idents, path.clone());
                definitions_map.insert(path.clone(), id);
                field_id.put(definitions, Some(id));
                id.put(definitions, Some(id));
                self.qualify_idents(
                    id.get(&self.children)[1],
                    &path,
                    files,
                    qualified_idents,
                    definitions_map,
                    definitions,
                    unresolved,
                );
            }
            AstKind::Arg => {
                self.qualify_idents(
                    id.get(&self.children)[1],
                    path,
                    files,
                    qualified_idents,
                    definitions_map,
                    definitions,
                    unresolved,
                );
            }
            AstKind::Bind => {
                let lhs = id.get(&self.children)[0];
                let ident = lhs.get(&self.idents);
                let path = format!("{path}.{ident}");
                lhs.put(qualified_idents, path.clone());
                definitions_map.insert(path.clone(), id);
                lhs.put(definitions, Some(id));
                id.put(definitions, Some(id));
                self.qualify_idents(
                    id.get(&self.children)[1],
                    &path,
                    files,
                    qualified_idents,
                    definitions_map,
                    definitions,
                    unresolved,
                );
            }
            AstKind::Proj => {
                self.qualify_idents(
                    id.get(&self.children)[0],
                    path,
                    files,
                    qualified_idents,
                    definitions_map,
                    definitions,
                    unresolved,
                );
            }
            AstKind::LIdent | AstKind::UIdent => {
                let ident = id.get(&self.idents);
                let path = format!("{path}.{ident}");
                id.put(qualified_idents, path.clone());
                unresolved.push(id);
            }
            AstKind::Block => {
                let idx = id.index();
                let path = format!("{path}.__blk{idx}");
                id.put(qualified_idents, path.clone());
                for child in id.get(&self.children) {
                    self.qualify_idents(
                        *child,
                        &path,
                        files,
                        qualified_idents,
                        definitions_map,
                        definitions,
                        unresolved,
                    );
                }
            }
            _ => {
                for child in id.get(&self.children) {
                    self.qualify_idents(
                        *child,
                        path,
                        files,
                        qualified_idents,
                        definitions_map,
                        definitions,
                        unresolved,
                    );
                }
            }
        }
    }

    pub fn find_definitions(
        &self,
        qualified_idents: &mut Arena<Ast, String>,
        definitions_map: &mut HashMap<String, AstId>,
        definitions: &mut Arena<Ast, Option<AstId>>,
        errors: &mut Errors,
        unresolved: &Vec<AstId>,
    ) {
        for id in unresolved {
            // Try to resolve identifiers to their fully qualified names
            if matches!(id.get(&self.kinds), AstKind::LIdent | AstKind::UIdent) {
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
                                format!("Unresolved identifier \"{}\"", id.get(&self.idents)),
                            )
                            .location_opt(*id.get(&self.locations));
                        break;
                    };
                    ident = format!("{}{}", &ident[..rrdot], &ident[rdot..]);
                }
            } else {
                unreachable!();
            }
        }
    }

    pub fn resolve_symbols(
        &mut self,
        files: &Files,
        root: AstId,
        errors: &mut Errors,
        prelude: impl Prelude,
    ) -> Symbols {
        let mut qualified_idents = Arena::new();
        let mut qualified_idents_map = HashMap::new();
        let mut definitions = Arena::new();
        let mut unresolved = Vec::new();
        prelude.apply(&mut self.ids, &mut self.kinds, &mut qualified_idents_map);
        self.qualify_idents(
            root,
            "",
            &files,
            &mut qualified_idents,
            &mut qualified_idents_map,
            &mut definitions,
            &mut unresolved,
        );
        self.find_definitions(
            &mut qualified_idents,
            &mut qualified_idents_map,
            &mut definitions,
            errors,
            &unresolved,
        );
        Symbols {
            qualified_idents,
            definitions,
        }
    }

    pub fn simplify(&mut self, root: AstId) {
        for i in 0..root.get(&self.children).len() {
            let child = root.get(&self.children)[i];
            self.simplify(child);
            if *child.get(&self.kinds) == AstKind::Group {
                let loc = *child.get(&self.children)[0].get(&self.locations);
                root.get_mut(&mut self.children)[i].put(&mut self.locations, loc);
                root.get_mut(&mut self.children)[i] = child.get(&self.children)[0];
            }
        }
    }
}
