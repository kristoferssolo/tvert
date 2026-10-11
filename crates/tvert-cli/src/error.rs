use miette::{Diagnostic, NamedSource, SourceSpan};
use std::{io, path::PathBuf, sync::Arc};
use thiserror::Error;
use toml::de::{DeTable, DeValue};
use tvert_editor::Error as EditorError;

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

    #[error("invalid editor configuration in {}", .source_code.name())]
    #[diagnostic(code(tvert::config::editor))]
    Editor {
        #[source_code]
        source_code: Arc<NamedSource<String>>,
        #[source]
        cause: EditorError,
    },
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

    pub fn setting(name: &str, source: &str, document: &DeTable<'_>, cause: EditorError) -> Self {
        let source_code = Arc::new(NamedSource::new(name, source.to_owned()));

        match &cause {
            EditorError::InvalidSetting { name, min, max, .. } => Self::InvalidSetting {
                setting: name,
                min: *min,
                max: *max,
                source_code,
                span: setting_span(document, name),
                cause,
            },
            _ => Self::Editor { source_code, cause },
        }
    }
}

fn setting_span(document: &DeTable<'_>, name: &str) -> Option<SourceSpan> {
    document
        .get("editor")
        .and_then(|editor| match editor.get_ref() {
            DeValue::Table(table) => table.get(name),
            _ => None,
        })
        .map(|value| value.span().into())
}
