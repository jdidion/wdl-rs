use crate::{
    model::{Alias, Document, DocumentElement, DocumentSource, Import, Namespace, Struct, Version},
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
        Ok(Self::Explicit(node.try_into()?))
    }
}

impl<'a> TryFrom<TSNode<'a>> for Alias {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            from: node.field_node(syntax::FROM)?,
            to: node.field_node(syntax::TO)?,
        })
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
        Ok(Self {
            name: node.field_node(syntax::NAME)?,
            fields: node.field_child_nodes(syntax::FIELDS)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for DocumentElement {
    type Error = Error;

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        let element = match node.kind() {
            syntax::IMPORT => Self::Import(node.try_into()?),
            syntax::STRUCT => Self::Struct(node.try_into()?),
            syntax::TASK => Self::Task(node.try_into()?),
            syntax::WORKFLOW => Self::Workflow(node.try_into()?),
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
