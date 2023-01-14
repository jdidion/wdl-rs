use crate::{
    ast::{
        Call, CallInput, Conditional, QualifiedName, Scatter, Workflow, WorkflowBodyElement,
        WorkflowElement,
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
            .map(|expr_pair| expr_pair.try_into())
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
            Rule::call => Self::Call(pair.try_into()?),
            Rule::scatter => Self::Scatter(pair.try_into()?),
            Rule::conditional => Self::Conditional(pair.try_into()?),
            _ => bail!("Invalid task element {:?}", pair),
        };
        Ok(e)
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for WorkflowElement {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let e = match pair.as_rule() {
            Rule::input => Self::Input(pair.try_into()?),
            Rule::output => Self::Output(pair.try_into()?),
            Rule::meta => Self::Meta(pair.try_into()?),
            Rule::parameter_meta => Self::ParameterMeta(pair.try_into()?),
            Rule::call => Self::Call(pair.try_into()?),
            Rule::scatter => Self::Scatter(pair.try_into()?),
            Rule::conditional => Self::Conditional(pair.try_into()?),
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
