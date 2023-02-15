use crate::{
    model::{
        Float, Integer, Meta, MetaArray, MetaAttribute, MetaObject, MetaObjectField, MetaString,
        MetaStringPart, MetaValue, ModelError, ParameterMeta,
    },
    parsers::pest::{node::PestNode, Rule},
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

fn pest_meta_number<'a>(node: PestNode<'a>, negate: bool) -> Result<MetaValue, ModelError> {
    let n = match node.as_rule() {
        Rule::dec_int | Rule::hex_int | Rule::oct_int => {
            let mut i: Integer = node.try_into()?;
            if negate {
                i = i.negate();
            }
            MetaValue::Int(i)
        }
        Rule::float => {
            let mut f: Float = node.try_into()?;
            if negate {
                f = f.negate();
            }
            MetaValue::Float(f)
        }
        _ => bail!(ModelError::parser(format!("Invalid number {:?}", node))),
    };
    Ok(n)
}

impl<'a> TryFrom<PestNode<'a>> for MetaStringPart {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let part = match node.as_rule() {
            Rule::simple_squote_literal | Rule::simple_dquote_literal => {
                Self::Content(node.try_into()?)
            }
            Rule::simple_squote_escape_sequence | Rule::simple_dquote_escape_sequence => {
                Self::Escape(node.try_into()?)
            }
            _ => bail!(ModelError::parser(format!(
                "Invalid meta string part {:?}",
                node
            ))),
        };
        Ok(part)
    }
}

impl<'a> TryFrom<PestNode<'a>> for MetaString {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            parts: node.into_inner().collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for MetaArray {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            elements: node.into_inner().collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for MetaObjectField {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_node().try_into()?,
            value: inner.next_node().try_into()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for MetaObject {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            fields: node.into_inner().collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for MetaValue {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let value = match node.as_rule() {
            Rule::null => Self::Null,
            Rule::boolean => Self::Boolean(node.try_into()?),
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
            Rule::simple_dquote_string | Rule::simple_squote_string => {
                Self::String(node.try_into()?)
            }
            Rule::meta_array => Self::Array(node.try_into()?),
            Rule::meta_object => Self::Object(node.try_into()?),
            _ => bail!(ModelError::parser(format!("Invalid meta value {:?}", node))),
        };
        Ok(value)
    }
}

impl<'a> TryFrom<PestNode<'a>> for MetaAttribute {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_node().try_into()?,
            value: inner.next_node().try_into()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Meta {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            attributes: node.into_inner().collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for ParameterMeta {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            attributes: node.into_inner().collect_anchors()?,
        })
    }
}
