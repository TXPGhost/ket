use std::{
    io::Write,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use colored::Colorize;
use notify::RecursiveMode;
use tokio::runtime::Runtime;

use crate::{
    ast::Ast,
    compiler::{CompilerState, compile},
    error::Errors,
    file::{Files, read_file},
    lexer::lex_file,
    parser::Parser,
    prelude::StandardPrelude,
    types::Types,
};

pub mod arena;
pub mod ast;
pub mod compiler;
pub mod error;
pub mod file;
pub mod lexer;
pub mod lsp;
pub mod parser;
pub mod prelude;
pub mod symb;
pub mod types;

fn clear() {
    print!("\x1B[2J\x1B[1;1H");
    std::io::stdout().flush().unwrap();
}

async fn live(filename: &str) {
    let runtime = Runtime::new().expect("unable to create runtime");
    let state = Arc::new(CompilerState::default());
    let filename: Arc<str> = filename.into();
    let filename_clone = filename.clone();
    let mut debouncer =
        notify_debouncer_mini::new_debouncer(Duration::from_millis(100), move |ev| match ev {
            Ok(_) => {
                clear();
                runtime.block_on(compile(None, &filename_clone, state.clone()));
            }
            Err(e) => eprintln!("{}", e),
        })
        .unwrap();
    debouncer
        .watcher()
        .watch(Path::new(filename.as_ref()), RecursiveMode::Recursive)
        .unwrap();

    clear();
    std::thread::sleep(Duration::MAX);
}

fn main() {
    let mut args = std::env::args();
    if args.len() <= 1 {
        println!("error: expected at least one argument");
        return;
    }
    let cmd = args.nth(1).unwrap();
    match cmd.as_str() {
        "live" => {
            live(&args.next().expect("missing file name"));
        }
        "lsp" => {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(lsp::lsp_main());
        }
        _ => {
            println!("unrecognized command. valid commands are `live` and `lsp`");
        }
    }
}
