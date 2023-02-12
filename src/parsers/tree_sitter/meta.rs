use crate::{
    model::{
        Meta, MetaArray, MetaAttribute, MetaObject, MetaObjectField, MetaString, MetaStringPart,
        MetaValue, ModelError,
    },
    parsers::tree_sitter::{node::TSNode, syntax},
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<TSNode<'a>> for MetaStringPart {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let part = match node.kind() {
            syntax::CONTENT => Self::Content(node.try_into()?),
            syntax::ESCAPE_SEQUENCE => Self::Escape(node.try_into()?),
            _ => bail!(ModelError::parser(format!(
                "Invalid string part {:?}",
                node
            ))),
        };
        Ok(part)
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaString {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            parts: node.into_children().collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaArray {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            elements: node
                .try_into_child_field(syntax::ELEMENTS)?
                .into_children()
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaObjectField {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        Ok(Self {
            name: children.next_field(syntax::NAME).try_into()?,
            value: children.next_field(syntax::VALUE).try_into()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaObject {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            fields: node
                .try_into_child_field(syntax::FIELDS)?
                .into_children()
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaValue {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let value = match node.kind() {
            syntax::NULL => Self::Null,
            syntax::TRUE => Self::Boolean(true),
            syntax::FALSE => Self::Boolean(false),
            syntax::DEC_INT | syntax::OCT_INT | syntax::HEX_INT => Self::Int(node.try_into()?),
            syntax::FLOAT => Self::Float(node.try_into()?),
            syntax::SIMPLE_STRING => Self::String(node.try_into()?),
            syntax::META_ARRAY => Self::Array(node.try_into()?),
            syntax::META_OBJECT => Self::Object(node.try_into()?),
            _ => bail!(ModelError::parser(format!("Invalid meta value {:?}", node))),
        };
        Ok(value)
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaAttribute {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        Ok(Self {
            name: children.next_field(syntax::NAME).try_into()?,
            value: children.next_field(syntax::VALUE).try_into()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Meta {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            attributes: node
                .try_into_child_field(syntax::ATTRIBUTES)?
                .into_children()
                .collect_anchors()?,
        })
    }
}
