use std::{
    io::Write,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use colored::Colorize;
use notify::RecursiveMode;

use crate::{
    ast::Ast,
    error::Errors,
    file::{Files, read_file},
    lexer::lex_file,
    parser::Parser,
    prelude::StandardPrelude,
    types::Types,
};

pub mod arena;
pub mod ast;
pub mod error;
pub mod file;
pub mod lexer;
pub mod parser;
pub mod prelude;
pub mod types;

fn clear() {
    print!("\x1B[2J\x1B[1;1H");
    std::io::stdout().flush().unwrap();
}

fn compile(filename: Arc<str>) {
    let mut errors = Errors::default();
    let mut files = Files::default();
    let mut ast = Ast::default();
    let mut types = Types::default();

    let filenames = [filename];

    let begin = Instant::now();
    for filename in filenames {
        println!(
            "{}{} \"{}\"",
            "Compiling".bright_green().bold(),
            ":".bold(),
            filename
        );
        let file = read_file(filename.as_ref(), &mut files, &mut errors);
        if let Some(file) = file {
            let tokens = lex_file(file, &files, &mut errors);
            let root = Parser::new(&tokens, &mut ast, &mut errors).parse();
            ast.qualify_and_resolve(&files, root, StandardPrelude);
            ast.pretty_print(root, &files);
            types.compute_types(&ast);
            types.pretty_print(&ast);
        }
    }

    let elapsed = begin.elapsed().as_secs_f32();
    if errors.has_errors() {
        errors.pretty_print(&files);
        println!("\nFinished with errors in in {:.4} secs", elapsed);
    } else {
        println!("\nFinished in {:.4} secs", elapsed);
    }
}

fn live(filename: &str) {
    let filename: Arc<str> = filename.into();
    let filename_clone = filename.clone();
    let mut debouncer =
        notify_debouncer_mini::new_debouncer(Duration::from_millis(100), move |ev| match ev {
            Ok(_) => {
                clear();
                compile(filename_clone.clone());
            }
            Err(e) => eprintln!("{}", e),
        })
        .unwrap();
    debouncer
        .watcher()
        .watch(Path::new(filename.as_ref()), RecursiveMode::Recursive)
        .unwrap();

    clear();
    compile(filename.clone());
    std::thread::sleep(Duration::MAX);
}

fn main() {
    let mut args = std::env::args();
    if args.len() <= 1 {
        println!("error: expected at least one argument");
        return;
    }
    live(&args.nth(1).unwrap());
}
