use crate::{
    model::{
        BoundDeclaration, Input, InputDeclaration, ModelError, Output, Type, UnboundDeclaration,
    },
    parsers::tree_sitter::{
        node::TSNode,
        syntax::{fields, keywords, rules, symbols},
    },
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

use super::node::TSIteratorExt;

impl<'a> TryFrom<TSNode<'a>> for Type {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let t = match node.kind() {
            rules::BOOLEAN_TYPE => Self::Boolean,
            rules::INT_TYPE => Self::Int,
            rules::FLOAT_TYPE => Self::Float,
            rules::STRING_TYPE => Self::String,
            rules::FILE_TYPE => Self::File,
            rules::OBJECT_TYPE => Self::Object,
            rules::ARRAY_TYPE => {
                let mut children = node.into_children();
                children.skip_terminal(keywords::ARRAY)?;
                children.skip_terminal(symbols::LBRACK)?;
                let item = children.next_field(fields::TYPE)?.try_into_boxed_anchor()?;
                children.skip_terminal(symbols::RBRACK)?;
                let non_empty = children.get_next_field(fields::NONEMPTY)?.is_some();
                Self::Array { item, non_empty }
            }
            rules::MAP_TYPE => {
                let mut children = node.into_children();
                children.skip_terminal(keywords::MAP)?;
                let key = children.next_field(fields::KEY)?.try_into_boxed_anchor()?;
                children.skip_terminal(symbols::COMMA)?;
                let value = children
                    .next_field(fields::VALUE)?
                    .try_into_boxed_anchor()?;
                Self::Map { key, value }
            }
            rules::PAIR_TYPE => {
                let mut children = node.into_children();
                children.skip_terminal(keywords::PAIR)?;
                let left = children.next_field(fields::LEFT)?.try_into_boxed_anchor()?;
                children.skip_terminal(symbols::COMMA)?;
                let right = children
                    .next_field(fields::RIGHT)?
                    .try_into_boxed_anchor()?;
                Self::Pair { left, right }
            }
            rules::USER_TYPE => Self::User(node.try_into_child_field(fields::NAME)?.try_into()?),
            rules::OPTIONAL_TYPE => Self::Optional(
                node.try_into_child_field(fields::TYPE)?
                    .try_into_boxed_anchor()?,
            ),
            _ => bail!(ModelError::parser(format!("Invalid type {:?}", node))),
        };
        Ok(t)
    }
}

impl<'a> TryFrom<TSNode<'a>> for UnboundDeclaration {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        println!("unbound declaration");
        let mut children = node.into_children();
        Ok(Self {
            wdl_type: children.next_field(fields::TYPE).try_into()?,
            name: children.next_field(fields::NAME).try_into()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for BoundDeclaration {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        let wdl_type = children.next_field(fields::TYPE).try_into()?;
        let name = children.next_field(fields::NAME).try_into()?;
        children.skip_terminal(symbols::ASSIGN)?;
        let expression = children.next_field(fields::EXPRESSION).try_into()?;
        Ok(Self {
            wdl_type,
            name,
            expression,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for InputDeclaration {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let decl = match node.kind() {
            rules::BOUND_DECLARATION => Self::Bound(node.try_into()?),
            rules::UNBOUND_DECLARATION => Self::Unbound(node.try_into()?),
            _ => bail!(ModelError::parser(format!(
                "Invalid declaration {:?}",
                node
            ))),
        };
        Ok(decl)
    }
}

impl<'a> TryFrom<TSNode<'a>> for Input {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        children.skip_terminal(keywords::INPUT)?;
        Ok(Self {
            declarations: children
                .next_field(fields::DECLARATIONS)?
                .into_block(symbols::LBRACE, symbols::RBRACE)
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Output {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        children.skip_terminal(keywords::OUTPUT)?;
        Ok(Self {
            declarations: children
                .next_field(fields::DECLARATIONS)?
                .into_block(symbols::LBRACE, symbols::RBRACE)
                .collect_anchors()?,
        })
    }
}
