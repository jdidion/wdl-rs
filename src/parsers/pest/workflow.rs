use crate::{
    model::{
        Call, CallInput, Conditional, QualifiedName, Scatter, Workflow, WorkflowBodyElement,
        WorkflowElement,
    },
    parsers::pest::{PestNode, Rule},
};
use anyhow::{bail, Error, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<PestNode<'a>> for QualifiedName {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        Ok(Self {
            parts: node.into_inner().collect_string_ctxs()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for CallInput {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        let name = inner.next_string_ctx()?;
        let expression = inner
            .next()
            .map(|expr_node| expr_node.try_into())
            .transpose()?;
        Ok(Self { name, expression })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Call {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        let target = inner.next_ctx()?;
        let alias = if let Some(Rule::call_alias) = inner.peek_rule() {
            let alias_node = inner.next_node()?;
            Some(alias_node.first_inner_string_ctx()?)
        } else {
            None
        };
        let inputs = if let Some(inputs_node) = inner.next() {
            inputs_node.into_inner().collect_ctxs()?
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

impl<'a> TryFrom<PestNode<'a>> for Scatter {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_string_ctx()?,
            expression: inner.next_ctx()?,
            body: inner.collect_ctxs()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Conditional {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            expression: inner.next_ctx()?,
            body: inner.collect_ctxs()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for WorkflowBodyElement {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let e = match node.as_rule() {
            Rule::call => Self::Call(node.try_into()?),
            Rule::scatter => Self::Scatter(node.try_into()?),
            Rule::conditional => Self::Conditional(node.try_into()?),
            _ => bail!("Invalid task element {:?}", node),
        };
        Ok(e)
    }
}

impl<'a> TryFrom<PestNode<'a>> for WorkflowElement {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let e = match node.as_rule() {
            Rule::input => Self::Input(node.try_into()?),
            Rule::output => Self::Output(node.try_into()?),
            Rule::meta => Self::Meta(node.try_into()?),
            Rule::parameter_meta => Self::ParameterMeta(node.try_into()?),
            Rule::call => Self::Call(node.try_into()?),
            Rule::scatter => Self::Scatter(node.try_into()?),
            Rule::conditional => Self::Conditional(node.try_into()?),
            _ => bail!("Invalid task element {:?}", node),
        };
        Ok(e)
    }
}

impl<'a> TryFrom<PestNode<'a>> for Workflow {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_string_ctx()?,
            body: inner.collect_ctxs()?,
        })
    }
}
