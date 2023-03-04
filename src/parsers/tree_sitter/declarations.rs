use crate::{
    model::{
        BoundDeclaration, Input, InputDeclaration, ModelError, Output, Type, UnboundDeclaration,
    },
    parsers::tree_sitter::{
        node::{BlockDelim, BlockEnds, TSNode},
        syntax::{fields, keywords, rules, symbols},
    },
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

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
                children.skip_terminal(symbols::LBRACK)?;
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
                children.skip_terminal(symbols::LPAREN)?;
                let left = children.next_field(fields::LEFT)?.try_into_boxed_anchor()?;
                children.skip_terminal(symbols::COMMA)?;
                let right = children
                    .next_field(fields::RIGHT)?
                    .try_into_boxed_anchor()?;
                Self::Pair { left, right }
            }
            rules::USER_TYPE => {
                let mut children = node.into_children();
                Self::User(children.next_field(fields::NAME)?.try_into()?)
            }
            rules::OPTIONAL_TYPE => {
                let mut children = node.into_children();
                let wdl_type = children.next_field(fields::TYPE)?.try_into_boxed_anchor()?;
                children.skip_terminal(symbols::OPTIONAL)?;
                Self::Optional(wdl_type)
            }
            _ => bail!(ModelError::parser(format!("Invalid type {:?}", node))),
        };
        Ok(t)
    }
}

impl<'a> TryFrom<TSNode<'a>> for UnboundDeclaration {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        let wdl_type = children.next_field(fields::TYPE).try_into()?;
        let name = children.next_field(fields::NAME).try_into()?;
        Ok(Self {
            type_: wdl_type,
            name,
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
            type_: wdl_type,
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
        let declarations = children
            .next_field(fields::DECLARATIONS)?
            .into_block(BlockEnds::Braces, BlockDelim::None)
            .collect_anchors()?;
        Ok(Self { declarations })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Output {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        children.skip_terminal(keywords::OUTPUT)?;
        let declarations = children
            .next_field(fields::DECLARATIONS)?
            .into_block(BlockEnds::Braces, BlockDelim::None)
            .collect_anchors()?;
        Ok(Self { declarations })
    }
}
