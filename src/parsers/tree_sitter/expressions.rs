use crate::{
    model::{
        Access, AccessOperation, Anchor, Apply, ArrayLiteral, Binary, BinaryOperator, Expression,
        MapEntry, MapLiteral, ModelError, ObjectField, ObjectLiteral, PairLiteral, StringLiteral,
        StringPart, Ternary, Unary, UnaryOperator,
    },
    parsers::tree_sitter::{node::TSNode, syntax},
};
use error_stack::{bail, Report, Result};
use std::{convert::TryFrom, str::FromStr};

impl<'a> TryFrom<TSNode<'a>> for StringPart {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let part = match node.kind() {
            syntax::CONTENT => Self::Literal(node.try_into()?),
            syntax::ESCAPE_SEQUENCE => Self::Escape(node.try_into()?),
            syntax::PLACEHOLDER => Self::Placeholder(node.try_into()?),
            _ => bail!(ModelError::parser(format!(
                "Invalid string part {:?}",
                node
            ))),
        };
        Ok(part)
    }
}

impl<'a> TryFrom<TSNode<'a>> for StringLiteral {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            parts: node
                .try_into_child_field(syntax::PARTS)?
                .into_children()
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for ArrayLiteral {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            elements: node
                .try_into_child_field(syntax::ELEMENTS)?
                .into_children()
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MapEntry {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        Ok(Self {
            key: children.next_field(syntax::KEY).try_into()?,
            value: children.next_field(syntax::VALUE).try_into()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MapLiteral {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            entries: node
                .try_into_child_field(syntax::ENTRIES)?
                .into_children()
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for PairLiteral {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        Ok(Self {
            left: children.next_field(syntax::LEFT)?.try_into_boxed_anchor()?,
            right: children
                .next_field(syntax::RIGHT)?
                .try_into_boxed_anchor()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for ObjectField {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        Ok(Self {
            name: children.next_field(syntax::NAME).try_into()?,
            expression: children.next_field(syntax::EXPRESSION).try_into()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for ObjectLiteral {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        Ok(Self {
            type_name: children.next_field(syntax::TYPE).try_into()?,
            fields: children
                .next_field(syntax::FIELDS)?
                .into_children()
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for UnaryOperator {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Self::from_str(node.try_as_str()?)
    }
}

impl<'a> TryFrom<TSNode<'a>> for Unary {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        Ok(Self {
            operator: children.next_field(syntax::OPERATOR)?.try_into()?,
            expression: children
                .next_field(syntax::EXPRESSION)?
                .try_into_boxed_anchor()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for BinaryOperator {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Self::from_str(node.try_as_str()?)
    }
}

impl<'a> TryFrom<TSNode<'a>> for Binary {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        let left = children.next_field(syntax::LEFT)?.try_into_boxed_anchor()?;
        let operator = children.next_field(syntax::OPERATOR)?.try_into()?;
        let right = children
            .next_field(syntax::RIGHT)?
            .try_into_boxed_anchor()?;
        Ok(Self {
            operator,
            left,
            right,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Apply {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        Ok(Self {
            name: children.next_field(syntax::NAME).try_into()?,
            arguments: children
                .next_field(syntax::ARGUMENTS)?
                .into_children()
                .collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Access {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        match node.kind() {
            syntax::INDEX_EXPRESSION => {
                let mut children = node.into_children();
                let collection = children
                    .next_field(syntax::COLLECTION)?
                    .try_into_boxed_anchor()?;
                let index = children.next_field(syntax::INDEX)?;
                let index_span = index.as_span();
                Ok(Self {
                    collection,
                    accesses: vec![Anchor {
                        element: AccessOperation::Index(index.try_into()?),
                        span: index_span,
                    }],
                })
            }
            syntax::FIELD_EXPRESSION => {
                let mut children = node.into_children();
                let collection = children
                    .next_field(syntax::COLLECTION)?
                    .try_into_boxed_anchor()?;
                let field = children.next_field(syntax::NAME)?;
                let field_span = field.as_span();
                Ok(Self {
                    collection,
                    accesses: vec![Anchor {
                        element: AccessOperation::Field(field.try_into()?),
                        span: field_span,
                    }],
                })
            }
            _ => bail!(ModelError::parser(format!("Invalid access {:?}", node))),
        }
    }
}

impl<'a> TryFrom<TSNode<'a>> for Ternary {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        Ok(Self {
            condition: children
                .next_field(syntax::CONDITION)?
                .try_into_boxed_anchor()?,
            true_branch: children.next_field(syntax::TRUE)?.try_into_boxed_anchor()?,
            false_branch: children
                .next_field(syntax::FALSE)?
                .try_into_boxed_anchor()?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Expression {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let e = match node.kind() {
            syntax::NONE => Self::None,
            syntax::TRUE => Self::Boolean(true),
            syntax::FALSE => Self::Boolean(false),
            syntax::DEC_INT | syntax::OCT_INT | syntax::HEX_INT => Self::Int(node.try_into()?),
            syntax::FLOAT => Self::Float(node.try_into()?),
            syntax::STRING => Self::String(node.try_into()?),
            syntax::ARRAY => Self::Array(node.try_into()?),
            syntax::MAP => Self::Map(node.try_into()?),
            syntax::PAIR => Self::Pair(node.try_into()?),
            syntax::OBJECT => Self::Object(node.try_into()?),
            syntax::UNARY_OPERATOR | syntax::NOT_OPERATOR => Self::Unary(node.try_into()?),
            syntax::BINARY_OPERATOR
            | syntax::AND_OPERATOR
            | syntax::OR_OPERATOR
            | syntax::COMPARISON_OPERATOR => Self::Binary(node.try_into()?),
            syntax::APPLY_EXPRESSION => Self::Apply(node.try_into()?),
            syntax::INDEX_EXPRESSION | syntax::FIELD_EXPRESSION => Self::Access(node.try_into()?),
            syntax::TERNARY_EXPRESSION => Self::Ternary(node.try_into()?),
            syntax::GROUP_EXPRESSION => Self::Group(
                node.try_into_child_field(syntax::EXPRESSION)?
                    .try_into_boxed_anchor()?,
            ),
            syntax::IDENTIFIER => Self::Identifier(node.try_into()?),
            _ => bail!(ModelError::parser(format!("Invalid expression {:?}", node))),
        };
        Ok(e)
    }
}
