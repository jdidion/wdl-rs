use crate::{
    model::{
        Alias, Anchor, Document, DocumentElement, DocumentSource, Import, ModelError, Namespace,
        Struct, Version,
    },
    parsers::pest::{node::PestNode, Rule},
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<PestNode<'a>> for Version {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            identifier: node.try_into_anchor_from_str()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Namespace {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(Self::Explicit(node.try_into()?))
    }
}

impl<'a> TryFrom<PestNode<'a>> for Alias {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            from: inner.next_node().try_into()?,
            to: inner.next_node().try_into()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Import {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        let uri: Anchor<String> = inner.next_node().try_into()?;
        let namespace = if let Some(Rule::namespace) = inner.peek_rule() {
            Namespace::try_from(inner.next_node()?)?
        } else {
            Namespace::from_uri(&uri.element)
        };
        let aliases = inner.collect_anchors()?;
        Ok(Self {
            uri,
            namespace,
            aliases,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Struct {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_node().try_into()?,
            fields: inner.collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for DocumentElement {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let e = match node.as_rule() {
            Rule::import => Self::Import(node.try_into()?),
            Rule::structdef => Self::Struct(node.try_into()?),
            Rule::task => Self::Task(node.try_into()?),
            Rule::workflow => Self::Workflow(node.try_into()?),
            _ => bail!(ModelError::parser(format!(
                "Invalid document element {:?}",
                node
            ))),
        };
        Ok(e)
    }
}

impl<'a> TryFrom<PestNode<'a>> for Document {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let comments = node.clone_comments();
        let mut inner = node.into_inner();
        let version = inner.next_node().try_into()?;
        let body = inner.collect_anchors()?;
        Ok(Self {
            source: DocumentSource::default(),
            version,
            body,
            comments: comments.take(),
        })
    }
}
