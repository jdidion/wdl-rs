use crate::{
    model::{Float, Integer},
    parsers::pest::{PairExt, Rule},
};
use anyhow::{Error, Result};
use pest::iterators::Pair;
use std::str::FromStr;

impl<'a> TryFrom<Pair<'a, Rule>> for Integer {
    type Error = Error;

    fn try_from(pair: Pair<'a, Rule>) -> Result<Self> {
        let inner = pair.first_inner()?;
        Self::from_str(inner.as_str())
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Float {
    type Error = Error;

    fn try_from(pair: Pair<'a, Rule>) -> Result<Self> {
        Self::from_str(pair.as_str())
    }
}
