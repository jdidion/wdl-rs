use crate::{
    model::{
        Float, Integer, Meta, MetaArray, MetaAttribute, MetaObject, MetaObjectField, MetaString,
        MetaStringPart, MetaValue,
    },
    parsers::pest::{PestNode, Rule},
};
use anyhow::{bail, Error, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<PestNode<'a>> for MetaStringPart {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let part = match node.as_rule() {
            Rule::simple_squote_literal | Rule::simple_dquote_literal => {
                Self::Content(node.as_string())
            }
            Rule::simple_squote_escape_sequence | Rule::simple_dquote_escape_sequence => {
                Self::Escape(node.as_string())
            }
            _ => bail!("Invalid meta string part {:?}", node),
        };
        Ok(part)
    }
}

impl<'a> TryFrom<PestNode<'a>> for MetaString {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        Ok(Self {
            parts: node.into_inner().collect_ctxs()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for MetaArray {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        Ok(Self {
            elements: node.into_inner().collect_ctxs()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for MetaObjectField {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_string_ctx()?,
            value: inner.next_ctx()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for MetaObject {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        Ok(Self {
            fields: node.into_inner().collect_ctxs()?,
        })
    }
}

fn pest_meta_number<'a>(node: PestNode<'a>, negate: bool) -> Result<MetaValue> {
    let n = match node.as_rule() {
        Rule::int => {
            let mut i: Integer = node.try_into()?;
            if negate {
                i.negate()?;
            }
            MetaValue::Int(i)
        }
        Rule::float => {
            let mut f: Float = node.try_into()?;
            if negate {
                f.negate()?;
            }
            MetaValue::Float(f)
        }
        _ => bail!("Invalid number {:?}", node),
    };
    Ok(n)
}

impl<'a> TryFrom<PestNode<'a>> for MetaValue {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let value = match node.as_rule() {
            Rule::null => Self::Null,
            Rule::true_literal => Self::Boolean(true),
            Rule::false_literal => Self::Boolean(false),
            Rule::meta_number => {
                let mut inner = node.into_inner();
                let first_node = inner.next_node()?;
                let first_rule = first_node.as_rule();
                match first_rule {
                    Rule::pos | Rule::neg => {
                        pest_meta_number(inner.next_node()?, first_rule == Rule::neg)?
                    }
                    _ => pest_meta_number(first_node, false)?,
                }
            }
            Rule::simple_string => Self::String(node.try_into()?),
            Rule::meta_array => Self::Array(node.try_into()?),
            Rule::meta_object => Self::Object(node.try_into()?),
            _ => bail!("Ivalid meta value {:?}", node),
        };
        Ok(value)
    }
}

impl<'a> TryFrom<PestNode<'a>> for MetaAttribute {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_string_ctx()?,
            value: inner.next_ctx()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Meta {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        Ok(Self {
            attributes: node.into_inner().collect_ctxs()?,
        })
    }
}
