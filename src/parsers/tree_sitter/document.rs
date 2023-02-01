use crate::{
    model::{
        Alias, Anchor, Document, DocumentElement, DocumentSource, Import, ModelError, Namespace,
        Struct, Version,
    },
    parsers::tree_sitter::{
        node::{TSNode, TSNodeIteratorResultExt, TSNodeResultExt},
        syntax,
    },
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<TSNode<'a>> for Version {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            identifier: node
                .try_into_child_field(syntax::IDENTIFIER)
                .into_anchor_from_str()?,
        })
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
        Ok(Self {
            from: children.next_field(syntax::FROM).try_into()?,
            to: children.next_field(syntax::TO).try_into()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Import {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        let uri: Anchor<String> = children.next_field(syntax::URI).try_into()?;
        let next = children.next_node()?;
        let (namespace, next) = match next.try_field()? {
            syntax::NAMESPACE => (
                Namespace::try_from(next)?,
                children.next_field(syntax::ALIASES)?,
            ),
            syntax::ALIAS => (Namespace::from_uri(&uri.element), next),
            other => bail!(ModelError::parser(format!(
                "Invalid import element {}",
                other
            ))),
        };
        let aliases = next.into_children().collect_anchors()?;
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
        Ok(Self {
            name: children.next_field(syntax::NAME).try_into()?,
            fields: children
                .next_field(syntax::FIELDS)
                .into_children()
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for DocumentElement {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let element = match node.kind() {
            syntax::IMPORT => Self::Import(node.try_into()?),
            syntax::STRUCT => Self::Struct(node.try_into()?),
            syntax::TASK => Self::Task(node.try_into()?),
            syntax::WORKFLOW => Self::Workflow(node.try_into()?),
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
        let version = children.next_field(syntax::VERSION).try_into()?;
        let body = children
            .next_field(syntax::BODY)
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
