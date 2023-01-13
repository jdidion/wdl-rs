use crate::{
    ast::{
        BoundDeclaration, Call, CallInput, Conditional, Input, Meta, Output, QualifiedName,
        Scatter, Workflow, WorkflowBodyElement, WorkflowElement,
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
        let name = node.field_string_node(syntax::NAME)?;
        let expression = node.get_field_node(syntax::EXPRESSION)?;
        Ok(Self { name, expression })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Call {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        let target = node.field_node(syntax::TARGET)?;
        let alias = node.get_field_node(syntax::ALIAS)?;
        let inputs = node.get_field_child_nodes(syntax::INPUTS)?;
        Ok(Self {
            target,
            alias,
            inputs,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Scatter {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        let name = node.field_string_node(syntax::NAME)?;
        let expression = node.field_node(syntax::EXPRESSION)?;
        let body = node.field_child_nodes(syntax::BODY)?;
        Ok(Self {
            name,
            expression,
            body,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Conditional {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        let expression = node.field_node(syntax::EXPRESSION)?;
        let body = node.field_child_nodes(syntax::BODY)?;
        Ok(Self { expression, body })
    }
}

impl<'a> TryFrom<TSNode<'a>> for WorkflowBodyElement {
    type Error = Error;

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        let element = match node.kind() {
            syntax::CALL => Self::Call(Call::try_from(node)?),
            syntax::SCATTER => Self::Scatter(Scatter::try_from(node)?),
            syntax::CONDITIONAL => Self::Conditional(Conditional::try_from(node)?),
            _ => bail!("Invalid scatter/conditional block element {:?}", node),
        };
        Ok(element)
    }
}

impl<'a> TryFrom<TSNode<'a>> for WorkflowElement {
    type Error = Error;

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        let element = match node.kind() {
            syntax::INPUT => Self::Input(Input::try_from(node)?),
            syntax::OUTPUT => Self::Output(Output::try_from(node)?),
            syntax::BOUND_DECLARATION => Self::Declaration(BoundDeclaration::try_from(node)?),
            syntax::CALL => Self::Call(Call::try_from(node)?),
            syntax::SCATTER => Self::Scatter(Scatter::try_from(node)?),
            syntax::CONDITIONAL => Self::Conditional(Conditional::try_from(node)?),
            syntax::META => Self::Meta(Meta::try_from(node)?),
            syntax::PARAMETER_META => Self::Meta(Meta::try_from(node)?),
            _ => bail!("Invalid Task element {:?}", node),
        };
        Ok(element)
    }
}

impl<'a> TryFrom<TSNode<'a>> for Workflow {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        let name = node.field_string_node(syntax::NAME)?;
        let body = node.get_field_child_nodes(syntax::BODY)?;
        Ok(Self { name, body })
    }
}
