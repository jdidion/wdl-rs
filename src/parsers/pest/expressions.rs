use crate::{
    model::{
        Access, AccessOperation, Apply, ArrayLiteral, Binary, BinaryOperator, Expression, MapEntry,
        MapLiteral, ModelError, ObjectField, ObjectLiteral, PairLiteral, StringLiteral, StringPart,
        Ternary, Unary, UnaryOperator,
    },
    parsers::pest::{
        node::{PestNode, PestNodeResultExt},
        Rule,
    },
};
use error_stack::{bail, Report, Result};
use std::{convert::TryFrom, str::FromStr};

impl<'a> TryFrom<PestNode<'a>> for StringPart {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let part = match node.as_rule() {
            Rule::squote_literal
            | Rule::dquote_literal
            | Rule::single_line_command_block_literal_sequence
            | Rule::multi_line_command_block_literal_sequence
            | Rule::command_heredoc_literal_sequence => Self::Literal(node.try_into()?),
            Rule::squote_escape_sequence
            | Rule::dquote_escape_sequence
            | Rule::command_block_escape_sequence
            | Rule::command_heredoc_escape_sequence => Self::Escape(node.try_into()?),
            Rule::tilde_placeholder | Rule::dollar_placeholder => {
                Self::Placeholder(Expression::try_from(node.one_inner()?)?)
            }
            _ => bail!(ModelError::parser(format!(
                "Invalid string part {:?}",
                node
            ))),
        };
        Ok(part)
    }
}

impl<'a> TryFrom<PestNode<'a>> for StringLiteral {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let inner = node.one_inner()?;
        Ok(Self {
            parts: inner.into_inner().collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for ArrayLiteral {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            elements: node.into_inner().collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for MapEntry {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            key: inner.next_node().try_into()?,
            value: inner.next_node().try_into()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for MapLiteral {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            entries: node.into_inner().collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for PairLiteral {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            left: inner.next_node().into_boxed_anchor()?,
            right: inner.next_node().into_boxed_anchor()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for ObjectField {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_node().try_into()?,
            expression: inner.next_node().try_into()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for ObjectLiteral {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            type_name: inner.next_node().try_into()?,
            fields: inner.collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Unary {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let rule = node.as_rule();
        let mut inner = node.into_inner();
        let operator = match rule {
            Rule::negation => {
                let sign_node = inner.next_node()?;
                match sign_node.as_rule() {
                    Rule::pos => UnaryOperator::Pos,
                    Rule::neg => UnaryOperator::Neg,
                    _ => bail!(ModelError::parser(format!(
                        "Invalid unary operator {:?}",
                        sign_node
                    ))),
                }
            }
            Rule::inversion => UnaryOperator::Not,
            _ => bail!(ModelError::parser(format!(
                "Invalid unary operator {:?}",
                rule
            ))),
        };
        Ok(Self {
            operator,
            expression: inner.next_node().into_boxed_anchor()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for BinaryOperator {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Self::from_str(node.as_str())
    }
}

impl<'a> TryFrom<PestNode<'a>> for Apply {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_node().try_into()?,
            arguments: inner.collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for AccessOperation {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let op = match node.as_rule() {
            Rule::index => Self::Index(Expression::try_from(node.one_inner()?)?),
            Rule::field => Self::Field(node.one_inner().into_string()?),
            _ => bail!(ModelError::parser(format!(
                "Invalid access operation {:?}",
                node
            ))),
        };
        Ok(op)
    }
}

impl<'a> TryFrom<PestNode<'a>> for Access {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            collection: inner.next_node().into_boxed_anchor()?,
            accesses: inner.collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Ternary {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            condition: inner.next_node().into_boxed_anchor()?,
            true_branch: inner.next_node().into_boxed_anchor()?,
            false_branch: inner.next_node().into_boxed_anchor()?,
        })
    }
}

fn node_to_binary<'a>(
    node: PestNode<'a>,
    operator: BinaryOperator,
) -> Result<Expression, ModelError> {
    let operands = node.into_inner().collect_anchors()?;
    if operands.len() == 1 {
        Ok(operands.into_iter().next().unwrap().element)
    } else {
        Ok(Expression::Binary(Binary { operator, operands }))
    }
}

impl<'a> TryFrom<PestNode<'a>> for Expression {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let expr_node = node.one_inner()?;
        let e = match expr_node.as_rule() {
            Rule::ternary => Self::Ternary(expr_node.try_into()?),
            Rule::disjunction => node_to_binary(expr_node, BinaryOperator::Or)?,
            Rule::conjunction => node_to_binary(expr_node, BinaryOperator::And)?,
            Rule::equal => node_to_binary(expr_node, BinaryOperator::Eq)?,
            Rule::not_equal => node_to_binary(expr_node, BinaryOperator::Neq)?,
            Rule::greater_than_or_equal => node_to_binary(expr_node, BinaryOperator::Gte)?,
            Rule::less_than_or_equal => node_to_binary(expr_node, BinaryOperator::Lte)?,
            Rule::greater_than => node_to_binary(expr_node, BinaryOperator::Gt)?,
            Rule::less_than => node_to_binary(expr_node, BinaryOperator::Lt)?,
            Rule::addition => node_to_binary(expr_node, BinaryOperator::Add)?,
            Rule::subtraction => node_to_binary(expr_node, BinaryOperator::Sub)?,
            Rule::multiplication => node_to_binary(expr_node, BinaryOperator::Mul)?,
            Rule::division => node_to_binary(expr_node, BinaryOperator::Div)?,
            Rule::remainder => node_to_binary(expr_node, BinaryOperator::Mod)?,
            Rule::negation | Rule::inversion => Self::Unary(expr_node.try_into()?),
            Rule::apply => Self::Apply(expr_node.try_into()?),
            Rule::access => Self::Access(expr_node.try_into()?),
            Rule::none => Self::None,
            Rule::true_literal => Self::Boolean(true),
            Rule::false_literal => Self::Boolean(false),
            Rule::int => Self::Int(expr_node.try_into()?),
            Rule::float => Self::Float(expr_node.try_into()?),
            Rule::string => Self::String(expr_node.try_into()?),
            Rule::array => Self::Array(expr_node.try_into()?),
            Rule::map => Self::Map(expr_node.try_into()?),
            Rule::pair => Self::Pair(expr_node.try_into()?),
            Rule::object => Self::Object(expr_node.try_into()?),
            Rule::identifier => Self::Identifier(expr_node.try_into()?),
            Rule::group => Self::Group(expr_node.one_inner().into_boxed_anchor()?),
            _ => bail!(ModelError::parser(format!(
                "Invalid expression {:?}",
                expr_node
            ))),
        };
        Ok(e)
    }
}
