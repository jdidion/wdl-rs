use crate::{
    model::{
        Call, CallInput, Conditional, ModelError, QualifiedIdentifier, Scatter, Workflow,
        WorkflowElement, WorkflowNestedElement,
    },
    parsers::tree_sitter::{
        node::{BlockDelim, BlockEnds, TSNode},
        syntax::{fields, keywords, rules, symbols},
    },
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<TSNode<'a>> for QualifiedIdentifier {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            parts: node
                .into_block(BlockEnds::None, BlockDelim::Dot)
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for CallInput {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        let name = children.next_field(fields::NAME).try_into()?;
        let expression = if children.skip_optional_last(symbols::ASSIGN)? {
            Some(children.next_field(fields::EXPRESSION)?.try_into()?)
        } else {
            None
        };
        Ok(Self { name, expression })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Call {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        children.skip_terminal(keywords::CALL)?;
        let target = children.next_field(fields::TARGET).try_into()?;
        let next = children.next().transpose()?;
        let next_field = next.as_ref().map(|node| node.get_field()).flatten();
        let (alias, next) = match next_field {
            Some(fields::ALIAS) => {
                let mut alias_children = next.unwrap().into_children();
                alias_children.skip_terminal(keywords::AS)?;
                let name = alias_children.next_field(fields::NAME)?.try_into()?;
                drop(alias_children);
                (Some(name), children.get_next_field(fields::INPUTS)?)
            }
            Some(fields::INPUTS) => (None, next),
            None => (None, None),
            other => bail!(ModelError::parser(format!(
                "Invalid call field {:?}",
                other
            ))),
        };
        let inputs = match next {
            Some(node) => {
                let mut inputs_children = node.into_block(BlockEnds::Braces, BlockDelim::None);
                if inputs_children.skip_optional_last(keywords::INPUT)? {
                    inputs_children.skip_terminal(symbols::COLON)?;
                    inputs_children.set_delim(BlockDelim::Comma)?;
                    Some(inputs_children.collect_anchors()?)
                } else {
                    Some(Vec::new())
                }
            }
            None => None,
        };
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
        children.skip_terminal(keywords::SCATTER)?;
        children.skip_terminal(symbols::LPAREN)?;
        let name = children.next_field(fields::NAME).try_into()?;
        children.skip_terminal(keywords::IN)?;
        let expression = children.next_field(fields::EXPRESSION).try_into()?;
        children.skip_terminal(symbols::RPAREN)?;
        let body = children
            .next_field(fields::BODY)?
            .into_block(BlockEnds::Braces, BlockDelim::None)
            .collect_anchors()?;
        Ok(Self {
            name,
            expression,
            body,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Conditional {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        children.skip_terminal(keywords::IF)?;
        children.skip_terminal(symbols::LPAREN)?;
        let expression = children.next_field(fields::EXPRESSION).try_into()?;
        children.skip_terminal(symbols::RPAREN)?;
        let body = children
            .next_field(fields::BODY)?
            .into_block(BlockEnds::Braces, BlockDelim::None)
            .collect_anchors()?;
        Ok(Self { expression, body })
    }
}

impl<'a> TryFrom<TSNode<'a>> for WorkflowNestedElement {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let element = match node.kind() {
            rules::BOUND_DECLARATION => Self::Declaration(node.try_into()?),
            rules::CALL => Self::Call(node.try_into()?),
            rules::SCATTER => Self::Scatter(node.try_into()?),
            rules::CONDITIONAL => Self::Conditional(node.try_into()?),
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
            rules::INPUT => Self::Input(node.try_into()?),
            rules::OUTPUT => Self::Output(node.try_into()?),
            rules::META => Self::Meta(node.try_into()?),
            rules::PARAMETER_META => Self::ParameterMeta(node.try_into()?),
            rules::BOUND_DECLARATION => Self::Declaration(node.try_into()?),
            rules::CALL => Self::Call(node.try_into()?),
            rules::SCATTER => Self::Scatter(node.try_into()?),
            rules::CONDITIONAL => Self::Conditional(node.try_into()?),
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
        children.skip_terminal(keywords::WORKFLOW)?;
        let name = children.next_field(fields::NAME).try_into()?;
        let body = children
            .next_field(fields::BODY)?
            .into_block(BlockEnds::Braces, BlockDelim::None)
            .collect_anchors()?;
        Ok(Self { name, body })
    }
}
