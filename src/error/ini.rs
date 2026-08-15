use miette::{SourceOffset, SourceSpan};

/// This struct wraps an [`ini::ParseError`], allowing miette to pretty-print it with source information.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("malformed ini")]
pub struct IniError {
    /// The reason that the input could not be parsed, without the `line:column` prefix that [`ini::ParseError`]
    /// renders, as miette already points at the location itself.
    message: String,

    /// The input that caused the error.
    #[source_code]
    input: String,

    /// The location that the error occured at.
    #[label("{message}")]
    location: SourceSpan,
}

impl IniError {
    /// Converts an [`ini::ParseError`] into an [`IniError`] that can be rendered by Miette.
    pub fn from(input: impl Into<String>, cause: ini::ParseError) -> Self {
        let input = input.into();

        // The parser only reports where it gave up, not how much of the line it was unhappy with, so the label covers
        // the single character that it stopped at.
        let offset = SourceOffset::from_location(&input, cause.line, cause.col);

        Self {
            message: cause.msg.into_owned(),
            input,
            location: SourceSpan::new(offset, 1),
        }
    }
}
