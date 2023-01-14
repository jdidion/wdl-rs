use crate::{
    ast::{
        Access, AccessOperation, Apply, ArrayLiteral, Binary, BinaryOperator, Expression, MapEntry,
        MapLiteral, Node, ObjectField, ObjectLiteral, PairLiteral, StringLiteral, StringPart,
        Ternary, Unary, UnaryOperator,
    },
    parsers::tree_sitter::{syntax, TSNode},
};
use anyhow::{bail, Error, Result};
use std::{convert::TryFrom, str::FromStr};

impl<'a> TryFrom<TSNode<'a>> for StringPart {
    type Error = Error;

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        let part = match node.kind() {
            syntax::CONTENT => Self::Literal(node.try_into()?),
            syntax::ESCAPE_SEQUENCE => Self::Escape(node.try_into()?),
            syntax::PLACEHOLDER => Self::Placeholder(node.try_into()?),
            _ => bail!("Invalid string part {:?}", node),
        };
        Ok(part)
    }
}

impl<'a> TryFrom<TSNode<'a>> for StringLiteral {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            parts: node.get_field_child_nodes(syntax::PARTS)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for ArrayLiteral {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            elements: node.field_child_nodes(syntax::ELEMENTS)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MapEntry {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            key: node.field_node(syntax::KEY)?,
            value: node.field_node(syntax::VALUE)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for MapLiteral {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            entries: node.field_child_nodes(syntax::ENTRIES)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for PairLiteral {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            left: node.field_boxed_node(syntax::LEFT)?,
            right: node.field_boxed_node(syntax::RIGHT)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for ObjectField {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            name: node.field_string_node(syntax::NAME)?,
            expression: node.field_node(syntax::EXPRESSION)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for ObjectLiteral {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            type_name: node.field_string_node(syntax::TYPE)?,
            fields: node.field_child_nodes(syntax::FIELDS)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for UnaryOperator {
    type Error = Error;

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        Self::from_str(node.try_as_str()?)
    }
}

impl<'a> TryFrom<TSNode<'a>> for Unary {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        let operator_field = node.field(syntax::OPERATOR)?;
        let operator = operator_field.try_into()?;
        let expression = node.field_boxed_node(syntax::EXPRESSION)?;
        Ok(Self {
            operator,
            expression,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for BinaryOperator {
    type Error = Error;

    fn try_from(node: TSNode<'a>) -> Result<Self> {
        Self::from_str(node.try_as_str()?)
    }
}

impl<'a> TryFrom<TSNode<'a>> for Binary {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        let left = node.field_node(syntax::LEFT)?;
        let operator_field = node.field(syntax::OPERATOR)?;
        let operator = operator_field.try_into()?;
        let right = node.field_node(syntax::RIGHT)?;
        Ok(Self {
            operator,
            operands: vec![left, right],
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Apply {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            name: node.field_string_node(syntax::NAME)?,
            arguments: node.field_child_nodes(syntax::ARGUMENTS)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Access {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        match node.kind() {
            syntax::INDEX_EXPRESSION => {
                let collection = node.field_boxed_node(syntax::COLLECTION)?;
                let index_field = node.field(syntax::INDEX)?;
                let index_span = index_field.span();
                let index_operation = AccessOperation::Index(index_field.try_into()?);
                let index = Node {
                    element: index_operation,
                    span: index_span,
                };
                Ok(Self {
                    collection,
                    accesses: vec![index],
                })
            }
            syntax::FIELD_EXPRESSION => {
                let collection = node.field_boxed_node(syntax::COLLECTION)?;
                let field_field = node.field(syntax::NAME)?;
                let field_span = field_field.span();
                let field_operation = AccessOperation::Field(field_field.try_as_string()?);
                let field = Node {
                    element: field_operation,
                    span: field_span,
                };
                Ok(Self {
                    collection,
                    accesses: vec![field],
                })
            }
            _ => bail!("Invalid access {:?}", node),
        }
    }
}

impl<'a> TryFrom<TSNode<'a>> for Ternary {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
        Ok(Self {
            condition: node.field_boxed_node(syntax::CONDITION)?,
            true_branch: node.field_boxed_node(syntax::TRUE)?,
            false_branch: node.field_boxed_node(syntax::FALSE)?,
        })
    }
}

impl<'a> TryFrom<TSNode<'a>> for Expression {
    type Error = Error;

    fn try_from(mut node: TSNode<'a>) -> Result<Self> {
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
            syntax::GROUP_EXPRESSION => Self::Group(node.field_boxed_node(syntax::EXPRESSION)?),
            syntax::IDENTIFIER => Self::Identifier(node.try_as_string()?),
            _ => bail!("Invalid expression {:?}", node),
        };
        Ok(e)
    }
}
