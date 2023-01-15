use crate::{
    model::{
        Access, AccessOperation, Apply, ArrayLiteral, Binary, BinaryOperator, Expression, MapEntry,
        MapLiteral, ObjectField, ObjectLiteral, PairLiteral, StringLiteral, StringPart, Ternary,
        Unary, UnaryOperator,
    },
    parsers::pest::{PairExt, PairsExt, Rule},
};
use anyhow::{bail, Error, Result};
use pest::iterators::Pair;
use std::{convert::TryFrom, str::FromStr};

impl<'a> TryFrom<Pair<'a, Rule>> for StringPart {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let part = match pair.as_rule() {
            Rule::squote_literal
            | Rule::dquote_literal
            | Rule::single_line_command_block_literal_sequence
            | Rule::multi_line_command_block_literal_sequence
            | Rule::command_heredoc_literal_sequence => Self::Literal(pair.as_string()),
            Rule::squote_escape_sequence
            | Rule::dquote_escape_sequence
            | Rule::command_block_escape_sequence
            | Rule::command_heredoc_escape_sequence => Self::Escape(pair.as_string()),
            Rule::tilde_placeholder | Rule::dollar_placeholder => {
                Self::Placeholder(Expression::try_from(pair.first_inner()?)?)
            }
            _ => bail!("Invalid string part {:?}", pair),
        };
        Ok(part)
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for StringLiteral {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let inner = pair.first_inner()?;
        Ok(Self {
            parts: inner.into_inner().collect_nodes()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for ArrayLiteral {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        Ok(Self {
            elements: pair.into_inner().collect_nodes()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for MapEntry {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            key: inner.next_node()?,
            value: inner.next_node()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for MapLiteral {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        Ok(Self {
            entries: pair.into_inner().collect_nodes()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for PairLiteral {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            left: inner.next_boxed_node()?,
            right: inner.next_boxed_node()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for ObjectField {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            name: inner.next_string_node()?,
            expression: inner.next_node()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for ObjectLiteral {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            type_name: inner.next_string_node()?,
            fields: inner.collect_nodes()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Unary {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();
        let operator = match rule {
            Rule::negation => {
                let sign_pair = inner.next_pair()?;
                match sign_pair.as_rule() {
                    Rule::pos => UnaryOperator::Pos,
                    Rule::neg => UnaryOperator::Neg,
                    _ => bail!("Invalid unary operator {:?}", sign_pair),
                }
            }
            Rule::inversion => UnaryOperator::Not,
            _ => bail!("Invalid unary operator {:?}", rule),
        };
        Ok(Self {
            operator,
            expression: inner.next_boxed_node()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for BinaryOperator {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        Self::from_str(pair.as_str())
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Apply {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            name: inner.next_string_node()?,
            arguments: inner.collect_nodes()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for AccessOperation {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let op = match pair.as_rule() {
            Rule::index => Self::Index(Expression::try_from(pair.first_inner()?)?),
            Rule::field => Self::Field(pair.first_inner_string()?),
            _ => bail!("Invalid access operation {:?}", pair),
        };
        Ok(op)
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Access {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            collection: inner.next_boxed_node()?,
            accesses: inner.collect_nodes()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Ternary {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            condition: inner.next_boxed_node()?,
            true_branch: inner.next_boxed_node()?,
            false_branch: inner.next_boxed_node()?,
        })
    }
}

fn pair_to_binary(pair: Pair<Rule>, operator: BinaryOperator) -> Result<Expression> {
    let operands = pair.into_inner().collect_nodes()?;
    if operands.len() == 1 {
        Ok(operands.into_iter().next().unwrap().element)
    } else {
        Ok(Expression::Binary(Binary { operator, operands }))
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Expression {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let expr_pair = pair.first_inner()?;
        let e = match expr_pair.as_rule() {
            Rule::ternary => Self::Ternary(expr_pair.try_into()?),
            Rule::disjunction => pair_to_binary(expr_pair, BinaryOperator::Or)?,
            Rule::conjunction => pair_to_binary(expr_pair, BinaryOperator::And)?,
            Rule::equal => pair_to_binary(expr_pair, BinaryOperator::Eq)?,
            Rule::not_equal => pair_to_binary(expr_pair, BinaryOperator::Neq)?,
            Rule::greater_than_or_equal => pair_to_binary(expr_pair, BinaryOperator::Gte)?,
            Rule::less_than_or_equal => pair_to_binary(expr_pair, BinaryOperator::Lte)?,
            Rule::greater_than => pair_to_binary(expr_pair, BinaryOperator::Gt)?,
            Rule::less_than => pair_to_binary(expr_pair, BinaryOperator::Lt)?,
            Rule::addition => pair_to_binary(expr_pair, BinaryOperator::Add)?,
            Rule::subtraction => pair_to_binary(expr_pair, BinaryOperator::Sub)?,
            Rule::multiplication => pair_to_binary(expr_pair, BinaryOperator::Mul)?,
            Rule::division => pair_to_binary(expr_pair, BinaryOperator::Div)?,
            Rule::remainder => pair_to_binary(expr_pair, BinaryOperator::Mod)?,
            Rule::negation | Rule::inversion => Self::Unary(expr_pair.try_into()?),
            Rule::apply => Self::Apply(expr_pair.try_into()?),
            Rule::access => Self::Access(expr_pair.try_into()?),
            Rule::none => Self::None,
            Rule::true_literal => Self::Boolean(true),
            Rule::false_literal => Self::Boolean(false),
            Rule::int => Self::Int(expr_pair.try_into()?),
            Rule::float => Self::Float(expr_pair.try_into()?),
            Rule::string => Self::String(expr_pair.try_into()?),
            Rule::array => Self::Array(expr_pair.try_into()?),
            Rule::map => Self::Map(expr_pair.try_into()?),
            Rule::pair => Self::Pair(expr_pair.try_into()?),
            Rule::object => Self::Object(expr_pair.try_into()?),
            Rule::identifier => Self::Identifier(expr_pair.as_str().to_owned()),
            Rule::group => Self::Group(expr_pair.first_inner_boxed_node()?),
            _ => bail!("Invalid expression {:?}", expr_pair),
        };
        Ok(e)
    }
}
