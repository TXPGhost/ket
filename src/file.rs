use crate::{
    arena::{Arena, Id, World},
    error::{ErrorKind, Errors},
};

#[derive(Default, Debug)]
pub struct Files {
    pub ids: World<Files>,
    pub sources: Arena<Files, String>,
    pub paths: Arena<Files, String>,
}
pub type FileId = Id<Files>;

pub fn read_file(path: &str, files: &mut Files, errors: &mut Errors) -> Option<FileId> {
    let contents = std::fs::read_to_string(path);
    match contents {
        Ok(contents) => {
            let id = files
                .ids
                .alloc()
                .put(&mut files.sources, contents)
                .put(&mut files.paths, path.to_owned());
            Some(id)
        }
        Err(err) => {
            errors
                .log(ErrorKind::Io, format!("{}", err))
                .caused_by(ErrorKind::Io, format!("Unable to open file \"{}\"", path));
            None
        }
    }
}

pub fn read_string(contents: String, path: &str, files: &mut Files) -> Option<FileId> {
    let id = files
        .ids
        .alloc()
        .put(&mut files.sources, contents)
        .put(&mut files.paths, path.to_owned());
    Some(id)
}
