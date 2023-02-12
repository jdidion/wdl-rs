use crate::{
    model::{Float, Integer, ModelError},
    parsers::pest::node::PestNode,
};
use error_stack::{IntoReport, Report, Result, ResultExt};
use std::str::FromStr;

impl<'a> TryFrom<PestNode<'a>> for bool {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> std::result::Result<Self, Self::Error> {
        node.as_str()
            .parse()
            .into_report()
            .change_context(ModelError::parser(format!("Invalid boolean {:?}", node)))
    }
}

impl<'a> TryFrom<PestNode<'a>> for Integer {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Self::from_str(node.as_str())
    }
}

impl<'a> TryFrom<PestNode<'a>> for Float {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Self::from_str(node.as_str())
    }
}
