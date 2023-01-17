mod declarations;
mod document;
mod expressions;
mod meta;
mod primitives;
mod task;
mod workflow;

use crate::{
    model::{Comments, Ctx, Document, DocumentSource, Location},
    parsers::WdlParser,
};
use anyhow::{bail, Context, Error, Result};
use pest::{
    iterators::{Pair, Pairs},
    Parser, Position,
};
use pest_derive;
use std::{cell::RefCell, rc::Rc, str::FromStr};

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

impl<'a, T: TryFrom<PestNode<'a>, Error = Error>> TryFrom<PestNode<'a>> for Ctx<T> {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let (start, end) = node.as_span();
        Ok(Self {
            element: node.try_into()?,
            start,
            end,
        })
    }
}

#[derive(Debug)]
pub struct PestNode<'a> {
    pair: Pair<'a, Rule>,
    comments: Rc<RefCell<Comments>>,
}

impl<'a> PestNode<'a> {
    pub fn into_inner(self) -> PestNodes<'a> {
        PestNodes {
            pairs: self.pair.into_inner(),
            comments: self.comments.clone(),
        }
    }

    pub fn as_rule(&self) -> Rule {
        self.pair.as_rule()
    }

    pub fn as_span(&self) -> (Location, Location) {
        let span = self.pair.as_span();
        (span.start_pos().into(), span.end_pos().into())
    }

    pub fn as_str(&self) -> &'a str {
        self.pair.as_str()
    }

    pub fn as_string(&self) -> String {
        self.pair.as_str().to_owned()
    }

    pub fn try_str_into_ctx<T: FromStr<Err = Error>>(self) -> Result<Ctx<T>> {
        let span = self.pair.as_span();
        let element = T::from_str(self.pair.as_str())?;
        Ok(Ctx {
            element,
            start: span.start_pos().into(),
            end: span.end_pos().into(),
        })
    }

    pub fn first_inner(self) -> Result<PestNode<'a>> {
        self.into_inner().next_node()
    }

    pub fn first_inner_string(self) -> Result<String> {
        let pair = self.into_inner().next_node()?;
        Ok(pair.as_str().to_owned())
    }

    pub fn first_inner_ctx<T: TryFrom<PestNode<'a>, Error = Error>>(self) -> Result<Ctx<T>> {
        let inner = self.first_inner()?;
        inner.try_into()
    }

    pub fn first_inner_boxed_ctx<T: TryFrom<PestNode<'a>, Error = Error>>(
        self,
    ) -> Result<Box<Ctx<T>>> {
        let inner = self.first_inner()?;
        Ok(Box::new(inner.try_into()?))
    }

    pub fn first_inner_string_ctx(self) -> Result<Ctx<String>> {
        let pair = self.first_inner()?;
        pair.try_into()
    }
}

impl<'a> TryFrom<PestNode<'a>> for &'a str {
    type Error = Error;

    fn try_from(value: PestNode<'a>) -> Result<Self> {
        Ok(value.pair.as_str())
    }
}

impl<'a> TryFrom<PestNode<'a>> for String {
    type Error = Error;

    fn try_from(value: PestNode<'a>) -> Result<Self> {
        Ok(value.pair.as_str().to_owned())
    }
}

pub struct PestNodes<'a> {
    pairs: Pairs<'a, Rule>,
    comments: Rc<RefCell<Comments>>,
}

impl<'a> PestNodes<'a> {
    fn get_next_pair(&mut self) -> Option<Pair<'a, Rule>> {
        while let Some(pair) = self.pairs.next() {
            if pair.as_rule() == Rule::COMMENT {
                let span = pair.as_span();
                let start: Location = span.start_pos().into();
                let end: Location = span.end_pos().into();
                let mut comments = self.comments.borrow_mut();
                comments
                    .add(
                        start.line,
                        Ctx {
                            element: pair.as_str().to_owned(),
                            start,
                            end,
                        },
                    )
                    .ok();
            } else {
                return Some(pair);
            }
        }
        None
    }

    pub fn next_node(&mut self) -> Result<PestNode<'a>> {
        self.next().context("Expected next pair")
    }

    pub fn peek_rule(&self) -> Option<Rule> {
        self.pairs.peek().map(|pair| pair.as_rule())
    }

    pub fn collect_ctxs<T: TryFrom<PestNode<'a>, Error = Error>>(self) -> Result<Vec<Ctx<T>>> {
        self.map(|node| node.try_into()).collect()
    }

    pub fn collect_string_ctxs(self) -> Result<Vec<Ctx<String>>> {
        self.map(|node| node.try_into()).collect()
    }

    pub fn next_ctx<T: TryFrom<PestNode<'a>, Error = Error>>(&mut self) -> Result<Ctx<T>> {
        let node = self.next_node()?;
        node.try_into()
    }

    pub fn next_boxed_ctx<T: TryFrom<PestNode<'a>, Error = Error>>(
        &mut self,
    ) -> Result<Box<Ctx<T>>> {
        let node = self.next_node()?;
        Ok(Box::new(node.try_into()?))
    }

    pub fn next_string_ctx(&mut self) -> Result<Ctx<String>> {
        let node = self.next_node()?;
        node.try_into()
    }

    pub fn next_str_into_ctx<T: FromStr<Err = Error>>(&mut self) -> Result<Ctx<T>> {
        let node = self.next_node()?;
        node.try_str_into_ctx()
    }
}

impl<'a> Iterator for PestNodes<'a> {
    type Item = PestNode<'a>;

    fn next(&mut self) -> Option<PestNode<'a>> {
        self.get_next_pair().map(|pair| PestNode {
            pair,
            comments: self.comments.clone(),
        })
    }
}

impl<'a> Drop for PestNodes<'a> {
    fn drop(&mut self) {
        // drains the iterator to ensure all comment pairs are added to `self.comments`
        for _ in self {}
    }
}

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
        let comments = Rc::new(RefCell::new(Comments::new()));
        if let Some(pair) = root_pair.next() {
            let node = PestNode {
                pair,
                comments: comments.clone(),
            };
            let mut doc: Document = node.try_into()?;
            doc.source = source;
            doc.validate()?;
            Ok(doc)
        } else {
            bail!("Document is empty")
        }
    }
}
