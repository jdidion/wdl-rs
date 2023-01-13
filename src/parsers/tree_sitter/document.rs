use crate::{
    ast::{
        Alias, Document, DocumentElement, DocumentSource, Import, Namespace, Node, Struct, Task,
        Version, Workflow,
    },
    parsers::tree_sitter::{syntax, TSNode},
};
use anyhow::{bail, Error, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<TSNode<'a>> for Version {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            identifier: node.field_string_into_node(syntax::IDENTIFIER)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Namespace {
    type Error = Error;

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        Ok(Self::Explicit(Node::try_from(node)?))
    }
}

impl<'a> TryFrom<TSNode<'a>> for Alias {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        let from = node.field_node(syntax::FROM)?;
        let to = node.field_node(syntax::TO)?;
        Ok(Self { from, to })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Import {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        let uri = node.field_node(syntax::URI)?;
        let namespace = if let Some(node) = node.get_field(syntax::NAMESPACE) {
            Namespace::try_from(node)?
        } else {
            Namespace::try_from_uri(&uri.element)?
        };
        let aliases = node.get_field_child_nodes(syntax::ALIASES)?;
        Ok(Self {
            uri,
            namespace,
            aliases,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Struct {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        let name = node.field_node(syntax::NAME)?;
        let fields = node.field_child_nodes(syntax::FIELDS)?;
        Ok(Self { name, fields })
    }
}

impl<'a> TryFrom<TSNode<'a>> for DocumentElement {
    type Error = Error;

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        let element = match node.kind() {
            syntax::IMPORT => Self::Import(Import::try_from(node)?),
            syntax::STRUCT => Self::Struct(Struct::try_from(node)?),
            syntax::TASK => Self::Task(Task::try_from(node)?),
            syntax::WORKFLOW => Self::Workflow(Workflow::try_from(node)?),
            other => bail!("Invalid node {}", other),
        };
        Ok(element)
    }
}

impl<'a> TryFrom<TSNode<'a>> for Document {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        let version = node.field_node(syntax::VERSION)?;
        let body = node.field_child_nodes(syntax::BODY)?;
        let doc = Self {
            source: DocumentSource::default(),
            version,
            body,
        };
        doc.validate()?;
        Ok(doc)
    }
}
