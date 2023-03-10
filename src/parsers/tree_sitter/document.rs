use crate::{
    model::{
        Alias, Anchor, Document, DocumentElement, DocumentSource, Import, ModelError, Namespace,
        Struct, Version,
    },
    parsers::tree_sitter::{
        node::{BlockDelim, BlockEnds, TSNode},
        syntax::{fields, keywords, rules},
    },
};
use error_stack::{bail, Report, Result};
use std::{convert::TryFrom, ops::Deref};

impl<'a> TryFrom<TSNode<'a>> for Version {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        children.skip_terminal(keywords::VERSION)?;
        let identifier = children
            .next_field(fields::IDENTIFIER)?
            .try_into_anchor_from_str()?;
        Ok(Self { identifier })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Namespace {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self::Explicit(node.try_into()?))
    }
}

impl<'a> TryFrom<TSNode<'a>> for Alias {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        children.skip_terminal(keywords::ALIAS)?;
        let from = children.next_field(fields::FROM).try_into()?;
        children.skip_terminal(keywords::AS)?;
        let to = children.next_field(fields::TO).try_into()?;
        Ok(Self { from, to })
    }
}

fn uri_string<'a>(node: TSNode<'a>) -> Result<Anchor<String>, ModelError> {
    let span = node.as_span();
    let node_parts: Result<Vec<TSNode<'a>>, ModelError> = node
        .into_block(BlockEnds::Quotes, BlockDelim::None)
        .collect();
    let str_parts: Result<Vec<&str>, ModelError> = node_parts?
        .into_iter()
        .map(|node| node.try_as_str())
        .collect();
    Ok(Anchor::new(str_parts?.join(""), span))
}

impl<'a> TryFrom<TSNode<'a>> for Import {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        children.skip_terminal(keywords::IMPORT)?;
        let uri: Anchor<String> = uri_string(children.next_field(fields::URI)?)?;
        let next = children.next().transpose()?;
        let (namespace, next) = match next {
            Some(node) if node.kind() == keywords::AS => (
                children.next_field(fields::NAMESPACE)?.try_into()?,
                children.next().transpose()?,
            ),
            _ => (Namespace::from_uri(uri.deref()), next),
        };
        let aliases = next
            .map(|node| {
                node.ensure_field(fields::ALIASES)?;
                node.into_children().collect_anchors()
            })
            .transpose()?
            .unwrap_or_else(|| Vec::new());
        Ok(Self {
            uri,
            namespace,
            aliases,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Struct {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        children.skip_terminal(keywords::STRUCT)?;
        let name = children.next_field(fields::NAME).try_into()?;
        let fields = children
            .next_field(fields::FIELDS)?
            .into_block(BlockEnds::Braces, BlockDelim::None)
            .collect_anchors()?;
        Ok(Self { name, fields })
    }
}

impl<'a> TryFrom<TSNode<'a>> for DocumentElement {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let element = match node.kind() {
            rules::IMPORT => Self::Import(node.try_into()?),
            rules::STRUCT => Self::Struct(node.try_into()?),
            rules::TASK => Self::Task(node.try_into()?),
            rules::WORKFLOW => Self::Workflow(node.try_into()?),
            other => bail!(ModelError::parser(format!("Invalid node {}", other))),
        };
        Ok(element)
    }
}

impl<'a> TryFrom<TSNode<'a>> for Document {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let comments = node.clone_comments();
        let mut children = node.into_children();
        let version = children.next_field(fields::VERSION).try_into()?;
        let body = children
            .next_field(fields::BODY)?
            .into_children()
            .collect_anchors()?;
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
