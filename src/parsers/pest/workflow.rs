use crate::{
    ast::{
        Call, CallInput, Conditional, Input, Meta, Node, Output, QualifiedName, Scatter, Workflow,
        WorkflowBodyElement, WorkflowElement,
    },
    parsers::pest::{PairExt, PairsExt, Rule},
};
use anyhow::{bail, Error, Result};
use pest::iterators::Pair;
use std::convert::TryFrom;

impl<'a> TryFrom<Pair<'a, Rule>> for QualifiedName {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        Ok(Self {
            parts: pair.into_inner().collect_string_nodes()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for CallInput {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        let name = inner.next_string_node()?;
        let expression = inner
            .next()
            .map(|expr_pair| Node::try_from(expr_pair))
            .transpose()?;
        Ok(Self { name, expression })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Call {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        let target = inner.next_node()?;
        let alias = if let Some(Rule::call_alias) = inner.peek().map(|pair| pair.as_rule()) {
            let alias_pair = inner.next_pair()?;
            Some(alias_pair.first_inner_string_node()?)
        } else {
            None
        };
        let inputs = if let Some(inputs_pair) = inner.next() {
            inputs_pair.into_inner().collect_nodes()?
        } else {
            Vec::new()
        };
        Ok(Self {
            target,
            alias,
            inputs,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Scatter {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            name: inner.next_string_node()?,
            expression: inner.next_node()?,
            body: inner.collect_nodes()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Conditional {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            expression: inner.next_node()?,
            body: inner.collect_nodes()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for WorkflowBodyElement {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let e = match pair.as_rule() {
            Rule::call => Self::Call(Call::try_from(pair)?),
            Rule::scatter => Self::Scatter(Scatter::try_from(pair)?),
            Rule::conditional => Self::Conditional(Conditional::try_from(pair)?),
            _ => bail!("Invalid task element {:?}", pair),
        };
        Ok(e)
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for WorkflowElement {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let e = match pair.as_rule() {
            Rule::input => Self::Input(Input::try_from(pair)?),
            Rule::output => Self::Output(Output::try_from(pair)?),
            Rule::meta => Self::Meta(Meta::try_from(pair)?),
            Rule::parameter_meta => Self::ParameterMeta(Meta::try_from(pair)?),
            Rule::call => Self::Call(Call::try_from(pair)?),
            Rule::scatter => Self::Scatter(Scatter::try_from(pair)?),
            Rule::conditional => Self::Conditional(Conditional::try_from(pair)?),
            _ => bail!("Invalid task element {:?}", pair),
        };
        Ok(e)
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Workflow {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            name: inner.next_string_node()?,
            body: inner.collect_nodes()?,
        })
    }
}
