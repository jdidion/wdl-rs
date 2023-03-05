mod pest;
mod tree_sitter;

pub use crate::parsers::pest::PestParser;
pub use crate::parsers::tree_sitter::TreeSitterParser;

use crate::model::{Document, DocumentSource};
use error_stack::{IntoReport, Result, ResultExt};
use std::{fs, path::Path};
use thiserror::Error;

/// Syntax errors that may be returned when creating model elements.
#[derive(Error, Debug)]
pub enum WdlParserError {
    #[error("error creating WDL parser")]
    Internal,
    #[error("error reading WDL document from {0}")]
    IO(DocumentSource),
    #[error("error parsing WDL document from {0}")]
    Syntax(DocumentSource),
    #[error("error building WDL model from {0}")]
    Model(DocumentSource),
}

pub trait WdlParser {
    fn parse_text<Text: AsRef<str>>(
        &mut self,
        text: Text,
        source: DocumentSource,
    ) -> Result<Document, WdlParserError>;

    fn parse_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Document, WdlParserError> {
        let path = path.as_ref().to_owned();
        let source = DocumentSource::File(path.clone());
        let text = fs::read_to_string(&path)
            .into_report()
            .change_context(WdlParserError::IO(source.clone()))?;
        self.parse_text(text, source)
    }
}
