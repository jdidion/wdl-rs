use crate::{
    ast::{Command, Input, Meta, Output, Runtime, RuntimeAttribute, Task, TaskElement},
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
            Rule::input => Self::Input(Input::try_from(pair)?),
            Rule::output => Self::Output(Output::try_from(pair)?),
            Rule::meta => Self::Meta(Meta::try_from(pair)?),
            Rule::parameter_meta => Self::ParameterMeta(Meta::try_from(pair)?),
            Rule::command => Self::Command(Command::try_from(pair)?),
            Rule::runtime => Self::Runtime(Runtime::try_from(pair)?),
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
