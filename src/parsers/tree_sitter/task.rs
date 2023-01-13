use crate::{
    ast::{
        BoundDeclaration, Command, Input, Meta, Output, Runtime, RuntimeAttribute, Task,
        TaskElement,
    },
    parsers::tree_sitter::{syntax, TSNode},
};
use anyhow::{bail, Error, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<TSNode<'a>> for Command {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            parts: node.field_child_nodes(syntax::PARTS)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for RuntimeAttribute {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            name: node.field_string_node(syntax::NAME)?,
            expression: node.field_node(syntax::EXPRESSION)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Runtime {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            attributes: node.field_child_nodes(syntax::ATTRIBUTES)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for TaskElement {
    type Error = Error;

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        let element = match node.kind() {
            syntax::INPUT => Self::Input(Input::try_from(node)?),
            syntax::OUTPUT => Self::Output(Output::try_from(node)?),
            syntax::BOUND_DECLARATION => Self::Declaration(BoundDeclaration::try_from(node)?),
            syntax::COMMAND => Self::Command(Command::try_from(node)?),
            syntax::RUNTIME => Self::Runtime(Runtime::try_from(node)?),
            syntax::META => Self::Meta(Meta::try_from(node)?),
            syntax::PARAMETER_META => Self::Meta(Meta::try_from(node)?),
            _ => bail!("Invalid Task element {:?}", node),
        };
        Ok(element)
    }
}

impl<'a> TryFrom<TSNode<'a>> for Task {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            name: node.field_string_node(syntax::NAME)?,
            body: node.field_child_nodes(syntax::BODY)?,
        })
    }
}
