use std::collections::HashMap;

use crate::{
    arena::{Arena, Id, World},
    ast::{Ast, AstKind},
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
        let mut add_primitive = |s: &str, k: AstKind| {
            let id = ids.alloc();
            id.put(kinds, k);
            qualified_idents_map.insert(format!(".{s}"), id);
        };

        add_primitive("I32", AstKind::PrimitiveI32);
        add_primitive("F32", AstKind::PrimitiveF32);
        add_primitive("String", AstKind::PrimitiveString);
        add_primitive("Char", AstKind::PrimitiveChar);
    }
}
