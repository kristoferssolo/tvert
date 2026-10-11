use color_eyre::{Result, eyre::Context};
use serde::Deserialize;
use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};
use tvert_editor::Settings;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Config {
    editor: EditorConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct EditorConfig {
    stroke_width: Option<u16>,
    pixelation_block_size: Option<u16>,
    blur_sigma: Option<u16>,
}

impl TryFrom<EditorConfig> for Settings {
    type Error = tvert_editor::Error;

    fn try_from(config: EditorConfig) -> Result<Self, Self::Error> {
        let defaults = Self::default();
        let settings = Self {
            stroke_width: config.stroke_width.unwrap_or(defaults.stroke_width),
            pixelation_block_size: config
                .pixelation_block_size
                .unwrap_or(defaults.pixelation_block_size),
            blur_sigma: config.blur_sigma.unwrap_or(defaults.blur_sigma),
        };
        settings.validate()?;
        Ok(settings)
    }
}

pub fn load(explicit: Option<&Path>) -> Result<Settings> {
    let Some(path) = explicit.map(Path::to_path_buf).or_else(default_path) else {
        return Ok(Settings::default());
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
        Err(error) if error.kind() == ErrorKind::NotFound && !required => {
            return Ok(Settings::default());
        }
        Err(error) => {
            return Err(error)
                .wrap_err_with(|| format!("failed to read configuration {}", path.display()));
        }
    };
    parse(&source).wrap_err_with(|| format!("invalid configuration {}", path.display()))
}

fn parse(source: &str) -> Result<Settings> {
    let config: Config = toml::from_str(source)?;
    Ok(config.editor.try_into()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok, assert_ok_eq};

    #[test]
    fn partial_configuration_keeps_unspecified_defaults() {
        let settings = assert_ok!(parse("[editor]\nstroke_width = 7"));
        assert_eq!(
            settings,
            Settings {
                stroke_width: 7,
                ..Settings::default()
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
        ] {
            let _ = assert_err!(parse(source));
        }
    }

    #[test]
    fn missing_default_file_is_optional_but_explicit_file_is_required() {
        let directory = assert_ok!(tempfile::tempdir());
        let path = directory.path().join("missing.toml");
        assert_ok_eq!(load_path(&path, false), Settings::default());
        let _ = assert_err!(load_path(&path, true));
    }
}
