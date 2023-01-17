use crate::{
    model::{Command, Runtime, RuntimeAttribute, Task, TaskElement},
    parsers::pest::{PestNode, Rule},
};
use anyhow::{bail, Error, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<PestNode<'a>> for Command {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let command_body = node.first_inner()?;
        Ok(Self {
            parts: command_body.into_inner().collect_ctxs()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for RuntimeAttribute {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_string_ctx()?,
            expression: inner.next_ctx()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Runtime {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        Ok(Self {
            attributes: node.into_inner().collect_ctxs()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for TaskElement {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let e = match node.as_rule() {
            Rule::input => Self::Input(node.try_into()?),
            Rule::output => Self::Output(node.try_into()?),
            Rule::meta => Self::Meta(node.try_into()?),
            Rule::parameter_meta => Self::ParameterMeta(node.try_into()?),
            Rule::command => Self::Command(node.try_into()?),
            Rule::runtime => Self::Runtime(node.try_into()?),
            _ => bail!("Invalid task element {:?}", node),
        };
        Ok(e)
    }
}

impl<'a> TryFrom<PestNode<'a>> for Task {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_string_ctx()?,
            body: inner.collect_ctxs()?,
        })
    }
}
