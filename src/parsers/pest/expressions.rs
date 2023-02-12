use crate::{
    model::{
        Access, AccessOperation, Anchor, Apply, ArrayLiteral, Binary, BinaryOperator, Expression,
        MapEntry, MapLiteral, ModelError, ObjectField, ObjectLiteral, PairLiteral, Span,
        StringLiteral, StringPart, Ternary, Unary, UnaryOperator,
    },
    parsers::pest::{node::PestNode, Rule},
};
use error_stack::{bail, Report, Result};
use std::{convert::TryFrom, str::FromStr};

impl<'a> TryFrom<PestNode<'a>> for StringPart {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let part = match node.as_rule() {
            Rule::squote_literal
            | Rule::dquote_literal
            | Rule::single_line_command_block_literal
            | Rule::multi_line_command_block_literal
            | Rule::command_heredoc_literal => Self::Literal(node.try_into()?),
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
            left: inner.next_node()?.try_into_boxed_anchor()?,
            right: inner.next_node()?.try_into_boxed_anchor()?,
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
            Rule::field => Self::Field(node.one_inner()?.try_into()?),
            _ => bail!(ModelError::parser(format!(
                "Invalid access operation {:?}",
                node
            ))),
        };
        Ok(op)
    }
}

impl<'a> TryFrom<PestNode<'a>> for Ternary {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            condition: inner.next_node()?.try_into_boxed_anchor()?,
            true_branch: inner.next_node()?.try_into_boxed_anchor()?,
            false_branch: inner.next_node()?.try_into_boxed_anchor()?,
        })
    }
}

fn try_node_into_binary<'a>(node: PestNode<'a>) -> Result<Expression, ModelError> {
    let mut inner = node.into_inner();
    let first = inner.next_node()?;
    let mut bin = if let Some(node) = inner.next() {
        Binary {
            operator: node?.try_into()?,
            left: first.try_into_boxed_anchor()?,
            right: inner.next_node()?.try_into_boxed_anchor()?,
        }
    } else {
        return first.try_into();
    };
    while let Some(node) = inner.next() {
        let span = Span::from_range(&bin.left.span, &bin.right.span);
        bin = Binary {
            operator: node?.try_into()?,
            left: Box::new(Anchor {
                element: Expression::Binary(bin),
                span,
            }),
            right: inner.next_node()?.try_into_boxed_anchor()?,
        }
    }
    Ok(Expression::Binary(bin))
}

fn try_node_into_unary<'a>(node: PestNode<'a>) -> Result<Expression, ModelError> {
    let mut inner = node.into_inner();
    let first = inner.next_node()?;
    let operator = match first.as_rule() {
        Rule::pos => UnaryOperator::Pos,
        Rule::neg => UnaryOperator::Neg,
        Rule::not => UnaryOperator::Not,
        _ => return first.try_into(),
    };
    Ok(Expression::Unary(Unary {
        operator,
        expression: inner.next_node()?.try_into_boxed_anchor()?,
    }))
}

fn try_node_into_access<'a>(node: PestNode<'a>) -> Result<Expression, ModelError> {
    let mut inner = node.into_inner();
    let first = inner.next_node()?;
    let accesses: Vec<Anchor<AccessOperation>> = inner.collect_anchors()?;
    if accesses.is_empty() {
        first.try_into()
    } else {
        Ok(Expression::Access(Access {
            collection: first.try_into_boxed_anchor()?,
            accesses,
        }))
    }
}

impl<'a> TryFrom<PestNode<'a>> for Expression {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let expr_node = node.one_inner()?;
        let e = match expr_node.as_rule() {
            Rule::ternary => Self::Ternary(expr_node.try_into()?),
            Rule::disjunction
            | Rule::conjunction
            | Rule::equality
            | Rule::comparison
            | Rule::math1
            | Rule::math2 => try_node_into_binary(expr_node)?,
            Rule::unary => try_node_into_unary(expr_node)?,
            Rule::access => try_node_into_access(expr_node)?,
            Rule::apply => Self::Apply(expr_node.try_into()?),
            Rule::none => Self::None,
            Rule::boolean => Self::Boolean(expr_node.try_into()?),
            Rule::hex_int | Rule::oct_int | Rule::dec_int => Self::Int(expr_node.try_into()?),
            Rule::float => Self::Float(expr_node.try_into()?),
            Rule::string => Self::String(expr_node.try_into()?),
            Rule::array => Self::Array(expr_node.try_into()?),
            Rule::map => Self::Map(expr_node.try_into()?),
            Rule::pair => Self::Pair(expr_node.try_into()?),
            Rule::object => Self::Object(expr_node.try_into()?),
            Rule::identifier => Self::Identifier(expr_node.try_into()?),
            Rule::group => Self::Group(expr_node.one_inner()?.try_into_boxed_anchor()?),
            _ => bail!(ModelError::parser(format!(
                "Invalid expression {:?}",
                expr_node
            ))),
        };
        Ok(e)
    }
}
