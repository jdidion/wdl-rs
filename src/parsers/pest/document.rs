use crate::{
    model::{Alias, Document, DocumentElement, DocumentSource, Import, Namespace, Struct, Version},
    parsers::pest::{PestNode, Rule},
};
use anyhow::{bail, Error, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<PestNode<'a>> for Version {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let inner = node.first_inner()?;
        Ok(Self {
            identifier: inner.try_str_into_ctx()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Namespace {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        Ok(Self::Explicit(node.try_into()?))
    }
}

impl<'a> TryFrom<PestNode<'a>> for Alias {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            from: inner.next_string_ctx()?,
            to: inner.next_string_ctx()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Import {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        let uri = inner.next_string_ctx()?;
        let namespace = if let Some(Rule::namespace) = inner.peek_rule() {
            Namespace::try_from(inner.next_node()?)?
        } else {
            Namespace::try_from_uri(&uri.element)?
        };
        let aliases = inner.collect_ctxs()?;
        Ok(Self {
            uri,
            namespace,
            aliases,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Struct {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_string_ctx()?,
            fields: inner.collect_ctxs()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for DocumentElement {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let e = match node.as_rule() {
            Rule::import => Self::Import(node.try_into()?),
            Rule::structdef => Self::Struct(node.try_into()?),
            Rule::task => Self::Task(node.try_into()?),
            Rule::workflow => Self::Workflow(node.try_into()?),
            _ => bail!("Invalid node {:?}", node),
        };
        Ok(e)
    }
}

impl<'a> TryFrom<PestNode<'a>> for Document {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let comments = node.comments.clone();
        let mut inner = node.into_inner();
        let version = inner.next_ctx()?;
        let body = inner.collect_ctxs()?;
        let doc = Self {
            source: DocumentSource::default(),
            version,
            body,
            comments: comments.take(),
        };
        doc.validate()?;
        Ok(doc)
    }
}
