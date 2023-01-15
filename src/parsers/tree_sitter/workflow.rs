use crate::{
    model::{
        Call, CallInput, Conditional, QualifiedName, Scatter, Workflow, WorkflowBodyElement,
        WorkflowElement,
    },
    parsers::tree_sitter::{syntax, TSNode},
};
use anyhow::{bail, Error, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<TSNode<'a>> for QualifiedName {
    type Error = Error;

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            parts: node.child_nodes()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for CallInput {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            name: node.field_string_node(syntax::NAME)?,
            expression: node.get_field_node(syntax::EXPRESSION)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Call {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            target: node.field_node(syntax::TARGET)?,
            alias: node.get_field_node(syntax::ALIAS)?,
            inputs: node.get_field_child_nodes(syntax::INPUTS)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Scatter {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            name: node.field_string_node(syntax::NAME)?,
            expression: node.field_node(syntax::EXPRESSION)?,
            body: node.field_child_nodes(syntax::BODY)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Conditional {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            expression: node.field_node(syntax::EXPRESSION)?,
            body: node.field_child_nodes(syntax::BODY)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for WorkflowBodyElement {
    type Error = Error;

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        let element = match node.kind() {
            syntax::CALL => Self::Call(node.try_into()?),
            syntax::SCATTER => Self::Scatter(node.try_into()?),
            syntax::CONDITIONAL => Self::Conditional(node.try_into()?),
            _ => bail!("Invalid scatter/conditional block element {:?}", node),
        };
        Ok(element)
    }
}

impl<'a> TryFrom<TSNode<'a>> for WorkflowElement {
    type Error = Error;

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        let element = match node.kind() {
            syntax::INPUT => Self::Input(node.try_into()?),
            syntax::OUTPUT => Self::Output(node.try_into()?),
            syntax::BOUND_DECLARATION => Self::Declaration(node.try_into()?),
            syntax::CALL => Self::Call(node.try_into()?),
            syntax::SCATTER => Self::Scatter(node.try_into()?),
            syntax::CONDITIONAL => Self::Conditional(node.try_into()?),
            syntax::META => Self::Meta(node.try_into()?),
            syntax::PARAMETER_META => Self::Meta(node.try_into()?),
            _ => bail!("Invalid Task element {:?}", node),
        };
        Ok(element)
    }
}

impl<'a> TryFrom<TSNode<'a>> for Workflow {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            name: node.field_string_node(syntax::NAME)?,
            body: node.get_field_child_nodes(syntax::BODY)?,
        })
    }
}
