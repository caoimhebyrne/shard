//! Derived from a miette example:
//! <https://github.com/zkat/miette/blob/e853bbf9bc78bbe0b225995de54a3108d77dcaf8/examples/serde_json.rs>

use miette::SourceOffset;

/// This struct wraps a [`yaml_serde::Error`], allowing miette to pretty-print it with source information.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("malformed yaml")]
pub struct YamlSerdeError {
    /// The cause of this error.
    cause: yaml_serde::Error,

    /// The input that caused the error.
    #[source_code]
    input: String,

    /// The location that the error occured at.
    #[label("{cause}")]
    source_offset: SourceOffset,
}

impl YamlSerdeError {
    /// Converts a [`yaml_serde::Error`] into a [`YamlSerdeError`] that can be rendered by Miette.
    pub fn from(input: impl Into<String>, cause: yaml_serde::Error) -> Self {
        let (line, column) = match cause.location() {
            Some(location) => (location.line(), location.column()),
            None => (0, 0),
        };

        let input = input.into();
        let source_offset = SourceOffset::from_location(&input, line, column);

        Self {
            cause,
            input,
            source_offset,
        }
    }
}
