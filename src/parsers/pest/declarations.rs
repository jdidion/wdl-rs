use crate::{
    model::{BoundDeclaration, Input, InputDeclaration, Output, Type, UnboundDeclaration},
    parsers::pest::{PestNode, Rule},
};
use anyhow::{bail, Error, Ok, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<PestNode<'a>> for Type {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let type_node = node.first_inner()?;
        let t = match type_node.as_rule() {
            Rule::optional_type => Self::Optional(type_node.first_inner_boxed_ctx()?),
            Rule::primitive_type => match type_node.as_str() {
                "Boolean" => Self::Boolean,
                "Int" => Self::Int,
                "Float" => Self::Float,
                "String" => Self::String,
                "File" => Self::File,
                "Object" => Self::Object,
                _ => bail!("Invalid primitive type {:?}", type_node),
            },
            Rule::non_empty_array_type => Self::NonEmpty(type_node.first_inner_boxed_ctx()?),
            Rule::maybe_empty_array_type => Self::Array(type_node.first_inner_boxed_ctx()?),
            Rule::map_type => {
                let mut inner = type_node.into_inner();
                let key = inner.next_boxed_ctx()?;
                let value = inner.next_boxed_ctx()?;
                Self::Map { key, value }
            }
            Rule::pair_type => {
                let mut inner = type_node.into_inner();
                let left = inner.next_boxed_ctx()?;
                let right = inner.next_boxed_ctx()?;
                Self::Pair { left, right }
            }
            Rule::user_type => Self::User(type_node.first_inner_string()?),
            _ => bail!("Invalid typedef {:?}", type_node),
        };
        Ok(t)
    }
}

impl<'a> TryFrom<PestNode<'a>> for UnboundDeclaration {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            wdl_type: inner.next_ctx()?,
            name: inner.next_string_ctx()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for BoundDeclaration {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            wdl_type: inner.next_ctx()?,
            name: inner.next_string_ctx()?,
            expression: inner.next_ctx()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for InputDeclaration {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let decl = match node.as_rule() {
            Rule::unbound_declaration => Self::Unbound(node.try_into()?),
            Rule::bound_declaration => Self::Bound(node.try_into()?),
            _ => bail!("Invalid declaration {:?}", node),
        };
        Ok(decl)
    }
}

impl<'a> TryFrom<PestNode<'a>> for Input {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        Ok(Input {
            declarations: node.into_inner().collect_ctxs()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Output {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        Ok(Output {
            declarations: node.into_inner().collect_ctxs()?,
        })
    }
}
