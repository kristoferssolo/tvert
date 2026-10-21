use miette::{Diagnostic, LabeledSpan, NamedSource, SourceSpan};
use std::{io, path::PathBuf, sync::Arc};
use thiserror::Error;
use toml::de::{DeTable, DeValue};
use tvert_editor::{Error as EditorError, ExportAction};

/// A configuration loading or validation failure
#[derive(Debug, Error, Diagnostic)]
pub enum ConfigError {
    #[error("failed to read configuration {}", .path.display())]
    #[diagnostic(
        code(tvert::config::read),
        help("check that the configuration path exists and is readable")
    )]
    Read {
        path: PathBuf,
        #[source]
        cause: io::Error,
    },

    #[error("invalid configuration in {}", .source_code.name())]
    #[diagnostic(code(tvert::config::toml))]
    Toml {
        #[source_code]
        source_code: Arc<NamedSource<String>>,
        #[label("here")]
        span: Option<SourceSpan>,
        #[source]
        cause: toml::de::Error,
    },

    #[error("invalid value for editor.{setting} in {}", .source_code.name())]
    #[diagnostic(
        code(tvert::config::value),
        help("set editor.{setting} to an integer between {min} and {max}")
    )]
    InvalidSetting {
        setting: &'static str,
        min: u16,
        max: u16,
        #[source_code]
        source_code: Arc<NamedSource<String>>,
        #[label("expected a value in {min}..={max}")]
        span: Option<SourceSpan>,
        #[source]
        cause: EditorError,
    },

    #[error(
        "editor.keybindings.{first} and editor.keybindings.{second} use the same shortcut in {}",
        .source_code.name()
    )]
    #[diagnostic(
        code(tvert::config::keybinding),
        help("give each export action a different shortcut")
    )]
    DuplicateKeyBinding {
        first: &'static str,
        second: &'static str,
        #[source_code]
        source_code: Arc<NamedSource<String>>,
        /// The conflicting values set in this source
        #[label(collection)]
        labels: Vec<LabeledSpan>,
        #[source]
        cause: EditorError,
    },

    #[error(
        "editor.keybindings.{setting} uses a reserved shortcut in {}",
        .source_code.name()
    )]
    #[diagnostic(
        code(tvert::config::keybinding),
        help("choose a shortcut that no history, tool, or selection shortcut uses")
    )]
    ReservedKeyBinding {
        setting: &'static str,
        #[source_code]
        source_code: Arc<NamedSource<String>>,
        #[label(collection)]
        label: Option<LabeledSpan>,
        #[source]
        cause: EditorError,
    },

    #[error("invalid editor configuration in {}", .source_code.name())]
    #[diagnostic(code(tvert::config::editor))]
    Editor {
        #[source_code]
        source_code: Arc<NamedSource<String>>,
        #[source]
        cause: EditorError,
    },

    #[error(transparent)]
    #[diagnostic(transparent)]
    Deserialize(Box<DeserializeError>),
}

#[derive(Debug, Error, Diagnostic)]
#[error("invalid value at {path} in {}: {}", .source_code.name(), .cause.message())]
#[diagnostic(code(tvert::config::value))]
pub struct DeserializeError {
    path: String,
    #[source_code]
    source_code: Arc<NamedSource<String>>,
    #[label("{}", .cause.message())]
    span: Option<SourceSpan>,
    #[source]
    cause: toml::de::Error,
}

impl ConfigError {
    pub fn toml(name: &str, source: &str, mut cause: toml::de::Error) -> Self {
        let span = cause.span().map(SourceSpan::from);

        // Miette renders the excerpt; retain the parser error without its duplicate excerpt
        cause.set_input(None);

        Self::Toml {
            source_code: Arc::new(NamedSource::new(name, source.to_owned())),
            span,
            cause,
        }
    }

    /// Locates an editor validation error among the settings `explicit` reads
    /// from `source`.
    ///
    /// Conflicts with inherited defaults label only the explicit setting.
    pub fn setting(name: &str, source: &str, explicit: &DeTable<'_>, cause: EditorError) -> Self {
        let source_code = Arc::new(NamedSource::new(name, source.to_owned()));

        match cause {
            EditorError::InvalidSetting { name, min, max, .. } => Self::InvalidSetting {
                setting: name,
                min,
                max,
                source_code,
                span: value_span(explicit, &["editor", name]),
                cause,
            },
            EditorError::DuplicateKeyBinding {
                binding,
                first,
                second,
            } => Self::DuplicateKeyBinding {
                first: first.setting_name(),
                second: second.setting_name(),
                source_code,
                labels: [(first, second), (second, first)]
                    .into_iter()
                    .filter_map(|(action, other)| {
                        let span = keybinding_span(explicit, action)?;
                        let inherited = if keybinding_span(explicit, other).is_some() {
                            ""
                        } else {
                            "the inherited "
                        };
                        let label = format!(
                            "{binding} is also used by {inherited}editor.keybindings.{}",
                            other.setting_name()
                        );
                        Some(LabeledSpan::new_with_span(Some(label), span))
                    })
                    .collect(),
                cause,
            },
            EditorError::ReservedKeyBinding {
                action,
                binding,
                ref reserved,
            } => Self::ReservedKeyBinding {
                setting: action.setting_name(),
                source_code,
                label: keybinding_span(explicit, action).map(|span| {
                    let label = format!("{binding} is reserved for {reserved}");
                    LabeledSpan::new_with_span(Some(label), span)
                }),
                cause,
            },
            cause => Self::Editor { source_code, cause },
        }
    }

    pub fn deserialize(
        name: &str,
        source: &str,
        error: serde_path_to_error::Error<toml::de::Error>,
    ) -> Self {
        let path = if error.path().iter().next().is_none() {
            "configuration root".to_owned()
        } else {
            error.path().to_string()
        };

        let mut cause = error.into_inner();
        let span = cause.span().map(SourceSpan::from);
        cause.set_input(None);

        Self::Deserialize(Box::new(DeserializeError {
            path,
            source_code: Arc::new(NamedSource::new(name, source.to_owned())),
            span,
            cause,
        }))
    }
}

fn keybinding_span(document: &DeTable<'_>, action: ExportAction) -> Option<SourceSpan> {
    value_span(document, &["editor", "keybindings", action.setting_name()])
}

fn value_span(document: &DeTable<'_>, path: &[&str]) -> Option<SourceSpan> {
    let (setting, tables) = path.split_last()?;
    let mut table = document;
    for key in tables {
        let DeValue::Table(child) = table.get(*key)?.get_ref() else {
            return None;
        };
        table = child;
    }
    table.get(*setting).map(|value| value.span().into())
}
