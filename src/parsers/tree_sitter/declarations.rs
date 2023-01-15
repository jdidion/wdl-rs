use crate::{
    model::{BoundDeclaration, Input, InputDeclaration, Output, Type, UnboundDeclaration},
    parsers::tree_sitter::{syntax, TSNode},
};
use anyhow::{bail, Error, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<TSNode<'a>> for Type {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        let t = match node.kind() {
            syntax::OPTIONAL_TYPE => Self::Optional(node.field_boxed_node(syntax::TYPE)?),
            syntax::BOOLEAN_TYPE => Self::Boolean,
            syntax::INT_TYPE => Self::Int,
            syntax::FLOAT_TYPE => Self::Float,
            syntax::STRING_TYPE => Self::String,
            syntax::FILE_TYPE => Self::File,
            syntax::OBJECT_TYPE => Self::Object,
            syntax::NONEMPTY_ARRAY_TYPE => Self::NonEmpty(node.field_boxed_node(syntax::TYPE)?),
            syntax::ARRAY_TYPE => Self::Array(node.field_boxed_node(syntax::TYPE)?),
            syntax::MAP_TYPE => {
                let key = node.field_boxed_node(syntax::KEY)?;
                let value = node.field_boxed_node(syntax::VALUE)?;
                Self::Map { key, value }
            }
            syntax::PAIR_TYPE => {
                let left = node.field_boxed_node(syntax::LEFT)?;
                let right = node.field_boxed_node(syntax::RIGHT)?;
                Self::Pair { left, right }
            }
            syntax::USER_TYPE => Self::User(node.field_string(syntax::NAME)?),
            _ => bail!("Invalid type {:?}", node),
        };
        Ok(t)
    }
}

impl<'a> TryFrom<TSNode<'a>> for UnboundDeclaration {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            wdl_type: node.field_node(syntax::TYPE)?,
            name: node.field_string_node(syntax::NAME)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for BoundDeclaration {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            wdl_type: node.field_node(syntax::TYPE)?,
            name: node.field_string_node(syntax::NAME)?,
            expression: node.field_node(syntax::EXPRESSION)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for InputDeclaration {
    type Error = Error;

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        let decl = match node.kind() {
            syntax::BOUND_DECLARATION => Self::Bound(node.try_into()?),
            syntax::UNBOUND_DECLARATION => Self::Unbound(node.try_into()?),
            _ => bail!("Invalid declaration {:?}", node),
        };
        Ok(decl)
    }
}

impl<'a> TryFrom<TSNode<'a>> for Input {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            declarations: node.field_child_nodes(syntax::DECLARATIONS)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Output {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            declarations: node.field_child_nodes(syntax::DECLARATIONS)?,
        })
    }
}
