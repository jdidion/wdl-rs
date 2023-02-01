use crate::{
    model::{Float, Integer, ModelError},
    parsers::pest::node::{PestNode, PestNodeResultExt},
};
use error_stack::{Report, Result};
use std::str::FromStr;

impl<'a> TryFrom<PestNode<'a>> for Integer {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Self::from_str(node.one_inner().into_str()?)
    }
}

impl<'a> TryFrom<PestNode<'a>> for Float {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Self::from_str(node.as_str())
    }
}
