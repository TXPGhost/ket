use std::collections::HashSet;

use colored::Colorize;

use crate::{
    arena::{Arena, Id, World},
    ast::{Ast, AstId, AstKind, InfixKind, Literal},
    error::{ErrorKind, Errors},
};

#[derive(Default, PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub enum Type {
    // Logic Types
    Never,
    Universal,

    // Atomic types
    None,
    String,
    Char,
    I32,
    F32,
    Bool,

    // Constants
    ConstI32(i64),

    // Compound types
    Struct,
    Tuple,
    Func,
    Vector,
    Array(usize),
    Optional,

    // Internal Types
    #[default]
    Unknown, // a type which is unknown
    Weak,  // a type which can be coerced into any other type
    Error, // a type which is impossible, resulting from a type error
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
            (_, Type::Universal) => true,
            (Type::None, Type::None) => true,
            (Type::String, Type::String) => true,
            (Type::I32, Type::I32) => true,
            (Type::ConstI32(_), Type::I32) => true,
            (Type::ConstI32(x), Type::ConstI32(y)) => x == y,
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
            (Type::Array(n1), Type::Array(n2)) => {
                if n1 != n2 {
                    return false;
                }
                self.subtype(lhs.get(&self.children)[0], rhs.get(&self.children)[0])
            }
            (Type::Array(_), Type::Vector) => {
                self.subtype(lhs.get(&self.children)[0], rhs.get(&self.children)[0])
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

    fn union(&mut self, lhs: TypeId, rhs: TypeId) -> TypeId {
        if self.equal(lhs, rhs) {
            return lhs;
        }
        if self.subtype(lhs, rhs) {
            return rhs;
        }
        if self.supertype(lhs, rhs) {
            return lhs;
        }
        let tid = self.ids.alloc();
        let ty = match (*lhs.get(&self.types), *rhs.get(&self.types)) {
            (Type::ConstI32(x), Type::ConstI32(y)) if x == y => Type::ConstI32(x),
            (Type::ConstI32(_) | Type::I32, Type::ConstI32(_) | Type::I32) => Type::I32,
            (Type::Array(x), Type::Array(y)) => {
                let lhs_child = lhs.get(&self.children)[0];
                let rhs_child = lhs.get(&self.children)[0];
                let child = self.union(lhs_child, rhs_child);
                tid.get_mut(&mut self.children).push(child);
                if x == y { Type::Array(x) } else { Type::Vector }
            }
            _ => Type::Error,
        };
        *tid.get_mut(&mut self.types) = ty;
        tid
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
            AstKind::Integer => {
                if let Some(Literal::Integer(v)) = id.get(&ast.literals) {
                    self.assign_new(id, Type::ConstI32(*v))
                } else {
                    self.assign_new(id, Type::I32)
                }
            }
            AstKind::Float => self.assign_new(id, Type::F32),
            AstKind::Call => {
                let func = id.get(&ast.children)[0];
                let args = id.get(&ast.children)[1];
                let func_ty = self.compute(ast, errors, func);
                let args_ty = self.compute(ast, errors, args);

                if *func_ty.get(&self.types) == Type::Weak {
                    return self.assign_new(id, Type::Weak);
                }

                if *func_ty.get(&self.types) != Type::Func {
                    errors
                        .log(ErrorKind::Type, "Cannot call non-function type")
                        .location_opt(*id.get(&ast.locations));
                    return self.assign_new(id, Type::Error);
                }

                let func_args_ty = func_ty.get(&self.children)[0];
                let func_body_ty = func_ty.get(&self.children)[1];

                if !self.subtype(args_ty, func_args_ty) {
                    errors
                        .log(ErrorKind::Type, "Argument type mismatch")
                        .location_opt(*args.get(&ast.locations));
                    self.assign_new(id, Type::Error)
                } else {
                    self.assign(id, func_body_ty)
                }
            }
            AstKind::Method => todo!(),
            AstKind::Group => self.assign_new(id, Type::Error),
            AstKind::Func => {
                let tid = self.assign_new(id, Type::Weak);

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
                        errors
                            .log(ErrorKind::Type, "Array type mismatch")
                            .location_opt(*id.get(&ast.locations));
                        return self.assign_new(id, Type::Error);
                    }
                    old_child_tid = Some(child_tid);
                }
                if let Some(old_child_tid) = old_child_tid {
                    tid.get_mut(&mut self.children).push(old_child_tid)
                }
                tid
            }
            AstKind::Repeat => {
                let len_tid = self.compute(ast, errors, id.get(&ast.children)[0]);
                let expr_tid = self.compute(ast, errors, id.get(&ast.children)[1]);
                let tid = if let Type::ConstI32(n) = *len_tid.get(&self.types) {
                    self.assign_new(id, Type::Array(n as usize))
                } else {
                    self.assign_new(id, Type::Vector)
                };
                tid.get_mut(&mut self.children).push(expr_tid);
                tid
            }
            AstKind::Vector => {
                let expr_tid = self.compute(ast, errors, id.get(&ast.children)[0]);
                let tid = self.assign_new(id, Type::Vector);
                tid.get_mut(&mut self.children).push(expr_tid);
                tid
            }
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
                            "Assigned expression has the incorrect type",
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
                        .log(ErrorKind::Type, "If condition must be of type Bool")
                        .location_opt(*id.get(&ast.locations));
                }
                self.assign_new(id, Type::None)
            }
            AstKind::IfElse => {
                let cond = id.get(&ast.children)[0];
                let body = id.get(&ast.children)[1];
                let else_body = id.get(&ast.children)[2];
                let cond_tid = self.compute(ast, errors, cond);
                let body_tid = self.compute(ast, errors, body);
                let else_body_tid = self.compute(ast, errors, else_body);
                if *cond_tid.get(&self.types) != Type::Bool {
                    errors
                        .log(ErrorKind::Type, "If condition must be of type Bool")
                        .location_opt(*id.get(&ast.locations));
                }
                let tid = self.union(body_tid, else_body_tid);
                self.assign(id, tid)
            }
            AstKind::Infix(kind) => match kind {
                InfixKind::Add | InfixKind::Sub | InfixKind::Mul | InfixKind::Div => {
                    let lhs = id.get(&ast.children)[0];
                    let rhs = id.get(&ast.children)[1];
                    let lhs_tid = self.compute(ast, errors, lhs);
                    let rhs_tid = self.compute(ast, errors, rhs);
                    let tid = match (lhs_tid.get(&self.types), rhs_tid.get(&self.types)) {
                        (Type::Weak, Type::Weak) => {
                            rhs.put(&mut self.assignments, Some(lhs_tid));
                            Type::Weak
                        }
                        (Type::Weak, Type::I32 | Type::ConstI32(_)) => {
                            lhs_tid.put(&mut self.types, Type::I32);
                            Type::I32
                        }
                        (Type::I32 | Type::ConstI32(_), Type::Weak) => {
                            rhs_tid.put(&mut self.types, Type::I32);
                            Type::I32
                        }
                        (Type::Weak, Type::F32) => {
                            lhs_tid.put(&mut self.types, Type::F32);
                            Type::F32
                        }
                        (Type::F32, Type::Weak) => {
                            rhs_tid.put(&mut self.types, Type::F32);
                            Type::F32
                        }
                        (Type::ConstI32(x), Type::ConstI32(y)) => match kind {
                            InfixKind::Add => Type::ConstI32(x + y),
                            InfixKind::Sub => Type::ConstI32(x - y),
                            InfixKind::Mul => Type::ConstI32(x * y),
                            InfixKind::Div => Type::ConstI32(x / y),
                            _ => unreachable!(),
                        },
                        (Type::I32 | Type::ConstI32(_), Type::I32 | Type::ConstI32(_)) => Type::I32,
                        (Type::F32, Type::F32) => Type::F32,
                        _ => {
                            errors
                                .log(ErrorKind::Type, "Illegal operand")
                                .location_opt(*id.get(&ast.locations));
                            Type::Error
                        }
                    };
                    self.assign_new(id, tid)
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
                    self.assign_new(id, Type::Bool)
                }
            },
            AstKind::Error => {
                errors
                    .log(ErrorKind::Type, "Prase error when assigning type")
                    .location_opt(*id.get(&ast.locations));
                self.assign_new(id, Type::Error)
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
        match ty {
            Type::Never => print!("{}", "|".blue().bold()),
            Type::Universal => print!("{}", "&".blue().bold()),
            Type::None => print!("{}", "_".blue().bold()),
            Type::String => print!("{}", "String".blue().bold()),
            Type::Char => print!("{}", "Char".blue().bold()),
            Type::I32 => print!("{}", "I32".blue().bold()),
            Type::ConstI32(i) => print!("{}", i.to_string().blue().bold()),
            Type::F32 => print!("{}", "F32".blue().bold()),
            Type::Bool => print!("{}", "Bool".blue().bold()),
            Type::Struct | Type::Tuple => {
                print!("{}", "(".blue().bold());
                let mut first = true;
                for child in tid.get(&self.children) {
                    if !first {
                        print!("{} ", ",".blue().bold());
                    }
                    first = false;
                    self.pretty_print_type(*child, seen);
                }
                print!("{}", ")".blue().bold());
            }
            Type::Func => {
                self.pretty_print_type(tid.get(&self.children)[0], seen);
                print!(" ");
                self.pretty_print_type(tid.get(&self.children)[1], seen);
            }
            Type::Vector => {
                print!("{}", "[]".to_string().blue().bold());
                self.pretty_print_type(tid.get(&self.children)[0], seen);
            }
            Type::Array(n) => {
                print!("{}", format!("[{n}]").blue().bold());
                self.pretty_print_type(tid.get(&self.children)[0], seen);
            }
            Type::Optional => {
                self.pretty_print_type(tid.get(&self.children)[0], seen);
                print!("{}", "?".to_string().blue().bold());
            }
            Type::Unknown => print!("{}", "Unknown".blue().bold()),
            Type::Error => print!("{}", "Error".blue().bold()),
            Type::Weak => print!("{}", format!("Weak{}", tid.index()).blue().bold()),
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
