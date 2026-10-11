use color_eyre::{Result, eyre::Context};
use serde::Deserialize;
use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};
use toml::Value;
use tvert_editor::Settings;

const DEFAULT_CONFIG: &str = include_str!("../../../configs/config.toml");

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
        return parse("");
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

fn load_path(path: &Path, required: bool) -> Result<Settings> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == ErrorKind::NotFound && !required => return parse(""),
        Err(error) => {
            return Err(error)
                .wrap_err_with(|| format!("failed to read configuration {}", path.display()));
        }
    };
    parse(&source).wrap_err_with(|| format!("invalid configuration {}", path.display()))
}

fn parse(source: &str) -> Result<Settings> {
    let mut defaults: Value =
        toml::from_str(DEFAULT_CONFIG).wrap_err("invalid embedded configuration TOML")?;
    let overrides = toml::from_str(source)?;
    merge(&mut defaults, overrides);
    let config: Config = defaults.try_into()?;
    Ok(config.editor.try_into()?)
}

fn merge(base: &mut Value, overrides: Value) {
    match (base, overrides) {
        (Value::Table(base), Value::Table(overrides)) => {
            for (key, value) in overrides {
                if let Some(existing) = base.get_mut(&key) {
                    merge(existing, value);
                } else {
                    base.insert(key, value);
                }
            }
        }
        (base, overrides) => *base = overrides,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok, assert_ok_eq};

    #[test]
    fn partial_configuration_keeps_unspecified_defaults() {
        let defaults = assert_ok!(parse(""));
        let settings = assert_ok!(parse("[editor]\nstroke_width = 7"));
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
            let _ = assert_err!(parse(source));
        }
    }

    #[test]
    fn missing_default_file_is_optional_but_explicit_file_is_required() {
        let directory = assert_ok!(tempfile::tempdir());
        let path = directory.path().join("missing.toml");
        let defaults = assert_ok!(parse(""));
        assert_ok_eq!(load_path(&path, false), defaults);
        let _ = assert_err!(load_path(&path, true));
    }

    #[test]
    fn user_file_overrides_embedded_defaults() {
        let directory = assert_ok!(tempfile::tempdir());
        let path = directory.path().join("config.toml");
        assert_ok!(fs::write(&path, "[editor]\nblur_sigma = 11"));
        let defaults = assert_ok!(parse(""));
        assert_ok_eq!(
            load(Some(&path)),
            Settings {
                blur_sigma: 11,
                ..defaults
            }
        );
    }
}
