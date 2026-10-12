use std::path::{Path, PathBuf};

use jiff::{Zoned, fmt::strtime};

use crate::error::PatternError;

/// A strftime path pattern for generated PNG destinations
///
/// A leading `~/` is replaced by the home directory without formatting it.
/// Relative patterns resolve against the working directory. Environment
/// variables and other shell syntax are kept literally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPattern {
    /// Home direcotry from a leading `~/`, kept out for strftime formatting
    root: PathBuf,
    format: String,
}

impl PathPattern {
    /// Parses a pattern and validates it by expanding it at the current time.
    ///
    /// `home` is required only when the pattern starts with `~/`.
    ///
    /// # Errors
    ///
    /// Returns [`PatternError`] for empty patterns, NUL characters, a missing
    /// home directory, invalid strftime directives, or expansions that do not
    /// name a PNG file.
    pub fn new(pattern: &str, home: Option<&Path>) -> Result<Self, PatternError> {
        if pattern.is_empty() {
            return Err(PatternError::Empty);
        }

        if pattern.contains('\0') {
            return Err(PatternError::Nul);
        }

        let parsed = match pattern.strip_prefix("~/") {
            Some(rest) => Self {
                root: home.ok_or(PatternError::MissingHome)?.to_path_buf(),
                // `~//shots` stays under HOME; joining an absolute path would replace it
                format: rest.trim_start_matches('/').to_owned(),
            },
            None => Self {
                root: PathBuf::new(),
                format: pattern.to_owned(),
            },
        };

        parsed.path_at(&Zoned::now())?;
        Ok(parsed)
    }

    /// Expands the whole pattern with one timestamp.
    ///
    /// # Errors
    ///
    /// Returns [`PatternError::Format`] for invalid directives,
    /// [`PatternError::MissingFilename`] if the expansion ends in a directory,
    /// or [`PatternError::NotPng`] if its filename lacks a `.png` extension.
    pub fn path_at(&self, time: &Zoned) -> Result<PathBuf, PatternError> {
        let formatted = strtime::format(&self.format, time)?;
        let path = self.root.join(&formatted);

        if formatted.ends_with('/') || path.file_name().is_none() {
            return Err(PatternError::MissingFilename { path });
        }

        if !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        {
            return Err(PatternError::NotPng { path });
        }
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use rstest::rstest;

    #[rstest]
    #[case(
        "~/Pictures/Screenshots/%Y/%m/tvert-%Y-%m-%d_%H-%M-%S.png",
        "/home/user",
        "/home/user/Pictures/Screenshots/2026/12/tvert-2026-12-31_23-59-59.png"
    )]
    #[case("~/%Y.png", "/home/100%Y", "/home/100%Y/2026.png")]
    #[case("shots/%d/100%%.png", "/home/user", "shots/31/100%.png")]
    #[case("$HOME/%Y.PNG", "/home/user", "$HOME/2026.PNG")]
    #[case("~other/%Y.png", "/home/user", "~other/2026.png")]
    fn dated_paths_expand_with_one_timestamp(
        #[case] pattern: &str,
        #[case] home: &str,
        #[case] expected: &str,
    ) {
        use jiff::{civil::date, tz::TimeZone};

        let time = assert_ok!(date(2026, 12, 31).at(23, 59, 59, 0).to_zoned(TimeZone::UTC));
        let pattern = assert_ok!(PathPattern::new(pattern, Some(Path::new(home))));

        assert_eq!(assert_ok!(pattern.path_at(&time)), PathBuf::from(expected));
    }

    #[rstest]
    #[case("", None, "empty")]
    #[case("shot\0.png", None, "NUL")]
    #[case("~/shot.png", None, "HOME")]
    #[case("%Y/%J.png", None, "unrecognized specifier directive `J`")]
    #[case("shot%", None, "expected byte after `%`")]
    #[case("shots/%Y.png/", None, "does not end with a filename")]
    #[case("~/", Some("/home/user"), "is not a .png file")]
    #[case("shots/%Y", None, "is not a .png file")]
    #[case("shots/.png", None, "is not a .png file")]
    fn invalid_patterns_are_rejected(
        #[case] pattern: &str,
        #[case] home: Option<&str>,
        #[case] reason: &str,
    ) {
        let error = assert_err!(PathPattern::new(pattern, home.map(Path::new)));

        assert!(error.to_string().contains(reason), "{error}");
    }
}
