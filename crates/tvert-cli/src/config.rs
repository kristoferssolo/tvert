use crate::error::ConfigError;
use serde::Deserialize;
use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};
use toml::{
    Deserializer, Spanned,
    de::{DeTable, DeValue},
};
use tvert_editor::Settings;

type Result<T, E = ConfigError> = std::result::Result<T, E>;

const DEFAULT_CONFIG: &str = include_str!("../../../configs/config.toml");
const DEFAULT_CONFIG_NAME: &str = "configs/config.toml (embedded)";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    editor: EditorConfig,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EditorConfig {
    stroke_width: u16,
    pixelation_block_size: u16,
    blur_sigma: u16,
}

impl TryFrom<EditorConfig> for Settings {
    type Error = tvert_editor::Error;

    fn try_from(config: EditorConfig) -> Result<Self, Self::Error> {
        let settings = Self {
            stroke_width: config.stroke_width,
            pixelation_block_size: config.pixelation_block_size,
            blur_sigma: config.blur_sigma,
        };
        settings.validate()?;
        Ok(settings)
    }
}

/// Loads editor settings from the embedded defaults and an optional user file.
///
/// # Errors
///
/// Returns an error for unreadable files, invalid TOML, unknown settings, or
/// unsupported values. A missing explicitly requested file is also an error.
pub fn load(explicit: Option<&Path>) -> Result<Settings> {
    let Some(path) = explicit.map(Path::to_path_buf).or_else(default_path) else {
        return parse(DEFAULT_CONFIG_NAME, "");
    };
    load_path(&path, explicit.is_some())
}

fn default_path() -> Option<PathBuf> {
    let directory = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .map(|home| home.join(".config"))
        })?;
    Some(directory.join("tvert/config.toml"))
}

fn load_path(path: &Path, required: bool) -> Result<Settings, ConfigError> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == ErrorKind::NotFound && !required => {
            return parse(DEFAULT_CONFIG_NAME, "");
        }
        Err(error) => {
            return Err(ConfigError::Read {
                path: path.to_path_buf(),
                cause: error,
            });
        }
    };
    parse(&path.display().to_string(), &source)
}

fn parse(name: &str, source: &str) -> Result<Settings> {
    let defaults = parse_document(DEFAULT_CONFIG_NAME, DEFAULT_CONFIG)?;

    // Validate defaults before mixing spans from two different sources
    let settings = decode(&defaults, DEFAULT_CONFIG_NAME, DEFAULT_CONFIG)?;
    if source.is_empty() {
        return Ok(settings);
    }

    let mut overrides = parse_document(name, source)?;
    fill_defaults(overrides.get_mut(), defaults.into_inner());
    decode(&overrides, name, source)
}

fn parse_document<'a>(name: &str, source: &'a str) -> Result<Spanned<DeTable<'a>>> {
    DeTable::parse(source).map_err(|error| ConfigError::toml(name, source, error))
}

fn decode(document: &Spanned<DeTable<'_>>, name: &str, source: &str) -> Result<Settings> {
    let config = Config::deserialize(Deserializer::from(document.clone()))
        .map_err(|error| ConfigError::toml(name, source, error))?;

    config
        .editor
        .try_into()
        .map_err(|error| ConfigError::setting(name, source, document.get_ref(), error))
}

fn fill_defaults<'a>(config: &mut DeTable<'a>, defaults: DeTable<'a>) {
    for (key, default) in defaults {
        match config.get_mut(key.get_ref().as_ref()) {
            Some(value) => {
                if let (DeValue::Table(config), DeValue::Table(defaults)) =
                    (value.get_mut(), default.into_inner())
                {
                    fill_defaults(config, defaults);
                }
            }
            None => {
                config.insert(key, default);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok, assert_ok_eq, assert_some, assert_some_eq};
    use miette::Diagnostic;

    #[test]
    fn partial_configuration_keeps_unspecified_defaults() {
        let defaults = assert_ok!(parse("test.toml", ""));
        let settings = assert_ok!(parse("test.toml", "[editor]\nstroke_width = 7"));

        assert_eq!(
            settings,
            Settings {
                stroke_width: 7,
                ..defaults
            }
        );
    }

    #[test]
    fn invalid_values_and_unknown_keys_are_rejected() {
        for source in [
            "[editor]\nstroke_width = 0",
            "[editor]\npixelation_block_size = 65",
            "[editor]\nblur_sigma = 0",
            "[editor]\nstroke_width = 'wide'",
            "[editor]\nstorke_width = 4",
            "[unknown]\nvalue = 1",
            "editor = false",
            "[editor]\nstroke_width = [4]",
            "[editor",
        ] {
            let _ = assert_err!(parse("test.toml", source));
        }
    }

    #[test]
    fn missing_default_file_is_optional_but_explicit_file_is_required() {
        let directory = assert_ok!(tempfile::tempdir());
        let path = directory.path().join("missing.toml");
        let defaults = assert_ok!(parse("test.toml", ""));

        assert_ok_eq!(load_path(&path, false), defaults);
        let _ = assert_err!(load_path(&path, true));
    }

    #[test]
    fn user_file_overrides_embedded_defaults() {
        let directory = assert_ok!(tempfile::tempdir());
        let path = directory.path().join("config.toml");
        assert_ok!(fs::write(&path, "[editor]\nblur_sigma = 11"));
        let defaults = assert_ok!(parse("test.toml", ""));

        assert_ok_eq!(
            load(Some(&path)),
            Settings {
                blur_sigma: 11,
                ..defaults
            }
        );
    }

    #[test]
    fn diagnostics_point_to_the_original_user_source_after_merging() {
        for (source, expected) in [
            ("# user settings\n[editor]\nblur_sigma = 3\n", "3"),
            ("editor.blur_sigma = 3", "3"),
            ("['editor']\n'blur_sigma' = 3", "3"),
            ("[editor]\nstroke_width = 'wide'", "'wide'"),
            ("[editor]\nstorke_width = 4", "storke_width"),
            ("editor = false", "false"),
        ] {
            let error = assert_err!(parse("user.toml", source));
            let label = assert_some!(assert_some!(error.labels()).next());
            let end = assert_some!(label.offset().checked_add(label.len()));
            assert_some_eq!(source.get(label.offset()..end), expected);

            let snippet =
                assert_ok!(assert_some!(error.source_code()).read_span(label.inner(), 0, 0));
            assert_some_eq!(snippet.name(), "user.toml");
        }
    }

    #[test]
    fn diagnostics_keep_the_original_error_causes() {
        let validation = assert_err!(parse("user.toml", "[editor]\nblur_sigma = 3"));
        assert!(assert_some!(std::error::Error::source(&validation)).is::<tvert_editor::Error>());

        let toml = assert_err!(parse("user.toml", "[editor]\nstroke_width = 'wide'"));
        assert!(assert_some!(std::error::Error::source(&toml)).is::<toml::de::Error>());

        let directory = assert_ok!(tempfile::tempdir());
        let missing = assert_err!(load_path(&directory.path().join("missing.toml"), true));
        assert!(assert_some!(std::error::Error::source(&missing)).is::<std::io::Error>());
    }
}
