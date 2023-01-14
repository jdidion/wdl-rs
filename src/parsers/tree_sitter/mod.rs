mod declarations;
mod document;
mod expressions;
mod meta;
mod primitives;
mod syntax;
mod task;
mod workflow;

use crate::{
    ast::{Document, DocumentSource, Node},
    parsers::WdlParser,
};
use anyhow::{anyhow, bail, ensure, Context, Error, Result};
use std::{collections::HashSet, ops::Range, str::FromStr};
use tree_sitter as ts;
use tree_sitter_wdl_1::language as wdl1_language;

//use smartstring::{LazyCompact, SmartString};
//pub type Str = SmartString<LazyCompact>;
/// TODO: look into different set/hashing implementations, e.g. FNV
//pub type StrSet = HashSet<Str>;

// TODO: need to handle these cases for all nodes:
// if node.is_error() {}
// if node.is_missing() {}
// if node.is_extra() {}

trait FromTSNode {
    fn nodes() -> HashSet<&'static str>;
}

#[derive(Debug)]
pub struct TSNode<'a> {
    node: ts::Node<'a>,
    text: &'a [u8],
}

impl<'a> TSNode<'a> {
    fn new(node: ts::Node<'a>, text: &'a [u8]) -> Self {
        Self { node, text }
    }

    pub fn kind(&self) -> &str {
        self.node.kind()
    }

    pub fn span(&self) -> Range<usize> {
        self.node.byte_range()
    }

    pub fn try_as_str(&self) -> Result<&'a str> {
        Ok(self.node.utf8_text(self.text)?)
    }

    pub fn try_as_string(&self) -> Result<String> {
        let s = self.node.utf8_text(self.text)?;
        Ok(s.to_owned())
    }

    pub fn children<T: TryFrom<TSNode<'a>, Error = Error>>(&self) -> Result<Vec<T>> {
        let cursor = &mut self.node.walk();
        self.node
            .named_children(cursor)
            .map(|node| TSNode::new(node, self.text))
            .map(|node| node.try_into())
            .collect()
    }

    pub fn child_nodes<T: TryFrom<TSNode<'a>, Error = Error>>(&self) -> Result<Vec<Node<T>>> {
        self.children()
    }

    pub fn get_field(&mut self, name: &str) -> Option<TSNode<'a>> {
        self.node.child_by_field_name(name).map(|node| TSNode {
            node,
            text: self.text,
        })
    }

    pub fn field(&mut self, name: &str) -> Result<TSNode<'a>> {
        if let Some(node) = self.get_field(name) {
            Ok(node)
        } else {
            bail!("Missing field {} in node {:?}", name, self.node)
        }
    }

    pub fn field_node<T: TryFrom<TSNode<'a>, Error = Error>>(
        &mut self,
        name: &str,
    ) -> Result<Node<T>> {
        let field_node = self.field(name)?;
        field_node.try_into()
    }

    pub fn get_field_node<T: TryFrom<TSNode<'a>, Error = Error>>(
        &mut self,
        name: &str,
    ) -> Result<Option<Node<T>>> {
        let field_node = self.get_field(name);
        field_node.map(|node| node.try_into()).transpose()
    }

    pub fn field_boxed_node<T: TryFrom<TSNode<'a>, Error = Error>>(
        &mut self,
        name: &str,
    ) -> Result<Box<Node<T>>> {
        Ok(Box::new(self.field_node(name)?))
    }

    pub fn field_string(&mut self, name: &str) -> Result<String> {
        let ts_node = self.field(name)?;
        ts_node.try_as_str().map(|s| s.to_owned())
    }

    pub fn field_string_node(&mut self, name: &str) -> Result<Node<String>> {
        let ts_node = self.field(name)?;
        ts_node.try_into()
    }

    pub fn field_string_into_node<T: FromStr<Err = Error>>(
        &mut self,
        name: &str,
    ) -> Result<Node<T>> {
        let ts_node = self.field(name)?;
        let span = ts_node.span();
        let element_str = ts_node.try_as_str()?;
        let element = T::from_str(element_str)?;
        Ok(Node { element, span })
    }

    pub fn field_child_nodes<T: TryFrom<TSNode<'a>, Error = Error>>(
        &mut self,
        name: &str,
    ) -> Result<Vec<Node<T>>> {
        let field_node = self.field(name)?;
        field_node.children()
    }

    pub fn get_field_child_nodes<T: TryFrom<TSNode<'a>, Error = Error>>(
        &mut self,
        name: &str,
    ) -> Result<Vec<Node<T>>> {
        if let Some(field_node) = self.get_field(name) {
            field_node.children()
        } else {
            Ok(Vec::new())
        }
    }
}

impl<'a> TryFrom<TSNode<'a>> for &'a str {
    type Error = Error;

    fn try_from(value: TSNode<'a>) -> Result<&'a str> {
        Ok(value.try_as_str()?)
    }
}

impl<'a> TryFrom<TSNode<'a>> for String {
    type Error = Error;

    fn try_from(value: TSNode<'a>) -> Result<Self> {
        let s = value.try_as_str()?;
        Ok(s.to_owned())
    }
}

impl<'a, T: TryFrom<TSNode<'a>, Error = Error>> TryFrom<TSNode<'a>> for Node<T> {
    type Error = Error;

    fn try_from(value: TSNode<'a>) -> Result<Self> {
        let span = value.span();
        Ok(Self {
            element: value.try_into()?,
            span,
        })
    }
}

pub struct TreeSitterParser {
    parser: ts::Parser,
}

impl TreeSitterParser {
    pub fn new() -> Result<Self> {
        let mut parser = ts::Parser::new();
        parser
            .set_language(wdl1_language())
            .context("Failed to set parser language to tree_sitter_wdl_1::language")?;
        Ok(Self { parser })
    }
}

impl WdlParser for TreeSitterParser {
    fn parse_text<Text: AsRef<str>>(
        &mut self,
        text: Text,
        source: DocumentSource,
    ) -> Result<Document> {
        let text = text.as_ref();
        let tree = self
            .parser
            .parse(text, None)
            .ok_or(anyhow!("Failed to parse WDL document from {:?}", source))?;
        let root = TSNode {
            node: tree.root_node(),
            text: text.as_bytes(),
        };
        ensure!(
            root.kind() == syntax::DOCUMENT,
            "Expected root node to be document, not {:?}",
            root
        );
        let mut doc: Document = root.try_into()?;
        doc.source = source;
        doc.validate()?;
        Ok(doc)
    }
}
