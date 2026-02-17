use colored::Colorize;

use crate::{
    arena::{Arena, Id, World},
    ast::{Ast, AstId, AstKind, InfixKind},
};

#[derive(Default, PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub enum Type {
    // Atomic types
    None,
    String,
    Char,
    I32,
    F32,
    Bool,

    // Compound types
    Struct,
    Tuple,
    Func,
    Args,

    // Error types
    #[default]
    Unknown,
    Error,
}

#[derive(Default, Debug)]
pub struct Types {
    pub ids: World<Types>,
    pub types: Arena<Types, Type>,
    pub children: Arena<Types, Vec<TypeId>>,
    pub assignments: Arena<Ast, Option<TypeId>>,
}
pub type TypeId = Id<Types>;

impl Types {
    pub fn compute_types(&mut self, ast: &Ast) {
        for id in ast.ids.iter() {
            self.compute(ast, id);
        }
    }

    fn assign_new(&mut self, id: AstId, ty: Type) -> TypeId {
        let tid = self.ids.alloc();
        tid.put(&mut self.types, ty);
        id.put(&mut self.assignments, Some(tid));
        tid
    }

    fn assign(&mut self, id: AstId, tid: TypeId) -> TypeId {
        id.put(&mut self.assignments, Some(tid));
        tid
    }

    fn subtype(&self, lhs: TypeId, rhs: TypeId) -> bool {
        match (lhs.get(&self.types), rhs.get(&self.types)) {
            (Type::None, Type::None) => true,
            (Type::String, Type::String) => true,
            (Type::I32, Type::I32) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::Struct, Type::Struct) => lhs == rhs,
            (Type::Args, Type::Args) => {
                let lhs_children = lhs.get(&self.children);
                let rhs_children = rhs.get(&self.children);
                if lhs_children.len() != rhs_children.len() {
                    return false;
                }
                for i in 0..lhs_children.len() {
                    if !self.subtype(lhs_children[i], rhs_children[i]) {
                        return false;
                    }
                }
                true
            }
            (Type::Func, Type::Func) => {
                let lhs_children = lhs.get(&self.children);
                let rhs_children = rhs.get(&self.children);
                if !self.supertype(lhs_children[0], rhs_children[0]) {
                    return false;
                }
                if !self.subtype(lhs_children[1], rhs_children[1]) {
                    return false;
                }
                true
            }
            _ => false,
        }
    }

    fn supertype(&self, lhs: TypeId, rhs: TypeId) -> bool {
        self.subtype(rhs, lhs)
    }

    fn equal(&self, lhs: TypeId, rhs: TypeId) -> bool {
        self.subtype(lhs, rhs) && self.subtype(rhs, lhs)
    }

    fn compute(&mut self, ast: &Ast, id: AstId) -> TypeId {
        let existing = *id.get(&self.assignments);
        if let Some(existing) = existing
            && *existing.get(&self.types) != Type::Unknown
        {
            return existing;
        }

        let kind = id.get(&ast.kinds);
        match kind {
            AstKind::LIdent | AstKind::UIdent => {
                let rid = id
                    .get(&ast.resolved_idents)
                    .unwrap_or_else(|| panic!("unresolved identifier ({})", id.index()));
                let tid = self.compute(ast, rid);
                self.assign(id, tid)
            }
            AstKind::Void => self.assign_new(id, Type::Error),
            AstKind::String => self.assign_new(id, Type::String),
            AstKind::Char => self.assign_new(id, Type::Char),
            AstKind::None => self.assign_new(id, Type::None),
            AstKind::Integer => self.assign_new(id, Type::I32),
            AstKind::Float => self.assign_new(id, Type::F32),
            AstKind::Call => todo!(),
            AstKind::Method => todo!(),
            AstKind::Group => todo!(),
            AstKind::Func => {
                let tid = self.assign_new(id, Type::Func);
                let args_tid = self.compute(ast, id.get(&ast.children)[0]);
                let body_tid = self.compute(ast, id.get(&ast.children)[1]);
                tid.get_mut(&mut self.children).push(args_tid);
                tid.get_mut(&mut self.children).push(body_tid);
                tid
            }
            AstKind::Block => {
                let mut last_child_tid = None;
                for child in id.get(&ast.children) {
                    last_child_tid = Some(self.compute(ast, *child));
                }
                if let Some(last_child_tid) = last_child_tid {
                    self.assign(id, last_child_tid)
                } else {
                    self.assign_new(id, Type::None)
                }
            }
            AstKind::Proj => todo!(),
            AstKind::Index => todo!(),
            AstKind::Struct | AstKind::Args | AstKind::Tuple => {
                let ty = match kind {
                    AstKind::Struct => Type::Struct,
                    AstKind::Args => Type::Args,
                    AstKind::Tuple => Type::Tuple,
                    _ => unreachable!(),
                };
                let tid = self.assign_new(id, ty);
                for child in id.get(&ast.children) {
                    let child_tid = self.compute(ast, *child);
                    tid.get_mut(&mut self.children).push(child_tid);
                }
                tid
            }
            AstKind::Array => todo!(),
            AstKind::Vector => todo!(),
            AstKind::Field => self.compute(ast, id.get(&ast.children)[1]),
            AstKind::Arg => todo!(),
            AstKind::Optional => todo!(),
            AstKind::Bind | AstKind::BindMut => self.assign_new(id, Type::None),
            AstKind::Assign => todo!("should this return the old value (for linearity)?"),
            AstKind::If => todo!(),
            AstKind::Infix(kind) => match kind {
                InfixKind::Add | InfixKind::Sub | InfixKind::Mul | InfixKind::Div => {
                    let lhs = id.get(&ast.children)[0];
                    let rhs = id.get(&ast.children)[1];
                    let lhs_tid = self.compute(ast, lhs);
                    let rhs_tid = self.compute(ast, rhs);
                    if !self.equal(lhs_tid, rhs_tid) {
                        self.assign_new(id, Type::Error)
                    } else {
                        self.assign(id, lhs_tid)
                    }
                }
            },
            AstKind::Error => self.assign_new(id, Type::Error),
            AstKind::PrimitiveType(ty) => self.assign_new(id, *ty),
        }
    }

    fn pretty_print_type(&self, tid: TypeId) {
        let ty = tid.get(&self.types);
        let ty = format!("{ty:?}");
        print!("{}", ty.blue().bold());

        let children = tid.get(&self.children);
        if !children.is_empty() {
            print!(" {}", "(".blue());
            let mut first = true;
            for child in children {
                if !first {
                    print!("{} ", ",".blue());
                }
                self.pretty_print_type(*child);
                first = false;
            }
            print!("{}", ")".blue());
        }
    }

    pub fn pretty_print(&self, ast: &Ast) {
        println!();
        for id in ast.ids.iter() {
            let qualified = id.get(&ast.qualified_idents);
            if !qualified.is_empty() {
                print!("{qualified} ");
                let tid = id.get(&self.assignments);
                if let Some(tid) = tid {
                    self.pretty_print_type(*tid);
                    println!();
                }
            }
        }
    }
}
