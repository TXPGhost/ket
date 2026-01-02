use crate::arena::{Arena, Ref};

pub struct File;
pub type FileSources = Arena<File, String>;
pub type FilePaths = Arena<File, Option<String>>;

pub fn read_file(
    path: &str,
    sources: &mut FileSources,
    paths: &mut FilePaths,
) -> Result<Ref<File>, std::io::Error> {
    let file = sources.alloc(std::fs::read_to_string(path)?);
    file.put(paths, Some(path.to_owned()));
    Ok(file)
}
