use std::error::Error;

use crate::{
    file::{FilePaths, FileSources, read_file},
    lexer::{TokenKinds, TokenLocations, lex_file},
};

pub mod arena;
pub mod file;
pub mod lexer;
pub mod parser;

fn main() -> Result<(), Box<dyn Error>> {
    let mut sources = FileSources::new();
    let mut paths = FilePaths::new();
    let mut tokens = TokenKinds::new();
    let mut locations = TokenLocations::new();

    let file = read_file("test/lex_test.ket", &mut sources, &mut paths)?;
    lex_file(file, &sources, &mut tokens, &mut locations).unwrap();

    dbg!(&sources);
    dbg!(&paths);
    dbg!(&tokens);
    dbg!(&locations);

    Ok(())
}
