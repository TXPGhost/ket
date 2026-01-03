use std::time::Instant;

use colored::Colorize;

use crate::{
    error::Errors,
    file::{Files, read_file},
    lexer::lex_file,
    parser::{Ast, Parser},
};

pub mod arena;
pub mod error;
pub mod file;
pub mod lexer;
pub mod parser;

fn main() {
    let mut errors = Errors::default();
    let mut files = Files::default();
    let mut ast = Ast::default();

    let filenames = ["test/lex_test.ket", "haha"];

    let begin = Instant::now();
    for filename in filenames {
        println!(
            "{}{} \"{}\"",
            "Compiling".bright_green().bold(),
            ":".bold(),
            filename
        );
        let file = read_file(filename, &mut files, &mut errors);
        if let Some(file) = file {
            let tokens = lex_file(file, &files, &mut errors);
            let parser = Parser::new(&tokens, &mut ast, &mut errors);
            let _ = parser.parse();
        }
    }

    let elapsed = begin.elapsed().as_secs_f32();
    if errors.has_errors() {
        errors.pretty_print(&files);
        println!("Finished with errors in in {:.4} secs", elapsed);
    } else {
        println!("Finished in {:.4} secs", elapsed);
    }
}
