use logos::{Lexer, Logos};
use thiserror::Error;

use crate::{
    arena::{Arena, Ref},
    file::{File, FileSources},
};

#[allow(missing_docs)]
#[derive(Logos, Clone, Copy, Debug, PartialEq)]
#[logos(skip r"[ \t]+")]
pub enum TokenKind {
    #[regex(r"[\n\f\r]+")]
    Newline,

    #[token(r",")]
    Comma,

    #[token(r"=")]
    Equals,

    #[token(r":=")]
    ColonEquals,

    #[token(r"==")]
    EqualsEquals,

    #[token(r"!=")]
    NotEquals,

    #[token(r"..")]
    DotDot,

    #[token(r".")]
    Dot,

    #[token(r":")]
    Colon,

    #[token(r"$")]
    DollarSign,

    #[token(r"::")]
    ColonColon,

    #[token(r";")]
    Semicolon,

    #[token(r"&")]
    Ampersand,

    #[token(r"?")]
    QuestionMark,

    #[token(r"<-")]
    LeftArrow,

    #[token(r"->")]
    RightArrow,

    #[token(r"|")]
    Bar,

    #[token(r"+")]
    Plus,

    #[token(r"++")]
    PlusPlus,

    #[token(r"-")]
    Minus,

    #[token(r"*")]
    Times,

    #[token(r"**")]
    TimesTimes,

    #[token(r"/")]
    Divide,

    #[token(r"//")]
    DivideDivide,

    #[token(r"<<")]
    LShift,

    #[token(r">>")]
    RShift,

    #[token(r"%")]
    Percent,

    #[token(r"(")]
    LParen,

    #[token(r")")]
    RParen,

    #[token(r"{")]
    LCurl,

    #[token(r"}")]
    RCurl,

    #[token(r"[")]
    LSquare,

    #[token(r"]")]
    RSquare,

    #[token(r"<")]
    LAngle,

    #[token(r">")]
    RAngle,

    #[regex(r"[\d]+")]
    Integer,

    #[regex(r"[\d]+\.[\d]+")]
    Float,

    #[regex(r"_")]
    Underscore,

    #[regex(r"[A-Z][a-zA-Z0-9]*")]
    UIdent,

    #[regex(r"[a-z][_a-z0-9]*")]
    LIdent,

    #[regex(r#"["]([^"\\\n]|\\.|\\\n)*["]"#)]
    String,

    #[regex(r#"[']([^'\\\n]|\\.|\\\n)*[']"#)]
    Character,
}

pub struct Token;
pub type TokenKinds = Arena<Token, TokenKind>;
pub type TokenLocations = Arena<Token, Option<Location>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    file: Ref<File>,
    start: u32,
    end: u32,
}

#[derive(Error, Debug)]
pub enum LexError {
    #[error("an invalid token was encountered")]
    InvalidToken(String),
}

pub fn lex_file(
    file: Ref<File>,
    sources: &FileSources,
    tokens: &mut TokenKinds,
    locations: &mut TokenLocations,
) -> Result<(), LexError> {
    let lexer = Lexer::<TokenKind>::new(file.get(sources).as_str());
    for (tok, span) in lexer.spanned() {
        match tok {
            Ok(tok) => {
                let tok_id = tokens.alloc(tok);
                tok_id.put(
                    locations,
                    Some(Location {
                        file,
                        start: span.start as u32,
                        end: span.end as u32,
                    }),
                )
            }
            Err(_) => return Err(LexError::InvalidToken("todo!".to_owned())),
        }
    }
    Ok(())
}
