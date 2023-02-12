use crate::{
    model::{
        BoundDeclaration, Input, InputDeclaration, ModelError, Output, Type, UnboundDeclaration,
    },
    parsers::tree_sitter::{node::TSNode, syntax},
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<TSNode<'a>> for Type {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let t = match node.kind() {
            syntax::BOOLEAN_TYPE => Self::Boolean,
            syntax::INT_TYPE => Self::Int,
            syntax::FLOAT_TYPE => Self::Float,
            syntax::STRING_TYPE => Self::String,
            syntax::FILE_TYPE => Self::File,
            syntax::OBJECT_TYPE => Self::Object,
            syntax::ARRAY_TYPE => {
                let mut children = node.into_children();
                let item = children.next_field(syntax::TYPE)?.try_into_boxed_anchor()?;
                let non_empty = children.get_next_field(syntax::NONEMPTY)?.is_some();
                Self::Array { item, non_empty }
            }
            syntax::MAP_TYPE => {
                let mut children = node.into_children();
                Self::Map {
                    key: children.next_field(syntax::KEY)?.try_into_boxed_anchor()?,
                    value: children
                        .next_field(syntax::VALUE)?
                        .try_into_boxed_anchor()?,
                }
            }
            syntax::PAIR_TYPE => {
                let mut children = node.into_children();
                Self::Pair {
                    left: children.next_field(syntax::LEFT)?.try_into_boxed_anchor()?,
                    right: children
                        .next_field(syntax::RIGHT)?
                        .try_into_boxed_anchor()?,
                }
            }
            syntax::USER_TYPE => Self::User(node.try_into_child_field(syntax::NAME)?.try_into()?),
            syntax::OPTIONAL_TYPE => Self::Optional(
                node.try_into_child_field(syntax::TYPE)?
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
        let mut children = node.into_children();
        Ok(Self {
            wdl_type: children.next_field(syntax::TYPE).try_into()?,
            name: children.next_field(syntax::NAME).try_into()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for BoundDeclaration {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        Ok(Self {
            wdl_type: children.next_field(syntax::TYPE).try_into()?,
            name: children.next_field(syntax::NAME).try_into()?,
            expression: children.next_field(syntax::EXPRESSION).try_into()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for InputDeclaration {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let decl = match node.kind() {
            syntax::BOUND_DECLARATION => Self::Bound(node.try_into()?),
            syntax::UNBOUND_DECLARATION => Self::Unbound(node.try_into()?),
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
        Ok(Self {
            declarations: node
                .try_into_child_field(syntax::DECLARATIONS)?
                .into_children()
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Output {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            declarations: node
                .try_into_child_field(syntax::DECLARATIONS)?
                .into_children()
                .collect_anchors()?,
        })
    }
}
