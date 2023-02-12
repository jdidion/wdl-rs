use crate::{
    model::{
        Call, CallInput, Conditional, ModelError, QualifiedName, Scatter, Workflow,
        WorkflowElement, WorkflowNestedElement,
    },
    parsers::tree_sitter::{node::TSNode, syntax},
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<TSNode<'a>> for QualifiedName {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            parts: node.into_children().collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for CallInput {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        let name = children.next_field(syntax::NAME).try_into()?;
        let expression = children.next().transpose().and_then(|opt| {
            if let Some(node) = opt {
                node.ensure_field(syntax::EXPRESSION)?;
                Ok(Some(node.try_into()?))
            } else {
                Ok(None)
            }
        })?;
        Ok(Self { name, expression })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Call {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        let target = children.next_field(syntax::TARGET).try_into()?;
        let next = children.next_node()?;
        let (alias, next) = match next.try_field()? {
            syntax::ALIAS => (Some(next.try_into()?), children.next_field(syntax::INPUTS)?),
            syntax::INPUTS => (None, next),
            other => bail!(ModelError::parser(format!("Invalid call field {}", other))),
        };
        let inputs = next.into_children().collect_anchors()?;
        Ok(Self {
            target,
            alias,
            inputs,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Scatter {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        Ok(Self {
            name: children.next_field(syntax::NAME).try_into()?,
            expression: children.next_field(syntax::EXPRESSION).try_into()?,
            body: children
                .next_field(syntax::BODY)?
                .into_children()
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Conditional {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        Ok(Self {
            expression: children.next_field(syntax::EXPRESSION).try_into()?,
            body: children
                .next_field(syntax::BODY)?
                .into_children()
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for WorkflowNestedElement {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let element = match node.kind() {
            syntax::CALL => Self::Call(node.try_into()?),
            syntax::SCATTER => Self::Scatter(node.try_into()?),
            syntax::CONDITIONAL => Self::Conditional(node.try_into()?),
            _ => bail!(ModelError::parser(format!(
                "Invalid scatter/conditional block element {:?}",
                node
            ))),
        };
        Ok(element)
    }
}

impl<'a> TryFrom<TSNode<'a>> for WorkflowElement {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let element = match node.kind() {
            syntax::INPUT => Self::Input(node.try_into()?),
            syntax::OUTPUT => Self::Output(node.try_into()?),
            syntax::BOUND_DECLARATION => Self::Declaration(node.try_into()?),
            syntax::CALL => Self::Call(node.try_into()?),
            syntax::SCATTER => Self::Scatter(node.try_into()?),
            syntax::CONDITIONAL => Self::Conditional(node.try_into()?),
            syntax::META => Self::Meta(node.try_into()?),
            syntax::PARAMETER_META => Self::Meta(node.try_into()?),
            _ => bail!(ModelError::parser(format!(
                "Invalid Task element {:?}",
                node
            ))),
        };
        Ok(element)
    }
}

impl<'a> TryFrom<TSNode<'a>> for Workflow {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        Ok(Self {
            name: children.next_field(syntax::NAME).try_into()?,
            body: children
                .next_field(syntax::BODY)?
                .into_children()
                .collect_anchors()?,
        })
    }
}
