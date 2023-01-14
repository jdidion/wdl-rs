use crate::{
    ast::{
        Meta, MetaArray, MetaAttribute, MetaObject, MetaObjectField, MetaString, MetaStringPart,
        MetaValue,
    },
    parsers::tree_sitter::{syntax, TSNode},
};
use anyhow::{bail, Error, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<TSNode<'a>> for MetaStringPart {
    type Error = Error;

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        let part = match node.kind() {
            syntax::CONTENT => Self::Content(node.try_as_string()?),
            syntax::ESCAPE_SEQUENCE => Self::Escape(node.try_as_string()?),
            _ => bail!("Invalid string part {:?}", node),
        };
        Ok(part)
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaString {
    type Error = Error;

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            parts: node.child_nodes()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaArray {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            elements: node.get_field_child_nodes(syntax::ELEMENTS)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaObjectField {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            name: node.field_node(syntax::NAME)?,
            value: node.field_node(syntax::VALUE)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaObject {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            fields: node.field_child_nodes(syntax::FIELDS)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaValue {
    type Error = Error;

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        let value = match node.kind() {
            syntax::NULL => Self::Null,
            syntax::TRUE => Self::Boolean(true),
            syntax::FALSE => Self::Boolean(false),
            syntax::DEC_INT | syntax::OCT_INT | syntax::HEX_INT => Self::Int(node.try_into()?),
            syntax::FLOAT => Self::Float(node.try_into()?),
            syntax::SIMPLE_STRING => Self::String(node.try_into()?),
            syntax::META_ARRAY => Self::Array(node.try_into()?),
            syntax::META_OBJECT => Self::Object(node.try_into()?),
            _ => bail!("Invalid meta value {:?}", node),
        };
        Ok(value)
    }
}

impl<'a> TryFrom<TSNode<'a>> for MetaAttribute {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            name: node.field_node(syntax::NAME)?,
            value: node.field_node(syntax::VALUE)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Meta {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            attributes: node.field_child_nodes(syntax::ATTRIBUTES)?,
        })
    }
}
