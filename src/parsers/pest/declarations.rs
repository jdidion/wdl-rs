use crate::{
    model::{BoundDeclaration, Input, InputDeclaration, Output, Type, UnboundDeclaration},
    parsers::pest::{PairExt, PairsExt, Rule},
};
use anyhow::{bail, Error, Ok, Result};
use pest::iterators::Pair;
use std::convert::TryFrom;

impl<'a> TryFrom<Pair<'a, Rule>> for Type {
    type Error = Error;

    fn try_from(pair: Pair<'a, Rule>) -> Result<Self> {
        let type_pair = pair.first_inner()?;
        let t = match type_pair.as_rule() {
            Rule::optional_type => Self::Optional(type_pair.first_inner_boxed_node()?),
            Rule::primitive_type => match type_pair.as_str() {
                "Boolean" => Self::Boolean,
                "Int" => Self::Int,
                "Float" => Self::Float,
                "String" => Self::String,
                "File" => Self::File,
                "Object" => Self::Object,
                _ => bail!("Invalid primitive type {:?}", type_pair),
            },
            Rule::non_empty_array_type => Self::NonEmpty(type_pair.first_inner_boxed_node()?),
            Rule::maybe_empty_array_type => Self::Array(type_pair.first_inner_boxed_node()?),
            Rule::map_type => {
                let mut inner = type_pair.into_inner();
                let key = inner.next_boxed_node()?;
                let value = inner.next_boxed_node()?;
                Self::Map { key, value }
            }
            Rule::pair_type => {
                let mut inner = type_pair.into_inner();
                let left = inner.next_boxed_node()?;
                let right = inner.next_boxed_node()?;
                Self::Pair { left, right }
            }
            Rule::user_type => Self::User(type_pair.first_inner_string()?),
            _ => bail!("Invalid typedef {:?}", type_pair),
        };
        Ok(t)
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for UnboundDeclaration {
    type Error = Error;

    fn try_from(pair: Pair<'a, Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            wdl_type: inner.next_node()?,
            name: inner.next_string_node()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for BoundDeclaration {
    type Error = Error;

    fn try_from(pair: Pair<'a, Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            wdl_type: inner.next_node()?,
            name: inner.next_string_node()?,
            expression: inner.next_node()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for InputDeclaration {
    type Error = Error;

    fn try_from(pair: Pair<'a, Rule>) -> Result<Self> {
        let decl = match pair.as_rule() {
            Rule::unbound_declaration => Self::Unbound(pair.try_into()?),
            Rule::bound_declaration => Self::Bound(pair.try_into()?),
            _ => bail!("Invalid declaration {:?}", pair),
        };
        Ok(decl)
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Input {
    type Error = Error;

    fn try_from(pair: Pair<'a, Rule>) -> Result<Self> {
        Ok(Input {
            declarations: pair.into_inner().collect_nodes()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Output {
    type Error = Error;

    fn try_from(pair: Pair<'a, Rule>) -> Result<Self> {
        Ok(Output {
            declarations: pair.into_inner().collect_nodes()?,
        })
    }
}
