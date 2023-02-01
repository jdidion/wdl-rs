use crate::{
    model::{Float, Integer, ModelError},
    parsers::tree_sitter::node::TSNode,
};
use error_stack::{Report, Result};
use std::{convert::TryFrom, str::FromStr};

impl<'a> TryFrom<TSNode<'a>> for Integer {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Self::from_str(node.try_as_str()?)
    }
}

impl<'a> TryFrom<TSNode<'a>> for Float {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Self::from_str(node.try_as_str()?)
    }
}
