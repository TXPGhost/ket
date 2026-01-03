use colored::Colorize;

use crate::{
    arena::{Arena, Id, World},
    file::Files,
    lexer::Location,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ErrorKind {
    Io,
    Lex,
    Parse,

    #[default]
    Unknown,
}

#[derive(Default, Debug)]
pub struct Errors {
    pub ids: World<Errors>,
    pub kinds: Arena<Errors, ErrorKind>,
    pub messages: Arena<Errors, String>,
    pub locations: Arena<Errors, Option<Location>>,
    pub causes: Arena<Errors, Option<ErrorId>>,
    is_root: Arena<Errors, bool>,
}
pub type ErrorId = Id<Errors>;

impl Errors {
    pub fn log<'a>(&'a mut self, kind: ErrorKind, message: impl Into<String>) -> ErrorRef<'a> {
        let id = self
            .ids
            .alloc()
            .put(&mut self.kinds, kind)
            .put(&mut self.messages, message.into());
        ErrorRef { errors: self, id }
    }

    fn print_location(&self, err: ErrorId, files: &Files) {
        if let Some(location) = err.get(&self.locations) {
            println!(
                "{}",
                format!(
                    " {}:{}:{}",
                    location.file.get(&files.paths),
                    location.start,
                    location.end
                )
                .bright_black()
                .bold()
            );
        } else {
            println!();
        }
    }

    pub fn pretty_print(&mut self, files: &Files) {
        self.compute_roots();
        for mut err in self.ids.iter() {
            if *err.get(&self.is_root) {
                let error_text = format!("{:?} Error", err.get(&self.kinds));
                let error_text_len = error_text.len();
                print!(
                    "{}{} {}",
                    error_text.bright_red().bold(),
                    ":".bold(),
                    err.get(&self.messages).bold()
                );
                self.print_location(err, files);
                while let Some(cause) = *err.get(&self.causes) {
                    print!(
                        "{}  {}",
                        " ".repeat(error_text_len),
                        cause.get(&self.messages).bold()
                    );
                    self.print_location(cause, files);
                    err = cause;
                }
            }
        }
    }

    pub fn has_errors(&self) -> bool {
        self.ids.num_allocs() > 0
    }

    fn compute_roots(&mut self) {
        for err in self.ids.iter() {
            err.put(&mut self.is_root, true);
        }
        for err in self.ids.iter() {
            if let Some(cause) = err.get(&self.causes) {
                cause.put(&mut self.is_root, false);
            }
        }
    }
}

pub struct ErrorRef<'a> {
    pub id: ErrorId,
    errors: &'a mut Errors,
}

impl<'a> ErrorRef<'a> {
    pub fn at_location(self, location: Location) -> Self {
        self.id.put(&mut self.errors.locations, Some(location));
        self
    }

    pub fn caused_by(self, kind: ErrorKind, message: impl Into<String>) -> Self {
        let id = self.errors.log(kind, message).id;
        id.put(&mut self.errors.causes, Some(self.id));
        Self {
            errors: self.errors,
            id,
        }
    }

    pub fn cause_of(self, kind: ErrorKind, message: impl Into<String>) -> Self {
        let id = self.errors.log(kind, message).id;
        self.id.put(&mut self.errors.causes, Some(id));
        Self {
            errors: self.errors,
            id,
        }
    }
}

pub trait ErrorExt {
    fn err_at_location(self, location: Location) -> Self;
    fn err_caused_by(self, kind: ErrorKind, message: impl Into<String>) -> Self;
    fn err_cause_of(self, kind: ErrorKind, message: impl Into<String>) -> Self;
}

impl<'a, T> ErrorExt for Result<T, ErrorRef<'a>> {
    fn err_at_location(self, location: Location) -> Self {
        self.map_err(|err| err.at_location(location))
    }

    fn err_caused_by(self, kind: ErrorKind, message: impl Into<String>) -> Self {
        self.map_err(|err| err.caused_by(kind, message))
    }

    fn err_cause_of(self, kind: ErrorKind, message: impl Into<String>) -> Self {
        self.map_err(|err| err.cause_of(kind, message))
    }
}
