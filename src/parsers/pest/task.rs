use crate::{
    model::{Command, ModelError, Runtime, RuntimeAttribute, Task, TaskElement},
    parsers::pest::{node::PestNode, Rule},
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<PestNode<'a>> for Command {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            parts: node.into_inner().collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for RuntimeAttribute {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_node().try_into()?,
            expression: inner.next_node().try_into()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Runtime {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            attributes: node.into_inner().collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for TaskElement {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let e = match node.as_rule() {
            Rule::input => Self::Input(node.try_into()?),
            Rule::output => Self::Output(node.try_into()?),
            Rule::bound_declaration => Self::Declaration(node.try_into()?),
            Rule::meta => Self::Meta(node.try_into()?),
            Rule::parameter_meta => Self::ParameterMeta(node.try_into()?),
            Rule::command_heredoc
            | Rule::single_line_command_block
            | Rule::multi_line_command_block => Self::Command(node.try_into()?),
            Rule::runtime => Self::Runtime(node.try_into()?),
            _ => bail!(ModelError::parser(format!(
                "Invalid task element {:?}",
                node
            ))),
        };
        Ok(e)
    }
}

impl<'a> TryFrom<PestNode<'a>> for Task {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_node().try_into()?,
            body: inner.collect_anchors()?,
        })
    }
}
