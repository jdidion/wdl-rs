use crate::{
    model::{
        BoundDeclaration, Input, InputDeclaration, ModelError, Output, Type, UnboundDeclaration,
    },
    parsers::pest::{node::PestNode, Rule},
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

use super::node::PestNodeResultExt;

impl<'a> TryFrom<PestNode<'a>> for Type {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let type_node = node.one_inner()?;
        let t = match type_node.as_rule() {
            Rule::optional_type => Self::Optional(type_node.one_inner().into_boxed_anchor()?),
            Rule::primitive_type => match type_node.as_str() {
                "Boolean" => Self::Boolean,
                "Int" => Self::Int,
                "Float" => Self::Float,
                "String" => Self::String,
                "File" => Self::File,
                "Object" => Self::Object,
                _ => bail!(ModelError::parser(format!(
                    "Invalid primitive type {:?}",
                    type_node
                ))),
            },
            Rule::non_empty_array_type => {
                Self::NonEmpty(type_node.one_inner().into_boxed_anchor()?)
            }
            Rule::maybe_empty_array_type => Self::Array(type_node.one_inner().into_boxed_anchor()?),
            Rule::map_type => {
                let mut inner = type_node.into_inner();
                let key = inner.next_node().into_boxed_anchor()?;
                let value = inner.next_node().into_boxed_anchor()?;
                Self::Map { key, value }
            }
            Rule::pair_type => {
                let mut inner = type_node.into_inner();
                let left = inner.next_node().into_boxed_anchor()?;
                let right = inner.next_node().into_boxed_anchor()?;
                Self::Pair { left, right }
            }
            Rule::user_type => Self::User(type_node.one_inner().into_string()?),
            _ => return type_node.into_err(|node| format!("Invalid typedef {:?}", node)),
        };
        Ok(t)
    }
}

impl<'a> TryFrom<PestNode<'a>> for UnboundDeclaration {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            wdl_type: inner.next_node().try_into()?,
            name: inner.next_node().try_into()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for BoundDeclaration {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            wdl_type: inner.next_node().try_into()?,
            name: inner.next_node().try_into()?,
            expression: inner.next_node().try_into()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for InputDeclaration {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let decl = match node.as_rule() {
            Rule::unbound_declaration => Self::Unbound(node.try_into()?),
            Rule::bound_declaration => Self::Bound(node.try_into()?),
            _ => return node.into_err(|node| format!("Invalid declaration {:?}", node)),
        };
        Ok(decl)
    }
}

impl<'a> TryFrom<PestNode<'a>> for Input {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(Input {
            declarations: node.into_inner().collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Output {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(Output {
            declarations: node.into_inner().collect_anchors()?,
        })
    }
}
