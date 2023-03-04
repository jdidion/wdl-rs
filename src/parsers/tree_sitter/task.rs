use crate::{
    model::{Command, ModelError, Runtime, RuntimeAttribute, Task, TaskElement},
    parsers::tree_sitter::{
        node::{BlockDelim, BlockEnds, TSNode},
        syntax::{fields, keywords, rules, symbols},
    },
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<TSNode<'a>> for Command {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        children.skip_terminal(keywords::COMMAND)?;
        let _ = children.next_node()?.try_as_str()?;
        let parts = if let Some(parts) = children.get_next_field(fields::PARTS)? {
            parts.into_children().collect_anchors()?
        } else {
            Vec::new()
        };
        Ok(Self { parts })
    }
}

impl<'a> TryFrom<TSNode<'a>> for RuntimeAttribute {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        let name = children.next_field(fields::NAME).try_into()?;
        children.skip_terminal(symbols::COLON)?;
        let expression = children.next_field(fields::EXPRESSION).try_into()?;
        Ok(Self { name, expression })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Runtime {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        children.skip_terminal(keywords::RUNTIME)?;
        let attributes = children
            .next_field(fields::ATTRIBUTES)?
            .into_block(BlockEnds::Braces, BlockDelim::None)
            .collect_anchors()?;
        Ok(Self { attributes })
    }
}

impl<'a> TryFrom<TSNode<'a>> for TaskElement {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let element = match node.kind() {
            rules::INPUT => Self::Input(node.try_into()?),
            rules::OUTPUT => Self::Output(node.try_into()?),
            rules::META => Self::Meta(node.try_into()?),
            rules::PARAMETER_META => Self::ParameterMeta(node.try_into()?),
            rules::BOUND_DECLARATION => Self::Declaration(node.try_into()?),
            rules::COMMAND => Self::Command(node.try_into()?),
            rules::RUNTIME => Self::Runtime(node.try_into()?),
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
        children.skip_terminal(keywords::TASK)?;
        let name = children.next_field(fields::NAME).try_into()?;
        let body = children
            .next_field(fields::BODY)?
            .into_block(BlockEnds::Braces, BlockDelim::None)
            .collect_anchors()?;
        Ok(Self { name, body })
    }
}
