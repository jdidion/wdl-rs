pub mod pest;
pub mod tree_sitter;

use crate::model::{Document, DocumentSource};
use anyhow::Result;
use std::{fs, path::Path};

pub trait WdlParser {
    fn parse_text<Text: AsRef<str>>(
        &mut self,
        text: Text,
        source: DocumentSource,
    ) -> Result<Document>;

    fn parse_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Document> {
        let text = fs::read_to_string(&path)?;
        let source = DocumentSource::File(path.as_ref().to_owned());
        self.parse_text(text, source)
    }
}
