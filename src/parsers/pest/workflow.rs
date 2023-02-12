use crate::{
    model::{
        Call, CallInput, Conditional, ModelError, QualifiedName, Scatter, Workflow,
        WorkflowElement, WorkflowNestedElement,
    },
    parsers::pest::{node::PestNode, Rule},
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<PestNode<'a>> for QualifiedName {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            parts: node.into_inner().collect_string_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for CallInput {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        let name = inner.next_node().try_into()?;
        let expression = inner
            .next()
            .map(|expr_node| expr_node.try_into())
            .transpose()?;
        Ok(Self { name, expression })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Call {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        let target = inner.next_node().try_into()?;
        let alias = if let Some(Rule::call_alias) = inner.peek_rule() {
            let alias_node = inner.next_node()?;
            Some(alias_node.one_inner().try_into()?)
        } else {
            None
        };
        let inputs = inner.collect_anchors()?;
        Ok(Self {
            target,
            alias,
            inputs,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Scatter {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_node().try_into()?,
            expression: inner.next_node().try_into()?,
            body: inner.collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Conditional {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            expression: inner.next_node().try_into()?,
            body: inner.collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for WorkflowNestedElement {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let e = match node.as_rule() {
            Rule::bound_declaration => Self::Declaration(node.try_into()?),
            Rule::call => Self::Call(node.try_into()?),
            Rule::scatter => Self::Scatter(node.try_into()?),
            Rule::conditional => Self::Conditional(node.try_into()?),
            _ => bail!(ModelError::parser(format!(
                "Invalid task element {:?}",
                node
            ))),
        };
        Ok(e)
    }
}

impl<'a> TryFrom<PestNode<'a>> for WorkflowElement {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let e = match node.as_rule() {
            Rule::input => Self::Input(node.try_into()?),
            Rule::output => Self::Output(node.try_into()?),
            Rule::bound_declaration => Self::Declaration(node.try_into()?),
            Rule::meta => Self::Meta(node.try_into()?),
            Rule::parameter_meta => Self::ParameterMeta(node.try_into()?),
            Rule::call => Self::Call(node.try_into()?),
            Rule::scatter => Self::Scatter(node.try_into()?),
            Rule::conditional => Self::Conditional(node.try_into()?),
            _ => bail!(ModelError::parser(format!(
                "Invalid task element {:?}",
                node
            ))),
        };
        Ok(e)
    }
}

impl<'a> TryFrom<PestNode<'a>> for Workflow {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_node().try_into()?,
            body: inner.collect_anchors()?,
        })
    }
}
