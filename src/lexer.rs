use colored::Colorize;
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
    #[regex(r"[\n\f\r]")]
    Newline,

    #[token(r",")]
    Comma,

    #[token(r"=")]
    Equals,

    #[token(r":=")]
    ColonEquals,

    #[token(r".=")]
    DotEquals,

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

    #[token(r"--")]
    MinusMinus,

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

    Unknown,

    #[default]
    EndOfFile,
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
    pub line_start: u32,
    pub line_end: u32,
    pub char_start: u32,
    pub char_end: u32,
}

impl Location {
    pub fn merge(lhs: &Option<Location>, rhs: &Option<Location>) -> Option<Location> {
        match (lhs, rhs) {
            (None, None) => None,
            (None, Some(rhs)) => Some(*rhs),
            (Some(lhs), None) => Some(*lhs),
            (Some(lhs), Some(rhs)) => {
                if lhs.file != rhs.file {
                    return None;
                };
                Some(Location {
                    file: lhs.file,
                    start: lhs.start.min(rhs.start),
                    end: lhs.end.max(rhs.end),
                    line_start: lhs.line_start.min(rhs.line_start),
                    line_end: lhs.line_end.max(rhs.line_end),
                    char_start: lhs.char_start.min(rhs.char_start),
                    char_end: lhs.char_end.max(rhs.char_end),
                })
            }
        }
    }

    pub fn pretty_print(&self, files: &Files) {
        println!(
            "{}",
            format!(
                "{}:{}{}:{}{}",
                self.file.get(&files.paths),
                self.line_start,
                if self.line_end != self.line_start {
                    format!("-{}", self.line_end)
                } else {
                    String::new()
                },
                self.char_start,
                if self.char_end != self.char_start {
                    format!("-{}", self.char_end)
                } else {
                    String::new()
                },
            )
            .bright_black()
            .bold()
        );
    }

    pub fn pretty_print_opt(opt: &Option<Self>, files: &Files) {
        match opt {
            Some(opt) => opt.pretty_print(files),
            None => println!(),
        }
    }
}

pub fn lex_file(file: FileId, files: &Files, errors: &mut Errors) -> Tokens {
    let mut tokens = Tokens::default();
    let lexer = Lexer::<TokenKind>::new(file.get(&files.sources).as_str());
    let mut line = 1;
    let mut char = 1;
    let mut last_end = 0;
    for (tok, span) in lexer.spanned() {
        let skipped = span.start as u32 - last_end;
        last_end = span.end as u32;
        char += skipped;

        let width = span.end as u32 - span.start as u32;
        let location = Location {
            file,
            start: span.start as u32,
            end: span.end as u32,
            line_start: line,
            line_end: line,
            char_start: char,
            char_end: char + width - 1,
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

        char += width;
        if tok == TokenKind::Newline {
            line += 1;
            char = 1;
        }
    }
    tokens
}
