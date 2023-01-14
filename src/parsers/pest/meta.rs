use crate::{
    ast::{
        Float, Integer, Meta, MetaArray, MetaAttribute, MetaObject, MetaObjectField, MetaString,
        MetaStringPart, MetaValue,
    },
    parsers::pest::{PairExt, PairsExt, Rule},
};
use anyhow::{bail, Error, Result};
use pest::iterators::Pair;
use std::convert::TryFrom;

impl<'a> TryFrom<Pair<'a, Rule>> for MetaStringPart {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let part = match pair.as_rule() {
            Rule::simple_squote_literal | Rule::simple_dquote_literal => {
                Self::Content(pair.as_string())
            }
            Rule::simple_squote_escape_sequence | Rule::simple_dquote_escape_sequence => {
                Self::Escape(pair.as_string())
            }
            _ => bail!("Invalid meta string part {:?}", pair),
        };
        Ok(part)
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for MetaString {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        Ok(Self {
            parts: pair.into_inner().collect_nodes()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for MetaArray {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        Ok(Self {
            elements: pair.into_inner().collect_nodes()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for MetaObjectField {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            name: inner.next_string_node()?,
            value: inner.next_node()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for MetaObject {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        Ok(Self {
            fields: pair.into_inner().collect_nodes()?,
        })
    }
}

fn pest_meta_number(pair: Pair<Rule>, negate: bool) -> Result<MetaValue> {
    let n = match pair.as_rule() {
        Rule::int => {
            let mut i: Integer = pair.try_into()?;
            if negate {
                i.negate()?;
            }
            MetaValue::Int(i)
        }
        Rule::float => {
            let mut f: Float = pair.try_into()?;
            if negate {
                f.negate()?;
            }
            MetaValue::Float(f)
        }
        _ => bail!("Invalid number {:?}", pair),
    };
    Ok(n)
}

impl<'a> TryFrom<Pair<'a, Rule>> for MetaValue {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let value = match pair.as_rule() {
            Rule::null => Self::Null,
            Rule::true_literal => Self::Boolean(true),
            Rule::false_literal => Self::Boolean(false),
            Rule::meta_number => {
                let mut inner = pair.into_inner();
                let first_pair = inner.next_pair()?;
                let first_rule = first_pair.as_rule();
                match first_rule {
                    Rule::pos | Rule::neg => {
                        pest_meta_number(inner.next_pair()?, first_rule == Rule::neg)?
                    }
                    _ => pest_meta_number(first_pair, false)?,
                }
            }
            Rule::simple_string => Self::String(pair.try_into()?),
            Rule::meta_array => Self::Array(pair.try_into()?),
            Rule::meta_object => Self::Object(pair.try_into()?),
            _ => bail!("Ivalid meta value {:?}", pair),
        };
        Ok(value)
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for MetaAttribute {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            name: inner.next_string_node()?,
            value: inner.next_node()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Meta {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        Ok(Self {
            attributes: pair.into_inner().collect_nodes()?,
        })
    }
}
