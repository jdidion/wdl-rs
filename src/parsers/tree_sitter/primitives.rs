use crate::{
    model::{Float, Integer},
    parsers::tree_sitter::{syntax, TSNode},
};
use anyhow::{bail, Error, Result};
use std::{convert::TryFrom, str::FromStr};

impl<'a> TryFrom<TSNode<'a>> for Integer {
    type Error = Error;

    // fn nodes() -> Option<StrSet> {
    //     Some(StrSet::from([
    //         syntax::DEC_INT,
    //         syntax::OCT_INT,
    //         syntax::HEX_INT,
    //     ]))
    // }

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        let int_str: &str = node.try_as_str()?;
        let n = match node.kind() {
            syntax::DEC_INT => Self::Decimal(int_str.parse::<i64>()?),
            syntax::OCT_INT => Self::Octal(int_str.to_owned()),
            syntax::HEX_INT => Self::Hex(int_str.to_owned()),
            _ => bail!("Invalid number {:?}", node),
        };
        Ok(n)
    }
}

impl<'a> TryFrom<TSNode<'a>> for Float {
    type Error = Error;

    // fn nodes() -> Option<StrSet> {
    //     Some(StrSet::from([syntax::FLOAT]))
    // }

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        Self::from_str(node.try_as_str()?)
    }
}
