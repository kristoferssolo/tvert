use crate::{Error, PathPattern};
use jiff::Zoned;
use std::{
    fs::{self, File},
    io::ErrorKind,
    iter,
    path::{Path, PathBuf},
};

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
    /// A pattern expanded when saving; its parent directories are created if needed
    Pattern(PathPattern),
}

impl Destination {
    /// Uses an explicit file when provided, otherwise the path pattern
    pub fn new(file: Option<PathBuf>, pattern: PathPattern) -> Self {
        file.map_or(Self::Pattern(pattern), Self::File)
    }

    pub(crate) fn create(&self) -> Result<(PathBuf, File), Error> {
        match self {
            Self::File(path) => create_new(path).map(|file| (path.clone(), file)),
            Self::Pattern(pattern) => {
                let path = pattern.path_at(&Zoned::now())?;
                if let Some(directory) = path.parent() {
                    fs::create_dir_all(directory).map_err(|source| Error::CreateDirectory {
                        path: directory.to_path_buf(),
                        source,
                    })?;
                }
                create_unique(path)
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

/// Creates `path`, or the first free `stem-N.extension` beside it.
fn create_unique(path: PathBuf) -> Result<(PathBuf, File), Error> {
    let stem = path.file_stem().unwrap_or_default();
    let extension = path.extension().unwrap_or_default();
    let numbered = (1..=MAX_SUFFIX).map(|suffix| {
        let mut name = stem.to_os_string();
        name.push(format!("-{suffix}."));
        name.push(extension);
        path.with_file_name(name)
    });

    for candidate in iter::once(path.clone()).chain(numbered) {
        match File::create_new(&candidate) {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(source) => {
                return Err(Error::CreateDestination {
                    path: candidate,
                    source,
                });
            }
        }
    }

    Err(Error::FilenamesExhausted { path })
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_matches, assert_ok, assert_ok_eq};
    use tempfile::tempdir;

    fn pattern(root: &Path, rest: &str) -> PathPattern {
        let pattern = format!("{}/{rest}", root.display());
        assert_ok!(PathPattern::new(&pattern, None))
    }

    #[test]
    fn explicit_file_takes_precedence_and_is_never_replaced() {
        let root = assert_ok!(tempdir());
        let file = root.path().join("explicit.png");
        let directory = root.path().join("screenshots");
        let destination = Destination::new(
            Some(file.clone()),
            pattern(root.path(), "screenshots/%Y.png"),
        );

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
    fn generated_directories_are_created_only_when_saving() {
        let root = assert_ok!(tempdir());
        let directory = root.path().join("screenshots");
        let destination = Destination::new(None, pattern(root.path(), "screenshots/%Y/%m/%s.png"));
        assert!(!directory.exists());

        let (created, _) = assert_ok!(destination.create());
        assert!(created.starts_with(&directory));
        assert_eq!(created.extension(), Some("png".as_ref()));
    }

    #[test]
    fn generated_name_collisions_add_a_suffix_in_the_generated_directory() {
        let root = assert_ok!(tempdir());
        let directory = root.path().join("shots.v2");
        let destination = Destination::new(None, pattern(root.path(), "shots.v2/shot.v1.PNG"));

        let (first, _) = assert_ok!(destination.create());
        assert_ok!(fs::write(&first, "kept"));
        assert_ok!(fs::write(directory.join("shot.v1-1.PNG"), "kept"));

        let (created, _) = assert_ok!(destination.create());
        assert_eq!(created, directory.join("shot.v1-2.PNG"));
        assert_ok_eq!(fs::read_to_string(first), "kept");
    }
}
