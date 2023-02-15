use crate::{
    model::{
        Meta, MetaArray, MetaAttribute, MetaObject, MetaObjectField, MetaString, MetaStringPart,
        MetaValue, ModelError, ParameterMeta,
    },
    parsers::tree_sitter::{
        node::TSNode,
        syntax::{fields, keywords, rules, symbols},
    },
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

use super::node::TSIteratorExt;

impl<'a> TryFrom<TSNode<'a>> for MetaStringPart {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let part = match node.kind() {
            rules::CONTENT => Self::Content(node.try_into()?),
            rules::ESCAPE_SEQUENCE => Self::Escape(node.try_into()?),
            _ => bail!(ModelError::parser(format!(
                "Invalid meta string part {:?}",
                node
            ))),
        };
        Ok(part)
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaString {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        let start_quote = children.next_node()?.try_as_str()?;
        let parts = match children.get_next_field(fields::PARTS)? {
            Some(parts) => parts.into_children().collect_anchors()?,
            None => Vec::new(),
        };
        let end_quote = children.next_node()?.try_as_str()?;
        assert_eq!(start_quote, end_quote);
        Ok(Self { parts })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaArray {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            elements: node
                .try_into_child_field(fields::ELEMENTS)?
                .into_list(symbols::COMMA, Some(symbols::LBRACK), Some(symbols::RBRACK))
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaObjectField {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        let name = children.next_field(fields::NAME).try_into()?;
        children.skip_terminal(symbols::COLON)?;
        let value = children.next_field(fields::VALUE).try_into()?;
        Ok(Self { name, value })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaObject {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            fields: node
                .try_into_child_field(fields::FIELDS)?
                .into_list(symbols::COMMA, Some(symbols::LBRACE), Some(symbols::RBRACE))
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaValue {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let value = match node.kind() {
            rules::NULL => Self::Null,
            rules::TRUE => Self::Boolean(true),
            rules::FALSE => Self::Boolean(false),
            rules::DEC_INT | rules::OCT_INT | rules::HEX_INT => Self::Int(node.try_into()?),
            rules::FLOAT => Self::Float(node.try_into()?),
            rules::SIMPLE_STRING => Self::String(node.try_into()?),
            rules::META_ARRAY => Self::Array(node.try_into()?),
            rules::META_OBJECT => Self::Object(node.try_into()?),
            _ => bail!(ModelError::parser(format!("Invalid meta value {:?}", node))),
        };
        Ok(value)
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaAttribute {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        let name = children.next_field(fields::NAME).try_into()?;
        children.skip_terminal(symbols::COLON)?;
        let value = children.next_field(fields::VALUE).try_into()?;
        Ok(Self { name, value })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Meta {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        children.skip_terminal(keywords::META)?;
        Ok(Self {
            attributes: children
                .next_field(fields::ATTRIBUTES)?
                .into_block(symbols::LBRACE, symbols::RBRACE)
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for ParameterMeta {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        children.skip_terminal(keywords::PARAMETER_META)?;
        Ok(Self {
            attributes: children
                .next_field(fields::ATTRIBUTES)?
                .into_block(symbols::LBRACE, symbols::RBRACE)
                .collect_anchors()?,
        })
    }
}
