mod declarations;
mod document;
mod expressions;
mod meta;
mod node;
mod primitives;
mod task;
mod workflow;

use crate::{
    model::{Comments, Document, DocumentSource, Position, Span},
    parsers::{pest::node::PestNode, WdlParser, WdlParserError},
};
use error_stack::{IntoReport, Result, ResultExt};
use pest::error::{Error as PestError, InputLocation, LineColLocation};
use pest_wdl_1 as wdl;
use pest_wdl_1::Rule;
use std::{cell::RefCell, rc::Rc};

pub struct PestParser;

impl PestParser {
    pub fn new() -> Self {
        PestParser {}
    }
}

impl WdlParser for PestParser {
    fn parse_text<Text: AsRef<str>>(
        &mut self,
        text: Text,
        source: DocumentSource,
    ) -> Result<Document, WdlParserError> {
        let text: &str = text.as_ref();
        let root_pair = wdl::parse_document(text)
            .into_report()
            .change_context(WdlParserError::Syntax(source.clone()))?;
        let root_node = PestNode::new(root_pair, Rc::new(RefCell::new(Comments::default())));
        let mut doc: Document = root_node
            .try_into()
            .change_context(WdlParserError::Model(source.clone()))?;
        doc.source = source.clone();
        doc.validate()
            .change_context(WdlParserError::Model(source))?;
        Ok(doc)
    }
}

impl<'a> From<pest::Position<'a>> for Position {
    fn from(value: pest::Position<'a>) -> Self {
        let (line, column) = value.line_col();
        Self {
            line: line - 1,
            column: column - 1,
            offset: value.pos(),
        }
    }
}

impl<'a> From<&pest::Span<'a>> for Span {
    fn from(value: &pest::Span<'a>) -> Self {
        Self {
            start: value.start_pos().into(),
            end: value.end_pos().into(),
        }
    }
}

impl From<PestError<Rule>> for Span {
    fn from(error: PestError<Rule>) -> Self {
        let ((start_line, start_column), (end_line, end_column)) = match error.line_col {
            LineColLocation::Pos(pos) => (pos, pos),
            LineColLocation::Span(start, end) => (start, end),
        };
        let (start_offset, end_offset) = match error.location {
            InputLocation::Pos(pos) => (pos, pos),
            InputLocation::Span(span) => span,
        };
        Self {
            start: Position {
                line: start_line - 1,
                column: start_column - 1,
                offset: start_offset - 1,
            },
            end: Position {
                line: end_line - 1,
                column: end_column - 1,
                offset: end_offset - 1,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        model::tests,
        parsers::{pest::PestParser, WdlParser, WdlParserError},
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
        let mut parser = PestParser::new();
        let wdl_file = test_path("comprehensive.wdl");
        let doc = parser.parse_file(wdl_file)?;
        tests::test_comprehensive(doc);
        Ok(())
    }

    #[test]
    fn test_expressions() -> Result<(), WdlParserError> {
        let mut parser = PestParser::new();
        let wdl_file = test_path("expressions.wdl");
        let _ = parser.parse_file(wdl_file)?;
        Ok(())
    }
}
