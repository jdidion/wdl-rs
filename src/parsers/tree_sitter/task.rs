use crate::{
    model::{Command, ModelError, Runtime, RuntimeAttribute, Task, TaskElement},
    parsers::tree_sitter::{node::TSNode, syntax},
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

use super::node::{TSNodeIteratorResultExt, TSNodeResultExt};

impl<'a> TryFrom<TSNode<'a>> for Command {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            parts: node
                .try_into_child_field(syntax::PARTS)
                .into_children()
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for RuntimeAttribute {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        Ok(Self {
            name: children.next_field(syntax::NAME).try_into()?,
            expression: children.next_field(syntax::EXPRESSION).try_into()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Runtime {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            attributes: node
                .try_into_child_field(syntax::ATTRIBUTES)
                .into_children()
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for TaskElement {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let element = match node.kind() {
            syntax::INPUT => Self::Input(node.try_into()?),
            syntax::OUTPUT => Self::Output(node.try_into()?),
            syntax::BOUND_DECLARATION => Self::Declaration(node.try_into()?),
            syntax::COMMAND => Self::Command(node.try_into()?),
            syntax::RUNTIME => Self::Runtime(node.try_into()?),
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

impl<'a> TryFrom<TSNode<'a>> for Task {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        Ok(Self {
            name: children.next_field(syntax::NAME).try_into()?,
            body: children
                .next_field(syntax::BODY)
                .into_children()
                .collect_anchors()?,
        })
    }
}
