use std::collections::HashMap;

use crate::{
    arena::{Arena, Id, World},
    ast::{Ast, AstKind},
    types::Type,
};

pub trait Prelude {
    fn apply(
        &self,
        ids: &mut World<Ast>,
        kinds: &mut Arena<Ast, AstKind>,
        qualified_idents_map: &mut HashMap<String, Id<Ast>>,
    );
}

pub struct StandardPrelude;
impl Prelude for StandardPrelude {
    fn apply(
        &self,
        ids: &mut World<Ast>,
        kinds: &mut Arena<Ast, AstKind>,
        qualified_idents_map: &mut HashMap<String, Id<Ast>>,
    ) {
        let mut add_primitive = |s: &str, t: Type| {
            let id = ids.alloc();
            id.put(kinds, AstKind::PrimitiveType(t));
            qualified_idents_map.insert(format!(".{s}"), id);
        };

        add_primitive("I32", Type::I32);
        add_primitive("F32", Type::F32);
        add_primitive("String", Type::String);
        add_primitive("Char", Type::Char);
    }
}
