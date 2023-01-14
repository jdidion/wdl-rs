use crate::{
    ast::{Command, Runtime, RuntimeAttribute, Task, TaskElement},
    parsers::pest::{PairExt, PairsExt, Rule},
};
use anyhow::{bail, Error, Result};
use pest::iterators::Pair;
use std::convert::TryFrom;

impl<'a> TryFrom<Pair<'a, Rule>> for Command {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let command_body = pair.first_inner()?;
        Ok(Self {
            parts: command_body.into_inner().collect_nodes()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for RuntimeAttribute {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            name: inner.next_string_node()?,
            expression: inner.next_node()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Runtime {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        Ok(Self {
            attributes: pair.into_inner().collect_nodes()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for TaskElement {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let e = match pair.as_rule() {
            Rule::input => Self::Input(pair.try_into()?),
            Rule::output => Self::Output(pair.try_into()?),
            Rule::meta => Self::Meta(pair.try_into()?),
            Rule::parameter_meta => Self::ParameterMeta(pair.try_into()?),
            Rule::command => Self::Command(pair.try_into()?),
            Rule::runtime => Self::Runtime(pair.try_into()?),
            _ => bail!("Invalid task element {:?}", pair),
        };
        Ok(e)
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Task {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            name: inner.next_string_node()?,
            body: inner.collect_nodes()?,
        })
    }
}
