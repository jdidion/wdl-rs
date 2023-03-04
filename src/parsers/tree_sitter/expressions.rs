use crate::{
    model::{
        Access, AccessOperation, Anchor, Apply, ArrayLiteral, Binary, BinaryOperator, Expression,
        MapEntry, MapLiteral, ModelError, ObjectField, ObjectLiteral, PairLiteral, StringLiteral,
        StringPart, Ternary, Unary, UnaryOperator,
    },
    parsers::tree_sitter::{
        node::{BlockDelim, BlockEnds, TSNode},
        syntax::{fields, keywords, rules, symbols},
    },
};
use error_stack::{bail, Report, Result};
use std::{convert::TryFrom, str::FromStr};

impl<'a> TryFrom<TSNode<'a>> for StringPart {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let part = match node.kind() {
            rules::CONTENT => Self::Content(node.try_into()?),
            rules::ESCAPE_SEQUENCE => Self::Escape(node.try_into()?),
            rules::PLACEHOLDER => {
                let mut children = node.into_children();
                let _ = children.next_node()?.try_as_str()?;
                Self::Placeholder(children.next_field(fields::EXPRESSION)?.try_into()?)
            }
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
        let mut children = node.into_block(BlockEnds::Quotes, BlockDelim::None);
        let parts = match children.get_next_field(fields::PARTS)? {
            Some(parts) => parts.into_children().collect_anchors()?,
            None => Vec::new(),
        };
        Ok(Self { parts })
    }
}

impl<'a> TryFrom<TSNode<'a>> for ArrayLiteral {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        let elements = children
            .next_field(fields::ELEMENTS)?
            .into_block(BlockEnds::Brackets, BlockDelim::Comma)
            .collect_anchors()?;
        Ok(Self { elements })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MapEntry {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        let key = children.next_field(fields::KEY).try_into()?;
        children.skip_terminal(symbols::COLON)?;
        let value = children.next_field(fields::VALUE).try_into()?;
        Ok(Self { key, value })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MapLiteral {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        let entries = children
            .next_field(fields::ENTRIES)?
            .into_block(BlockEnds::Braces, BlockDelim::Comma)
            .collect_anchors()?;
        Ok(Self { entries })
    }
}

impl<'a> TryFrom<TSNode<'a>> for PairLiteral {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        children.skip_terminal(symbols::LPAREN)?;
        let left = children.next_field(fields::LEFT)?.try_into_boxed_anchor()?;
        children.skip_terminal(symbols::COMMA)?;
        let right = children
            .next_field(fields::RIGHT)?
            .try_into_boxed_anchor()?;
        Ok(Self { left, right })
    }
}

impl<'a> TryFrom<TSNode<'a>> for ObjectField {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        let name = children.next_field(fields::NAME).try_into()?;
        children.skip_terminal(symbols::COLON)?;
        let expression = children.next_field(fields::EXPRESSION).try_into()?;
        Ok(Self { name, expression })
    }
}

impl<'a> TryFrom<TSNode<'a>> for ObjectLiteral {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let mut children = node.into_children();
        let type_name = children.next_field(fields::TYPE).try_into()?;
        let fields = children
            .next_field(fields::FIELDS)?
            .into_block(BlockEnds::Braces, BlockDelim::Comma)
            .collect_anchors()?;
        Ok(Self { type_name, fields })
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
        let operator = children.next_field(fields::OPERATOR)?.try_into()?;
        let expression = children
            .next_field(fields::EXPRESSION)?
            .try_into_boxed_anchor()?;
        Ok(Self {
            operator,
            expression,
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
        let left = children.next_field(fields::LEFT)?.try_into_boxed_anchor()?;
        let operator = children.next_field(fields::OPERATOR)?.try_into()?;
        let right = children
            .next_field(fields::RIGHT)?
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
        let name = children.next_field(fields::NAME).try_into()?;
        let arguments = children
            .next_field(fields::ARGUMENTS)?
            .into_block(BlockEnds::Parens, BlockDelim::Comma)
            .collect_anchors()?;
        Ok(Self { name, arguments })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Access {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        match node.kind() {
            rules::INDEX_EXPRESSION => {
                let mut children = node.into_children();
                let collection = children
                    .next_field(fields::COLLECTION)?
                    .try_into_boxed_anchor()?;
                children.skip_terminal(symbols::LBRACK)?;
                let index = children.next_field(fields::INDEX)?;
                let index_span = index.as_span();
                Ok(Self {
                    collection,
                    accesses: vec![Anchor::new(
                        AccessOperation::Index(index.try_into()?),
                        index_span,
                    )],
                })
            }
            rules::FIELD_EXPRESSION => {
                let mut children = node.into_children();
                let collection = children
                    .next_field(fields::OBJECT)?
                    .try_into_boxed_anchor()?;
                children.skip_terminal(symbols::DOT)?;
                let field = children.next_field(fields::NAME)?;
                let field_span = field.as_span();
                Ok(Self {
                    collection,
                    accesses: vec![Anchor::new(
                        AccessOperation::Field(field.try_into()?),
                        field_span,
                    )],
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
        children.skip_terminal(keywords::IF)?;
        let condition = children
            .next_field(fields::CONDITION)?
            .try_into_boxed_anchor()?;
        children.skip_terminal(keywords::THEN)?;
        let true_branch = children.next_field(fields::TRUE)?.try_into_boxed_anchor()?;
        children.skip_terminal(keywords::ELSE)?;
        let false_branch = children
            .next_field(fields::FALSE)?
            .try_into_boxed_anchor()?;
        Ok(Self {
            condition,
            true_branch,
            false_branch,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Expression {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let e = match node.kind() {
            rules::NONE => Self::None,
            rules::TRUE => Self::Boolean(true),
            rules::FALSE => Self::Boolean(false),
            rules::DEC_INT | rules::OCT_INT | rules::HEX_INT => Self::Int(node.try_into()?),
            rules::FLOAT => Self::Float(node.try_into()?),
            rules::STRING => Self::String(node.try_into()?),
            rules::ARRAY => Self::Array(node.try_into()?),
            rules::MAP => Self::Map(node.try_into()?),
            rules::PAIR => Self::Pair(node.try_into()?),
            rules::OBJECT => Self::Object(node.try_into()?),
            rules::UNARY_OPERATOR | rules::NOT_OPERATOR => Self::Unary(node.try_into()?),
            rules::BINARY_OPERATOR
            | rules::AND_OPERATOR
            | rules::OR_OPERATOR
            | rules::COMPARISON_OPERATOR => Self::Binary(node.try_into()?),
            rules::APPLY_EXPRESSION => Self::Apply(node.try_into()?),
            rules::INDEX_EXPRESSION | rules::FIELD_EXPRESSION => Self::Access(node.try_into()?),
            rules::TERNARY_EXPRESSION => Self::Ternary(node.try_into()?),
            rules::GROUP_EXPRESSION => Self::Group(
                node.into_block(BlockEnds::Parens, BlockDelim::None)
                    .next_field(fields::EXPRESSION)?
                    .try_into_boxed_anchor()?,
            ),
            rules::IDENTIFIER => Self::Identifier(node.try_into()?),
            _ => bail!(ModelError::parser(format!("Invalid expression {:?}", node))),
        };
        Ok(e)
    }
}
