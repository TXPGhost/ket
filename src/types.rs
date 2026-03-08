use colored::Colorize;

use crate::{
    arena::{Arena, Id, World},
    ast::{Ast, AstId, AstKind, InfixKind, Literal},
    errors::{ErrorKind, Errors},
    resolve::Symbols,
};

#[derive(Default, PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub enum Type {
    // Primitive types
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

#[derive(Debug, Default)]
pub struct StructData {
    pub struct_field_id: Option<AstId>,
    pub child_field_ids: Vec<AstId>,
}

#[derive(Default, Debug)]
pub struct Types {
    pub ids: World<Types>,
    pub types: Arena<Types, Type>,
    pub children: Arena<Types, Vec<TypeId>>,
    pub struct_data: Arena<Types, Option<Box<StructData>>>,
    pub assignments: Arena<Ast, Option<TypeId>>,
}
pub type TypeId = Id<Types>;

impl Types {
    pub fn compute_types(&mut self, ast: &Ast, symbols: &mut Symbols, errors: &mut Errors) {
        for id in ast.ids.clone().iter() {
            self.compute(ast, symbols, errors, id);
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
        let lhs_ty = lhs.get(&self.types);
        let rhs_ty = rhs.get(&self.types);
        match (lhs_ty, rhs_ty) {
            (Type::None, Type::None) => true,
            (Type::String, Type::String) => true,
            (Type::Char, Type::Char) => true,
            (Type::I32, Type::I32) => true,
            (Type::F32, Type::F32) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::ConstI32(_), Type::I32) => true,
            (Type::ConstI32(x), Type::ConstI32(y)) => x == y,
            (Type::Struct, Type::Struct) => lhs == rhs,
            (Type::Tuple, Type::Tuple | Type::Struct) => {
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

    fn compute(
        &mut self,
        ast: &Ast,
        symbols: &mut Symbols,
        errors: &mut Errors,
        id: AstId,
    ) -> TypeId {
        let existing = *id.get(&self.assignments);
        if let Some(existing) = existing
            && *existing.get(&self.types) != Type::Unknown
        {
            return existing;
        }

        let kind = *id.get(&ast.kinds);
        match kind {
            AstKind::LIdent | AstKind::UIdent => {
                let Some(def_id) = id.get(&symbols.definitions) else {
                    return self.assign_new(id, Type::Unknown);
                };
                let kind = def_id.get(&ast.kinds);
                match kind {
                    AstKind::Bind | AstKind::Field => {
                        let tid = self.compute(ast, symbols, errors, def_id.get(&ast.children)[1]);
                        self.assign(id, tid)
                    }
                    AstKind::PrimitiveI32 => self.assign_new(id, Type::I32),
                    AstKind::PrimitiveF32 => self.assign_new(id, Type::F32),
                    AstKind::PrimitiveString => self.assign_new(id, Type::String),
                    AstKind::PrimitiveChar => self.assign_new(id, Type::Char),
                    _ => unreachable!("unreachable non-definition type: {kind:?}"),
                }
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
                let func_ty = self.compute(ast, symbols, errors, func);
                let args_ty = self.compute(ast, symbols, errors, args);

                let (params_ty, result_ty) = match func_ty.get(&self.types) {
                    Type::Struct => (func_ty, func_ty),
                    Type::Func => (
                        func_ty.get(&self.children)[0],
                        func_ty.get(&self.children)[1],
                    ),
                    Type::Unknown => {
                        return self.assign_new(id, Type::Unknown);
                    }
                    _ => {
                        errors
                            .log(ErrorKind::Type, "Cannot call non-function type")
                            .location_opt(*id.get(&ast.locations));
                        return self.assign_new(id, Type::Error);
                    }
                };

                let n_args = args_ty.get(&self.children).len();
                let n_params = params_ty.get(&self.children).len();
                if n_args < n_params {
                    errors
                        .log(
                            ErrorKind::Type,
                            format!(
                                "Not enough arguments: expected {} arguments but found {}",
                                n_params, n_args
                            ),
                        )
                        .location_opt(*args.get(&ast.locations));
                } else if n_args > n_params {
                    errors
                        .log(
                            ErrorKind::Type,
                            format!(
                                "Too many arguments: expected {} arguments but found {}",
                                n_params, n_args
                            ),
                        )
                        .location_opt(*args.get(&ast.locations));
                } else {
                    for i in 0..n_args {
                        let arg_ty = args_ty.get(&self.children)[i];
                        let param_ty = params_ty.get(&self.children)[i];

                        if !self.subtype(arg_ty, param_ty) {
                            errors
                                .log(
                                    ErrorKind::Type,
                                    format!(
                                        "Argument type mismatch: expected {} but found {}",
                                        self.string_of_type(param_ty, false, ast),
                                        self.string_of_type(arg_ty, false, ast),
                                    ),
                                )
                                .location_opt(*args.get(&ast.children)[i].get(&ast.locations));
                        }
                    }
                }
                self.assign(id, result_ty)
            }
            AstKind::Method => {
                errors
                    .log(ErrorKind::Type, "TODO: implement Method type checker")
                    .location_opt(*id.get(&ast.locations));
                self.assign_new(id, Type::Unknown)
            }
            AstKind::Group => self.assign_new(id, Type::Error),
            AstKind::Func => {
                let tid = self.assign_new(id, Type::Unknown);

                let args_tid = self.compute(ast, symbols, errors, id.get(&ast.children)[0]);
                tid.get_mut(&mut self.children).push(args_tid);

                let body_tid = self.compute(ast, symbols, errors, id.get(&ast.children)[1]);
                tid.get_mut(&mut self.children).push(body_tid);

                tid.put(&mut self.types, Type::Func);

                tid
            }
            AstKind::Block => {
                let mut last_child_tid = None;
                for i in 0..id.get(&ast.children).len() {
                    let child = id.get(&ast.children)[i];
                    last_child_tid = Some(self.compute(ast, symbols, errors, child));
                }
                if let Some(last_child_tid) = last_child_tid {
                    self.assign(id, last_child_tid)
                } else {
                    self.assign_new(id, Type::None)
                }
            }
            AstKind::Proj => {
                let base = id.get(&ast.children)[0];
                let field = id.get(&ast.children)[1];

                let base_ty = self.compute(ast, symbols, errors, base);

                match base_ty.get(&self.types) {
                    Type::Tuple => {
                        if !matches!(field.get(&ast.kinds), AstKind::Integer) {
                            errors
                                .log(ErrorKind::Type, "Tuple field name must be an integer")
                                .location_opt(*id.get(&ast.locations));
                            return self.assign_new(id, Type::Error);
                        }
                        let Some(Literal::Integer(index)) = field.get(&ast.literals) else {
                            unreachable!();
                        };
                        let index = *index as usize;
                        let num_children = base_ty.get(&self.children).len();
                        if num_children <= index {
                            errors
                                .log(
                                    ErrorKind::Type,
                                    format!(
                                        "Tuple index ({}) is out of bounds for length ({})",
                                        index, num_children,
                                    ),
                                )
                                .location_opt(*field.get(&ast.locations));
                            return self.assign_new(id, Type::Error);
                        }
                        let tid = base_ty.get(&self.children)[index];
                        self.assign(id, tid)
                    }
                    Type::Struct => {
                        if !matches!(field.get(&ast.kinds), AstKind::LIdent | AstKind::UIdent) {
                            errors
                                .log(ErrorKind::Type, "Struct field name must be an identifier")
                                .location_opt(*id.get(&ast.locations));
                            return self.assign_new(id, Type::Error);
                        }
                        let ident = field.get(&ast.idents);
                        let struct_data = base_ty
                            .get(&self.struct_data)
                            .as_ref()
                            .expect("struct should have struct data");
                        for (field_tid, field_id) in base_ty
                            .get(&self.children)
                            .iter()
                            .zip(struct_data.child_field_ids.iter())
                        {
                            if field_id.get(&ast.children)[0].get(&ast.idents) == ident {
                                field.put(&mut symbols.definitions, Some(*field_id));
                                return self.assign(id, *field_tid);
                            }
                        }
                        errors
                            .log(
                                ErrorKind::Type,
                                format!(
                                    "No such field \"{}\" on struct \"{}\"",
                                    ident,
                                    base.get(&ast.idents)
                                ),
                            )
                            .location_opt(*id.get(&ast.locations));
                        self.assign_new(id, Type::Error)
                    }
                    _ => {
                        errors
                            .log(
                                ErrorKind::Type,
                                "Projection operator not supported for the given type",
                            )
                            .location_opt(*id.get(&ast.locations));
                        self.assign_new(id, Type::Error)
                    }
                }
            }
            AstKind::Index => {
                errors
                    .log(ErrorKind::Type, "TODO: implement Index type checker")
                    .location_opt(*id.get(&ast.locations));
                self.assign_new(id, Type::Unknown)
            }
            AstKind::Tuple => {
                let tid = self.assign_new(id, Type::Tuple);
                for i in 0..id.get(&ast.children).len() {
                    let child = id.get(&ast.children)[i];
                    let child_tid = self.compute(ast, symbols, errors, child);
                    tid.get_mut(&mut self.children).push(child_tid);
                }
                tid
            }
            AstKind::Struct => {
                let tid = self.assign_new(id, Type::Struct);
                let mut struct_data = StructData::default();
                for i in 0..id.get(&ast.children).len() {
                    let child = id.get(&ast.children)[i];
                    let child_tid = self.compute(ast, symbols, errors, child);
                    tid.get_mut(&mut self.children).push(child_tid);
                    struct_data.child_field_ids.push(child);
                }
                tid.put(&mut self.struct_data, Some(Box::new(struct_data)));
                tid
            }
            AstKind::Array => {
                let len = id.get(&ast.children).len();
                let tid = self.assign_new(id, Type::Array(len));

                let mut old_child_tid = None;
                for i in 0..id.get(&ast.children).len() {
                    let child = id.get(&ast.children)[i];
                    let child_tid = self.compute(ast, symbols, errors, child);
                    if let Some(old_child_tid) = &mut old_child_tid {
                        *old_child_tid = self.union(*old_child_tid, child_tid);
                    } else {
                        old_child_tid = Some(child_tid);
                    }
                }
                if let Some(old_child_tid) = old_child_tid {
                    tid.get_mut(&mut self.children).push(old_child_tid)
                } else {
                    let child_tid = self.ids.alloc();
                    child_tid.put(&mut self.types, Type::Unknown);
                    tid.get_mut(&mut self.children).push(child_tid);
                }
                tid
            }
            AstKind::Repeat => {
                let len_tid = self.compute(ast, symbols, errors, id.get(&ast.children)[0]);
                let expr_tid = self.compute(ast, symbols, errors, id.get(&ast.children)[1]);
                let tid = if let Type::ConstI32(n) = *len_tid.get(&self.types) {
                    self.assign_new(id, Type::Array(n as usize))
                } else {
                    self.assign_new(id, Type::Vector)
                };
                tid.get_mut(&mut self.children).push(expr_tid);
                tid
            }
            AstKind::Vector => {
                let expr_tid = self.compute(ast, symbols, errors, id.get(&ast.children)[0]);
                let tid = self.assign_new(id, Type::Vector);
                tid.get_mut(&mut self.children).push(expr_tid);
                tid
            }
            AstKind::Field => {
                let tid = self.compute(ast, symbols, errors, id.get(&ast.children)[1]);
                if *id.get(&ast.children)[1].get(&ast.kinds) == AstKind::Struct {
                    tid.get_mut(&mut self.struct_data)
                        .as_mut()
                        .expect("struct should have struct data")
                        .struct_field_id = Some(id.get(&ast.children)[0]);
                }
                self.assign(id.get(&ast.children)[0], tid);
                tid
            }
            AstKind::Arg => {
                let tid = self.compute(ast, symbols, errors, id.get(&ast.children)[1]);
                self.assign(id, tid)
            }
            AstKind::Optional => {
                let tid = self.assign_new(id, Type::Optional);
                let child_tid = self.compute(ast, symbols, errors, id.get(&ast.children)[0]);
                tid.get_mut(&mut self.children).push(child_tid);
                tid
            }
            AstKind::Bind | AstKind::BindMut => self.assign_new(id, Type::None),
            AstKind::Assign => {
                let lhs = id.get(&ast.children)[0];
                let rhs = id.get(&ast.children)[0];
                let lhs_ty = self.compute(ast, symbols, errors, lhs);
                let rhs_ty = self.compute(ast, symbols, errors, rhs);

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
                let cond_tid = self.compute(ast, symbols, errors, cond);
                self.compute(ast, symbols, errors, body);
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
                let cond_tid = self.compute(ast, symbols, errors, cond);
                let body_tid = self.compute(ast, symbols, errors, body);
                let else_body_tid = self.compute(ast, symbols, errors, else_body);
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
                    let lhs_tid = self.compute(ast, symbols, errors, lhs);
                    let rhs_tid = self.compute(ast, symbols, errors, rhs);
                    let tid = match (lhs_tid.get(&self.types), rhs_tid.get(&self.types)) {
                        (Type::Unknown, Type::Unknown) => {
                            rhs.put(&mut self.assignments, Some(lhs_tid));
                            Type::Unknown
                        }
                        (Type::Unknown, Type::I32 | Type::ConstI32(_)) => {
                            lhs_tid.put(&mut self.types, Type::I32);
                            Type::I32
                        }
                        (Type::I32 | Type::ConstI32(_), Type::Unknown) => {
                            rhs_tid.put(&mut self.types, Type::I32);
                            Type::I32
                        }
                        (Type::Unknown, Type::F32) => {
                            lhs_tid.put(&mut self.types, Type::F32);
                            Type::F32
                        }
                        (Type::F32, Type::Unknown) => {
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
                                .log(
                                    ErrorKind::Type,
                                    format!("Illegal operand for {kind:?} operator"),
                                )
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
                    // TODO: actually check
                    // let lhs = id.get(&ast.children)[0];
                    // let rhs = id.get(&ast.children)[1];
                    // let lhs_tid = self.compute(ast, errors, lhs);
                    // let rhs_tid = self.compute(ast, errors, rhs);
                    self.assign_new(id, Type::Bool)
                }
            },
            AstKind::Error => self.assign_new(id, Type::Unknown),
            AstKind::PrimitiveI32 => self.assign_new(id, Type::I32),
            AstKind::PrimitiveF32 => self.assign_new(id, Type::F32),
            AstKind::PrimitiveString => self.assign_new(id, Type::String),
            AstKind::PrimitiveChar => self.assign_new(id, Type::Char),
        }
    }

    fn write_type_into(&self, tid: TypeId, expand: bool, ast: &Ast, buf: &mut String) {
        let ty = tid.get(&self.types);
        match ty {
            Type::None => *buf += "_",
            Type::String => *buf += "__String",
            Type::Char => *buf += "__Char",
            Type::I32 => *buf += "__I32",
            Type::ConstI32(i) => *buf += &i.to_string(),
            Type::F32 => *buf += "__F32",
            Type::Bool => *buf += "__Bool",
            Type::Tuple => {
                *buf += "(";
                let mut first = true;
                for child in tid.get(&self.children) {
                    if !first {
                        *buf += ", ";
                    }
                    first = false;
                    self.write_type_into(*child, false, ast, buf);
                }
                *buf += ")";
            }
            Type::Struct => {
                let struct_data = tid
                    .get(&self.struct_data)
                    .as_ref()
                    .expect("struct should have struct data");
                let id = struct_data.struct_field_id;
                // TODO: expand should be an identifier (not a bool)
                // TODO: is this even the right approach?
                // - we want expansion when it's the definition _site_ of a struct
                if expand || id.map(|id| id.get(&ast.kinds)) != Some(&AstKind::UIdent) {
                    *buf += "(";
                    let mut first = true;
                    for (child, child_id) in tid
                        .get(&self.children)
                        .iter()
                        .zip(struct_data.child_field_ids.iter())
                    {
                        if !first {
                            *buf += ", ";
                        }
                        first = false;
                        *buf += child_id.get(&ast.children)[0].get(&ast.idents).as_str();
                        *buf += " ";
                        self.write_type_into(*child, false, ast, buf);
                    }
                    *buf += ")";
                } else if let Some(id) = id {
                    *buf += id.get(&ast.idents).as_str();
                } else {
                    *buf += "(...)";
                }
            }
            Type::Func => {
                self.write_type_into(tid.get(&self.children)[0], false, ast, buf);
                *buf += " ";
                self.write_type_into(tid.get(&self.children)[1], false, ast, buf);
            }
            Type::Vector => {
                *buf += "[]";
                self.write_type_into(tid.get(&self.children)[0], false, ast, buf);
            }
            Type::Array(0) => {
                *buf += "[]";
            }
            Type::Array(n) => {
                *buf += &format!("[{n}]");
                self.write_type_into(tid.get(&self.children)[0], false, ast, buf);
            }
            Type::Optional => {
                self.write_type_into(tid.get(&self.children)[0], false, ast, buf);
                *buf += "?";
            }
            Type::Unknown => *buf += &format!("__Unknown{}", tid.index()),
            Type::Error => *buf += "__Error",
        }
    }

    pub fn string_of_type(&self, tid: TypeId, expand: bool, ast: &Ast) -> String {
        let mut result = String::new();
        self.write_type_into(tid, expand, ast, &mut result);
        result
    }

    pub fn pretty_print(&self, ast: &Ast, symbols: &Symbols) {
        println!();
        for id in ast.ids.iter() {
            let qualified = id.get(&symbols.qualified_idents);
            if qualified.is_empty() {
                continue;
            }
            let tid = id.get(&self.assignments);
            if let Some(tid) = tid {
                print!(
                    "{qualified}{}",
                    " ".repeat(44_usize.saturating_sub(qualified.len())),
                );
                println!("{}", self.string_of_type(*tid, true, ast).blue().bold());
            } else {
                print!("{} {qualified} ", "[???]".bright_black());
            }
        }
    }
}
