use crate::error::Error;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
};

fn normalized(path: &Path) -> Result<PathBuf, Error> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut result = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    // Resolve existing ancestors, including symlinks, before detecting aliases.
    let mut ancestor = result.as_path();
    let mut tail = Vec::new();
    while !ancestor.exists() {
        tail.push(
            ancestor
                .file_name()
                .ok_or_else(|| Error::Input("invalid output path".into()))?
                .to_owned(),
        );
        ancestor = ancestor
            .parent()
            .ok_or_else(|| Error::Input("invalid output path".into()))?;
    }
    let mut resolved = ancestor.canonicalize()?;
    for part in tail.into_iter().rev() {
        resolved.push(part);
    }
    Ok(resolved)
}

pub(crate) fn check(input: &Path, paths: &[PathBuf]) -> Result<(), Error> {
    let input = normalized(input)?;
    let mut destinations = Vec::new();
    for path in paths {
        let destination = normalized(path)?;
        if destination == input || destinations.contains(&destination) {
            return Err(Error::Input(
                "input and output destinations must be distinct".into(),
            ));
        }
        if fs::symlink_metadata(path).is_ok() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("refusing to overwrite {}", path.display()),
            )));
        }
        destinations.push(destination);
    }
    Ok(())
}

/// Reserve every destination with create_new before writing. If anything fails,
/// remove only files created by this operation. Existing files are never opened.
pub(crate) fn write_all(files: Vec<(PathBuf, Vec<u8>)>) -> Result<(), Error> {
    let mut reserved: Vec<(PathBuf, File)> = Vec::new();
    let result = (|| {
        for (path, _) in &files {
            let file = OpenOptions::new().write(true).create_new(true).open(path)?;
            reserved.push((path.clone(), file));
        }
        for ((_, file), (_, bytes)) in reserved.iter_mut().zip(&files) {
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        Ok(())
    })();
    if result.is_err() {
        for (path, file) in reserved {
            drop(file);
            let _ = fs::remove_file(path);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lexical_aliases_are_equal() {
        assert_eq!(
            normalized(Path::new("./foo/../bar.svg")).unwrap(),
            normalized(Path::new("bar.svg")).unwrap()
        );
    }
}
