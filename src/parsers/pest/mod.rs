mod declarations;
mod document;
mod expressions;
mod meta;
mod primitives;
mod task;
mod workflow;

use crate::{
    ast::{Document, DocumentSource, Location, Node},
    parsers::WdlParser,
};
use anyhow::{bail, Context, Error, Result};
use pest::{
    iterators::{Pair, Pairs},
    Parser, Position,
};
use pest_derive;
use std::str::FromStr;

// TODO: create attribute macro for implementations of `TryFrom<Pair>` that check the pair's
// rule against the list of valid rules specified as a parameter to the attribute.

#[derive(pest_derive::Parser)]
#[grammar = "parsers/pest/wdl.pest"]
struct PestParser;

impl WdlParser for PestParser {
    fn parse_text<Text: AsRef<str>>(
        &mut self,
        text: Text,
        source: DocumentSource,
    ) -> Result<Document> {
        let mut root_pair = Self::parse(Rule::document, text.as_ref())?;
        if let Some(doc_pair) = root_pair.next() {
            let mut doc: Document = doc_pair.try_into()?;
            doc.source = source;
            doc.validate()?;
            Ok(doc)
        } else {
            bail!("Document is empty")
        }
    }
}

impl<'a> From<Position<'a>> for Location {
    fn from(value: Position<'a>) -> Self {
        let (line, column) = value.line_col();
        Self {
            line,
            column,
            offset: value.pos(),
        }
    }
}

trait PairExt<'a> {
    fn as_string(&self) -> String;

    fn try_into_string_node(self) -> Result<Node<String>>;

    fn try_str_into_node<T: FromStr<Err = Error>>(self) -> Result<Node<T>>;

    fn first_inner(self) -> Result<Pair<'a, Rule>>;

    fn first_inner_string(self) -> Result<String>;

    fn first_inner_node<T: TryFrom<Pair<'a, Rule>, Error = Error>>(self) -> Result<Node<T>>;

    fn first_inner_boxed_node<T: TryFrom<Pair<'a, Rule>, Error = Error>>(
        self,
    ) -> Result<Box<Node<T>>>;

    fn first_inner_string_node(self) -> Result<Node<String>>;
}

impl<'a> PairExt<'a> for Pair<'a, Rule> {
    fn as_string(&self) -> String {
        self.as_str().to_owned()
    }

    fn try_into_string_node(self) -> Result<Node<String>> {
        let span = self.as_span();
        let element = self.as_string();
        Ok(Node {
            element,
            start: span.start_pos().into(),
            end: span.end_pos().into(),
        })
    }

    fn try_str_into_node<T: FromStr<Err = Error>>(self) -> Result<Node<T>> {
        let span = self.as_span();
        let element = T::from_str(self.as_str())?;
        Ok(Node {
            element,
            start: span.start_pos().into(),
            end: span.end_pos().into(),
        })
    }

    fn first_inner(self) -> Result<Pair<'a, Rule>> {
        self.into_inner()
            .next()
            .context("Expected pair to have at least one inner node")
    }

    fn first_inner_string(self) -> Result<String> {
        let pair = self.first_inner()?;
        Ok(pair.as_string())
    }

    fn first_inner_node<T: TryFrom<Pair<'a, Rule>, Error = Error>>(self) -> Result<Node<T>> {
        let inner = self.first_inner()?;
        inner.try_into()
    }

    fn first_inner_boxed_node<T: TryFrom<Pair<'a, Rule>, Error = Error>>(
        self,
    ) -> Result<Box<Node<T>>> {
        let inner = self.first_inner()?;
        Ok(Box::new(inner.try_into()?))
    }

    fn first_inner_string_node(self) -> Result<Node<String>> {
        let pair = self.first_inner()?;
        pair.try_into_string_node()
    }
}

trait PairsExt<'a> {
    fn next_pair(&mut self) -> Result<Pair<'a, Rule>>;

    fn collect_nodes<T: TryFrom<Pair<'a, Rule>, Error = Error>>(self) -> Result<Vec<Node<T>>>;

    fn collect_string_nodes(self) -> Result<Vec<Node<String>>>;

    fn next_node<T: TryFrom<Pair<'a, Rule>, Error = Error>>(&mut self) -> Result<Node<T>>;

    fn next_boxed_node<T: TryFrom<Pair<'a, Rule>, Error = Error>>(
        &mut self,
    ) -> Result<Box<Node<T>>>;

    fn next_string_node(&mut self) -> Result<Node<String>>;

    fn next_str_into_node<T: FromStr<Err = Error>>(&mut self) -> Result<Node<T>>;
}

impl<'a> PairsExt<'a> for Pairs<'a, Rule> {
    fn next_pair(&mut self) -> Result<Pair<'a, Rule>> {
        if let Some(pair) = self.next() {
            Ok(pair)
        } else {
            bail!("Expected next node")
        }
    }

    fn collect_nodes<T: TryFrom<Pair<'a, Rule>, Error = Error>>(self) -> Result<Vec<Node<T>>> {
        self.map(|pair| pair.try_into()).collect()
    }

    fn collect_string_nodes(self) -> Result<Vec<Node<String>>> {
        self.map(|pair| pair.try_into_string_node()).collect()
    }

    fn next_node<T: TryFrom<Pair<'a, Rule>, Error = Error>>(&mut self) -> Result<Node<T>> {
        let pair = self.next_pair()?;
        pair.try_into()
    }

    fn next_boxed_node<T: TryFrom<Pair<'a, Rule>, Error = Error>>(
        &mut self,
    ) -> Result<Box<Node<T>>> {
        let pair = self.next_pair()?;
        Ok(Box::new(pair.try_into()?))
    }

    fn next_string_node(&mut self) -> Result<Node<String>> {
        let pair = self.next_pair()?;
        pair.try_into_string_node()
    }

    fn next_str_into_node<T: FromStr<Err = Error>>(&mut self) -> Result<Node<T>> {
        let pair = self.next_pair()?;
        pair.try_str_into_node()
    }
}

impl<'a, T: TryFrom<Pair<'a, Rule>, Error = Error>> TryFrom<Pair<'a, Rule>> for Node<T> {
    type Error = Error;

    fn try_from(pair: Pair<'a, Rule>) -> Result<Self> {
        let span = pair.as_span();
        Ok(Self {
            element: pair.try_into()?,
            start: span.start_pos().into(),
            end: span.end_pos().into(),
        })
    }
}
