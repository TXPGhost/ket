use std::sync::Arc;

use crate::ast::AstId;
use crate::file::{read_file, read_string};
use crate::lexer::Lexer;
use crate::resolve::{Resolver, Symbols};
use crate::{
    ast::Ast, errors::Errors, file::Files, parser::Parser, resolve::StandardPrelude, types::Types,
};
use tokio::sync::Mutex;

#[derive(Debug, Default)]
pub struct CompilerState {
    pub errors: Mutex<Errors>,
    pub files: Mutex<Files>,
    pub ast: Mutex<Ast>,
    pub symbols: Mutex<Symbols>,
    pub types: Mutex<Types>,
    pub root: Mutex<Option<AstId>>,
}

pub async fn compile(contents: Option<String>, path: &str, state: Arc<CompilerState>) {
    let mut errors = Errors::default();
    let mut files = Files::default();
    let mut ast = Ast::default();
    let mut symbols = Symbols::default();
    let mut types = Types::default();

    let file = match contents {
        Some(contents) => read_string(contents, path, &mut files),
        None => read_file(path, &mut files, &mut errors),
    };
    let Some(file) = file else {
        return;
    };

    let tokens = Lexer::new(&files, &mut errors).lex(file);
    let root = Parser::new(&tokens, &mut ast, &files, &mut errors).parse();
    Resolver::new(&mut ast, &mut symbols, &mut errors).resolve(root, StandardPrelude);
    types.compute_types(&ast, &mut symbols, &mut errors);

    *state.errors.lock().await = errors;
    *state.files.lock().await = files;
    *state.ast.lock().await = ast;
    *state.types.lock().await = types;
    *state.symbols.lock().await = symbols;
    *state.root.lock().await = Some(root);
}
