use crate::{
    model::{
        Alias, Anchor, Document, DocumentElement, DocumentSource, Import, ModelError, Namespace,
        Struct, Version,
    },
    parsers::tree_sitter::{
        node::{TSIteratorExt, TSNode},
        syntax::{fields, keywords, rules, symbols},
    },
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<TSNode<'a>> for Version {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        children.skip_terminal(keywords::VERSION)?;
        Ok(Self {
            identifier: children
                .next_field(fields::IDENTIFIER)?
                .try_into_anchor_from_str()?,
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
        children.skip_terminal(keywords::ALIAS)?;
        let from = children.next_field(fields::FROM).try_into()?;
        children.skip_terminal(keywords::AS)?;
        let to = children.next_field(fields::TO).try_into()?;
        Ok(Self { from, to })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Import {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        children.skip_terminal(keywords::IMPORT)?;
        let uri: Anchor<String> = children.next_field(fields::URI).try_into()?;
        let namespace = if children.skip_optional(keywords::AS)? {
            children.next_field(fields::NAMESPACE)?.try_into()?
        } else {
            Namespace::from_uri(&uri.element)
        };
        let aliases = if let Some(next) = children.get_next_field(fields::ALIASES)? {
            next.into_children().collect_anchors()?
        } else {
            Vec::new()
        };
        println!("returning from import");
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
        Ok(Self {
            name: children.next_field(fields::NAME).try_into()?,
            fields: children
                .next_field(fields::FIELDS)?
                .into_block(symbols::LBRACE, symbols::RBRACE)
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for DocumentElement {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        println!("before DocumentElement");
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
