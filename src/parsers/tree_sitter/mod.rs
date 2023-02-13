mod declarations;
mod document;
mod expressions;
mod meta;
mod node;
mod primitives;
mod syntax;
mod task;
mod workflow;

use crate::{
    model::{Comments, Document, DocumentSource, Position, Span},
    parsers::{tree_sitter::node::TSNode, WdlParser, WdlParserError},
};
use error_stack::{IntoReport, Result, ResultExt};
use std::{cell::RefCell, rc::Rc};
use tree_sitter as ts;
use tree_sitter_wdl_1;

pub struct TreeSitterParser(ts::Parser);

impl TreeSitterParser {
    pub fn new() -> Result<Self, WdlParserError> {
        Ok(Self(
            tree_sitter_wdl_1::parser()
                .into_report()
                .change_context(WdlParserError::Internal)?,
        ))
    }
}

impl WdlParser for TreeSitterParser {
    fn parse_text<Text: AsRef<str>>(
        &mut self,
        text: Text,
        source: DocumentSource,
    ) -> Result<Document, WdlParserError> {
        let text = text.as_ref();
        let tree = self
            .0
            .parse(text, None)
            .ok_or(WdlParserError::Syntax(source.clone()))?;
        let root = TSNode::from_cursor(
            Rc::new(RefCell::new(tree.walk())),
            text.as_bytes(),
            Rc::new(RefCell::new(Comments::default())),
        );
        let mut doc: Document = root
            .try_into()
            .change_context(WdlParserError::Model(source.clone()))?;
        doc.source = source.clone();
        doc.validate()
            .change_context(WdlParserError::Model(source))?;
        Ok(doc)
    }
}

impl<'a> From<&ts::Node<'a>> for Span {
    fn from(node: &ts::Node<'a>) -> Self {
        let start = node.start_position();
        let end = node.end_position();
        let span = node.byte_range();
        Span {
            start: Position {
                line: start.row,
                column: start.column,
                offset: span.start,
            },
            end: Position {
                line: end.row,
                column: end.column,
                offset: span.end,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        model::VersionIdentifier,
        parsers::{tree_sitter::TreeSitterParser, WdlParser, WdlParserError},
    };
    use error_stack::Result;
    use std::path::PathBuf;

    fn test_path(filename: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("test")
            .join(filename)
    }

    #[test]
    fn test_comprehensive() -> Result<(), WdlParserError> {
        let mut parser = TreeSitterParser::new()?;
        let wdl_file = test_path("comprehensive.wdl");
        let doc = parser.parse_file(wdl_file)?;
        assert_eq!(*(*doc.version).identifier, VersionIdentifier::V1_1);
        Ok(())
    }
}
