use std::{
    fs::{self, File},
    io::ErrorKind,
    iter,
    path::{Path, PathBuf},
};

use jiff::Zoned;

use crate::Error;

/// Numbered alternatives tried after a generated filename is taken
const MAX_SUFFIX: u16 = 999;

/// Where a saved PNG is created
///
/// Constructing a destination does not touch the filesystem. Files and
/// directories are created only when saving.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// A file that must not already exist
    File(PathBuf),
    /// A directory that receives a timestamped filename and is created if needed
    Directory(PathBuf),
}

impl Destination {
    /// Uses an explicit file when provided, otherwise the default directory.
    pub fn new(file: Option<PathBuf>, directory: PathBuf) -> Self {
        file.map_or(Self::Directory(directory), Self::File)
    }

    pub(crate) fn create(&self) -> Result<(PathBuf, File), Error> {
        match self {
            Self::File(path) => create_new(path).map(|file| (path.clone(), file)),
            Self::Directory(directory) => {
                fs::create_dir_all(directory).map_err(|source| Error::CreateDirectory {
                    path: directory.clone(),
                    source,
                })?;
                let stem = Zoned::now().strftime("tvert-%Y-%m-%d_%H-%M-%S").to_string();
                create_unique(directory, &stem)
            }
        }
    }
}

fn create_new(path: &Path) -> Result<File, Error> {
    File::create_new(path).map_err(|source| Error::CreateDestination {
        path: path.to_path_buf(),
        source,
    })
}

fn create_unique(directory: &Path, stem: &str) -> Result<(PathBuf, File), Error> {
    let numbered = (1..=MAX_SUFFIX).map(|suffix| format!("{stem}-{suffix}.png"));

    for name in iter::once(format!("{stem}.png")).chain(numbered) {
        let path = directory.join(name);
        match File::create_new(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(source) => return Err(Error::CreateDestination { path, source }),
        }
    }

    Err(Error::FilenamesExhausted {
        directory: directory.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_matches, assert_ok, assert_ok_eq};
    use tempfile::tempdir;

    #[test]
    fn explicit_file_takes_precedence_and_is_never_replaced() {
        let root = assert_ok!(tempdir());
        let file = root.path().join("explicit.png");
        let directory = root.path().join("screenshots");
        let destination = Destination::new(Some(file.clone()), directory.clone());

        let (created, _) = assert_ok!(destination.create());
        assert_eq!(created, file);
        assert!(!directory.exists());

        assert_matches!(
            destination.create(),
            Err(Error::CreateDestination { path, source })
                if path == file && source.kind() == ErrorKind::AlreadyExists
        );
    }

    #[test]
    fn default_directory_is_created_only_when_saving() {
        let root = assert_ok!(tempdir());
        let directory = root.path().join("nested/screenshots");
        let destination = Destination::new(None, directory.clone());
        assert!(!directory.exists());

        let (created, _) = assert_ok!(destination.create());
        assert_eq!(created.parent(), Some(directory.as_path()));
        assert_eq!(created.extension(), Some("png".as_ref()));
    }

    #[test]
    fn generated_name_collisions_add_the_next_free_suffix() {
        let directory = assert_ok!(tempdir());
        let taken = directory.path().join("shot.png");
        assert_ok!(fs::write(&taken, "kept"));
        assert_ok!(fs::write(directory.path().join("shot-1.png"), "kept"));

        let (created, _) = assert_ok!(create_unique(directory.path(), "shot"));
        assert_eq!(created, directory.path().join("shot-2.png"));
        assert_ok_eq!(fs::read_to_string(taken), "kept");
    }
}
