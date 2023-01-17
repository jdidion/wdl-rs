use crate::{
    model::{Float, Integer},
    parsers::pest::PestNode,
};
use anyhow::{Error, Result};
use std::str::FromStr;

impl<'a> TryFrom<PestNode<'a>> for Integer {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let inner = node.first_inner()?;
        Self::from_str(inner.as_str())
    }
}

impl<'a> TryFrom<PestNode<'a>> for Float {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        Self::from_str(node.as_str())
    }
}
