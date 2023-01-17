use crate::{
    model::{
        Access, AccessOperation, Apply, ArrayLiteral, Binary, BinaryOperator, Expression, MapEntry,
        MapLiteral, ObjectField, ObjectLiteral, PairLiteral, StringLiteral, StringPart, Ternary,
        Unary, UnaryOperator,
    },
    parsers::pest::{PestNode, Rule},
};
use anyhow::{bail, Error, Result};
use std::{convert::TryFrom, str::FromStr};

impl<'a> TryFrom<PestNode<'a>> for StringPart {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let part = match node.as_rule() {
            Rule::squote_literal
            | Rule::dquote_literal
            | Rule::single_line_command_block_literal_sequence
            | Rule::multi_line_command_block_literal_sequence
            | Rule::command_heredoc_literal_sequence => Self::Literal(node.as_string()),
            Rule::squote_escape_sequence
            | Rule::dquote_escape_sequence
            | Rule::command_block_escape_sequence
            | Rule::command_heredoc_escape_sequence => Self::Escape(node.as_string()),
            Rule::tilde_placeholder | Rule::dollar_placeholder => {
                Self::Placeholder(Expression::try_from(node.first_inner()?)?)
            }
            _ => bail!("Invalid string part {:?}", node),
        };
        Ok(part)
    }
}

impl<'a> TryFrom<PestNode<'a>> for StringLiteral {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let inner = node.first_inner()?;
        Ok(Self {
            parts: inner.into_inner().collect_ctxs()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for ArrayLiteral {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        Ok(Self {
            elements: node.into_inner().collect_ctxs()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for MapEntry {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            key: inner.next_ctx()?,
            value: inner.next_ctx()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for MapLiteral {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        Ok(Self {
            entries: node.into_inner().collect_ctxs()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for PairLiteral {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            left: inner.next_boxed_ctx()?,
            right: inner.next_boxed_ctx()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for ObjectField {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_string_ctx()?,
            expression: inner.next_ctx()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for ObjectLiteral {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            type_name: inner.next_string_ctx()?,
            fields: inner.collect_ctxs()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Unary {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let rule = node.as_rule();
        let mut inner = node.into_inner();
        let operator = match rule {
            Rule::negation => {
                let sign_node = inner.next_node()?;
                match sign_node.as_rule() {
                    Rule::pos => UnaryOperator::Pos,
                    Rule::neg => UnaryOperator::Neg,
                    _ => bail!("Invalid unary operator {:?}", sign_node),
                }
            }
            Rule::inversion => UnaryOperator::Not,
            _ => bail!("Invalid unary operator {:?}", rule),
        };
        Ok(Self {
            operator,
            expression: inner.next_boxed_ctx()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for BinaryOperator {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        Self::from_str(node.as_str())
    }
}

impl<'a> TryFrom<PestNode<'a>> for Apply {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_string_ctx()?,
            arguments: inner.collect_ctxs()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for AccessOperation {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let op = match node.as_rule() {
            Rule::index => Self::Index(Expression::try_from(node.first_inner()?)?),
            Rule::field => Self::Field(node.first_inner_string()?),
            _ => bail!("Invalid access operation {:?}", node),
        };
        Ok(op)
    }
}

impl<'a> TryFrom<PestNode<'a>> for Access {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            collection: inner.next_boxed_ctx()?,
            accesses: inner.collect_ctxs()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Ternary {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let mut inner = node.into_inner();
        Ok(Self {
            condition: inner.next_boxed_ctx()?,
            true_branch: inner.next_boxed_ctx()?,
            false_branch: inner.next_boxed_ctx()?,
        })
    }
}

fn node_to_binary<'a>(node: PestNode<'a>, operator: BinaryOperator) -> Result<Expression> {
    let operands = node.into_inner().collect_ctxs()?;
    if operands.len() == 1 {
        Ok(operands.into_iter().next().unwrap().element)
    } else {
        Ok(Expression::Binary(Binary { operator, operands }))
    }
}

impl<'a> TryFrom<PestNode<'a>> for Expression {
    type Error = Error;

    fn try_from(node: PestNode<'a>) -> Result<Self> {
        let expr_node = node.first_inner()?;
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
            Rule::identifier => Self::Identifier(expr_node.as_str().to_owned()),
            Rule::group => Self::Group(expr_node.first_inner_boxed_ctx()?),
            _ => bail!("Invalid expression {:?}", expr_node),
        };
        Ok(e)
    }
}
