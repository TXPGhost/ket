use std::collections::HashSet;

use colored::Colorize;

use crate::{
    arena::{Arena, Id, World},
    ast::{Ast, AstId, AstKind, InfixKind},
    error::{ErrorKind, Errors},
};

#[derive(Default, PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub enum Type {
    // Specialized Types
    Never,
    Any,

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
    Vector,
    Array(usize),
    Optional,

    // Unknown type
    #[default]
    Unknown,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub enum TypeError {
    VoidHasNoType,
    CannotCallNonFunc,
    ArgumentMismatch,
    ArrayTypeMismatch,
    AssignTypeMismatch,
    NonBooleanCondition,
    OperatorTypeMismatch,
    IllegalAst,
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
    pub fn compute_types(&mut self, ast: &Ast, errors: &mut Errors) {
        for id in ast.ids.iter() {
            self.compute(ast, errors, id);
        }
    }

    fn assign_new(&mut self, id: AstId, ty: Type) -> TypeId {
        if let Some(tid) = id.get(&self.assignments) {
            tid.put(&mut self.types, ty);
            *tid
        } else {
            let tid = self.ids.alloc();
            tid.put(&mut self.types, ty);
            id.put(&mut self.assignments, Some(tid));
            tid
        }
    }

    fn assign(&mut self, id: AstId, tid: TypeId) -> TypeId {
        id.put(&mut self.assignments, Some(tid));
        tid
    }

    fn subtype(&self, lhs: TypeId, rhs: TypeId) -> bool {
        match (lhs.get(&self.types), rhs.get(&self.types)) {
            (Type::Never, _) => true,
            (_, Type::Any) => true,
            (Type::None, Type::None) => true,
            (Type::String, Type::String) => true,
            (Type::I32, Type::I32) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::Struct, Type::Struct) => lhs == rhs,
            (Type::Tuple, Type::Tuple) => {
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
            (_, Type::Optional) => {
                let rhs = rhs.get(&self.children)[0];
                self.subtype(lhs, rhs)
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

    fn union(&mut self, lhs: AstId, rhs: AstId) -> TypeId {
        let lhs_tid = lhs.get(&self.assignments).unwrap();
        let rhs_tid = lhs.get(&self.assignments).unwrap();
        if self.equal(lhs_tid, rhs_tid) {
            return lhs_tid;
        }
        match (lhs_tid.get(&self.types), rhs_tid.get(&self.types)) {
            (Type::Any, _) => lhs_tid,
            (_, Type::Any) => rhs_tid,
            (Type::Never, _) => {
                println!("merge");
                lhs.put(&mut self.assignments, Some(rhs_tid));
                rhs_tid
            }
            (_, Type::Never) => {
                println!("merge");
                rhs.put(&mut self.assignments, Some(lhs_tid));
                lhs_tid
            }
            _ => {
                println!(
                    "unimplemented: union of {:?} ({}) and {:?} ({})",
                    lhs_tid.get(&self.types),
                    lhs_tid.index(),
                    rhs_tid.get(&self.types),
                    rhs_tid.index(),
                );
                lhs_tid
            }
        }
    }

    fn intersect(&mut self, lhs: AstId, rhs: AstId) -> TypeId {
        let lhs_tid = lhs.get(&self.assignments).unwrap();
        let rhs_tid = lhs.get(&self.assignments).unwrap();
        if self.equal(lhs_tid, rhs_tid) {
            return lhs_tid;
        }
        match (lhs_tid.get(&self.types), rhs_tid.get(&self.types)) {
            (Type::Never, _) => lhs_tid,
            (_, Type::Never) => rhs_tid,
            (Type::Any, _) => {
                println!("merge");
                lhs.put(&mut self.assignments, Some(rhs_tid));
                rhs_tid
            }
            (_, Type::Any) => {
                println!("merge");
                rhs.put(&mut self.assignments, Some(lhs_tid));
                lhs_tid
            }
            _ => {
                println!(
                    "unimplemented: intersection of {:?} ({}) and {:?} ({})",
                    lhs_tid.get(&self.types),
                    lhs_tid.index(),
                    rhs_tid.get(&self.types),
                    rhs_tid.index(),
                );
                lhs_tid
            }
        }
    }

    fn compute(&mut self, ast: &Ast, errors: &mut Errors, id: AstId) -> TypeId {
        let existing = *id.get(&self.assignments);
        if let Some(existing) = existing
            && *existing.get(&self.types) != Type::Unknown
        {
            return existing;
        }

        let kind = id.get(&ast.kinds);
        match kind {
            AstKind::LIdent | AstKind::UIdent => {
                let Some(rid) = id.get(&ast.resolved_idents) else {
                    return self.assign_new(id, Type::Unknown);
                };
                let tid = self.compute(ast, errors, *rid);
                self.assign(id, tid)
            }
            AstKind::Void => unreachable!("cannot assign type to void"),
            AstKind::String => self.assign_new(id, Type::String),
            AstKind::Char => self.assign_new(id, Type::Char),
            AstKind::None => self.assign_new(id, Type::None),
            AstKind::Integer => self.assign_new(id, Type::I32),
            AstKind::Float => self.assign_new(id, Type::F32),
            AstKind::Call => {
                let func = id.get(&ast.children)[0];
                let args = id.get(&ast.children)[1];
                let func_ty = self.compute(ast, errors, func);
                let args_ty = self.compute(ast, errors, args);

                if *func_ty.get(&self.types) == Type::Any {
                    return self.assign_new(id, Type::Any);
                }

                if *func_ty.get(&self.types) != Type::Func {
                    errors
                        .log(ErrorKind::Type, "cannot call non-function type")
                        .location_opt(*id.get(&ast.locations));
                    return self.assign_new(id, Type::Unknown);
                }

                let func_args_ty = func_ty.get(&self.children)[0];
                let func_body_ty = func_ty.get(&self.children)[1];

                if !self.subtype(args_ty, func_args_ty) {
                    errors
                        .log(ErrorKind::Type, "argument type mismatch")
                        .location_opt(*args.get(&ast.locations));
                    self.assign_new(id, Type::Unknown)
                } else {
                    self.assign(id, func_body_ty)
                }
            }
            AstKind::Method => todo!(),
            AstKind::Group => todo!(),
            AstKind::Func => {
                let tid = self.assign_new(id, Type::Any);

                let args_tid = self.compute(ast, errors, id.get(&ast.children)[0]);
                tid.get_mut(&mut self.children).push(args_tid);

                let body_tid = self.compute(ast, errors, id.get(&ast.children)[1]);
                tid.get_mut(&mut self.children).push(body_tid);

                tid.put(&mut self.types, Type::Func);

                tid
            }
            AstKind::Block => {
                let mut last_child_tid = None;
                for child in id.get(&ast.children) {
                    last_child_tid = Some(self.compute(ast, errors, *child));
                }
                if let Some(last_child_tid) = last_child_tid {
                    self.assign(id, last_child_tid)
                } else {
                    self.assign_new(id, Type::None)
                }
            }
            AstKind::Proj => todo!(),
            AstKind::Index => todo!(),
            AstKind::Struct | AstKind::Tuple => {
                let ty = match kind {
                    AstKind::Struct => Type::Struct,
                    AstKind::Tuple => Type::Tuple,
                    _ => unreachable!(),
                };
                let tid = self.assign_new(id, ty);
                for child in id.get(&ast.children) {
                    let child_tid = self.compute(ast, errors, *child);
                    tid.get_mut(&mut self.children).push(child_tid);
                }
                tid
            }
            AstKind::Array => {
                let len = id.get(&ast.children).len();
                let tid = self.assign_new(id, Type::Array(len));

                let mut old_child_tid = None;
                for child in id.get(&ast.children) {
                    let child_tid = self.compute(ast, errors, *child);
                    if let Some(old_child_tid) = old_child_tid
                        && !self.equal(old_child_tid, child_tid)
                    {
                        errors.log(ErrorKind::Type, "array type mismatch");
                        return self.assign_new(id, Type::Unknown);
                    }
                    old_child_tid = Some(child_tid);
                }
                if let Some(old_child_tid) = old_child_tid {
                    tid.get_mut(&mut self.children).push(old_child_tid)
                }
                tid
            }
            AstKind::Vector => todo!(),
            AstKind::Field => self.compute(ast, errors, id.get(&ast.children)[1]),
            AstKind::Arg => todo!(),
            AstKind::Optional => {
                let tid = self.assign_new(id, Type::Optional);
                let child_tid = self.compute(ast, errors, id.get(&ast.children)[0]);
                tid.get_mut(&mut self.children).push(child_tid);
                tid
            }
            AstKind::Bind | AstKind::BindMut => self.assign_new(id, Type::None),
            AstKind::Assign => {
                let lhs = id.get(&ast.children)[0];
                let rhs = id.get(&ast.children)[0];
                let lhs_ty = self.compute(ast, errors, lhs);
                let rhs_ty = self.compute(ast, errors, rhs);

                if !self.subtype(lhs_ty, rhs_ty) {
                    errors
                        .log(
                            ErrorKind::Type,
                            "assigned expression has the incorrect type",
                        )
                        .location_opt(*id.get(&ast.locations));
                }
                self.assign(id, lhs_ty)
            }
            AstKind::If => {
                let cond = id.get(&ast.children)[0];
                let body = id.get(&ast.children)[1];
                let cond_tid = self.compute(ast, errors, cond);
                self.compute(ast, errors, body);
                if *cond_tid.get(&self.types) != Type::Bool {
                    errors
                        .log(ErrorKind::Type, "if condition must be of type Bool")
                        .location_opt(*id.get(&ast.locations));
                }
                self.assign_new(id, Type::None)
            }
            AstKind::IfElse => {
                let cond = id.get(&ast.children)[0];
                let body = id.get(&ast.children)[1];
                let else_body = id.get(&ast.children)[2];
                let cond_tid = self.compute(ast, errors, cond);
                self.compute(ast, errors, body);
                self.compute(ast, errors, else_body);
                if *cond_tid.get(&self.types) != Type::Bool {
                    errors
                        .log(ErrorKind::Type, "if condition must be of type Bool")
                        .location_opt(*id.get(&ast.locations));
                }
                let tid = self.union(body, else_body);
                self.assign(id, tid)
            }
            AstKind::Infix(kind) => match kind {
                InfixKind::Add | InfixKind::Sub | InfixKind::Mul | InfixKind::Div => {
                    let lhs = id.get(&ast.children)[0];
                    let rhs = id.get(&ast.children)[1];
                    self.compute(ast, errors, lhs);
                    self.compute(ast, errors, rhs);
                    let tid = self.intersect(lhs, rhs);
                    self.assign(id, tid)
                }
                InfixKind::Gt
                | InfixKind::Lt
                | InfixKind::Ge
                | InfixKind::Le
                | InfixKind::Eq
                | InfixKind::Ne => {
                    let lhs = id.get(&ast.children)[0];
                    let rhs = id.get(&ast.children)[1];
                    let lhs_tid = self.compute(ast, errors, lhs);
                    let rhs_tid = self.compute(ast, errors, rhs);
                    if !self.equal(lhs_tid, rhs_tid) {
                        errors
                            .log(ErrorKind::Type, "operator types must be equal")
                            .location_opt(*id.get(&ast.locations));
                    }
                    self.assign_new(id, Type::Bool)
                }
            },
            AstKind::Error => {
                errors
                    .log(ErrorKind::Type, "prase error when assigning type")
                    .location_opt(*id.get(&ast.locations));
                self.assign_new(id, Type::Unknown)
            }
            AstKind::PrimitiveType(ty) => self.assign_new(id, *ty),
        }
    }

    fn pretty_print_type(&self, tid: TypeId, seen: &mut HashSet<TypeId>) {
        if seen.contains(&tid) {
            print!("{}", format!("[{}]", tid.index()).bright_black());
            return;
        }
        seen.insert(tid);

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
                self.pretty_print_type(*child, seen);
                first = false;
            }
            print!("{}", ")".blue());
        }
    }

    pub fn pretty_print(&self, ast: &Ast) {
        println!();
        for id in ast.ids.iter() {
            let qualified = id.get(&ast.qualified_idents);
            if qualified.is_empty() {
                continue;
            }
            let tid = id.get(&self.assignments);
            if let Some(tid) = tid {
                print!(
                    "{qualified}{}",
                    " ".repeat(20_usize.saturating_sub(qualified.len())),
                );
                self.pretty_print_type(*tid, &mut HashSet::new());
                println!();
            } else {
                print!("{} {qualified} ", "[???]".bright_black());
            }
        }
    }
}
