use logos::{Lexer, Logos};

use crate::{
    arena::{Arena, Id, World},
    error::{ErrorKind, Errors},
    file::{FileId, Files},
};

#[allow(missing_docs)]
#[derive(Logos, Clone, Copy, Debug, PartialEq, Default)]
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

    #[default]
    Unknown,
}

#[derive(Default, Debug)]
pub struct Tokens {
    pub ids: World<Tokens>,
    pub kinds: Arena<Tokens, TokenKind>,
    pub locations: Arena<Tokens, Option<Location>>,
}
pub type TokenId = Id<Tokens>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    pub file: FileId,
    pub start: u32,
    pub end: u32,
}

impl Location {
    pub fn new(file: FileId, start: u32, end: u32) -> Self {
        Self { file, start, end }
    }
}

pub fn lex_file(file: FileId, files: &Files, errors: &mut Errors) -> Tokens {
    let mut tokens = Tokens::default();
    let lexer = Lexer::<TokenKind>::new(file.get(&files.sources).as_str());
    for (tok, span) in lexer.spanned() {
        let location = Location {
            file,
            start: span.start as u32,
            end: span.end as u32,
        };
        let tok = tok.unwrap_or_else(|_| {
            let slice = &file.get(&files.sources)[span.start..span.end];
            errors
                .log(ErrorKind::Lex, format!("Unrecognized token '{}'", slice))
                .location(location);
            TokenKind::Unknown
        });
        tokens
            .ids
            .alloc()
            .put(&mut tokens.kinds, tok)
            .put(&mut tokens.locations, Some(location));
    }
    tokens
}
