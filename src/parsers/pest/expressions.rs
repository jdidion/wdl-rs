use crate::{
    model::{
        Access, AccessOperation, Anchor, Apply, ArrayLiteral, Binary, BinaryOperator, Expression,
        InnerSpan, MapEntry, MapLiteral, ModelError, ObjectField, ObjectLiteral, PairLiteral, Span,
        StringLiteral, StringPart, Ternary, Unary, UnaryOperator,
    },
    parsers::pest::{
        node::{PestNode, PestNodes},
        Rule,
    },
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<PestNode<'a>> for StringPart {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let part = match node.as_rule() {
            Rule::squote_literal
            | Rule::dquote_literal
            | Rule::single_line_command_block_literal
            | Rule::multi_line_command_block_literal
            | Rule::command_heredoc_literal => Self::Content(node.try_into()?),
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
        Ok(Self {
            parts: node.into_inner().collect_anchors()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for ArrayLiteral {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let elements: Result<Vec<Anchor<Expression>>, ModelError> = node
            .into_inner()
            .collect_nodes()?
            .into_iter()
            .map(|node| try_into_expression_anchor(node))
            .collect();
        Ok(Self {
            elements: elements?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for MapEntry {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            key: inner.next_node().try_into()?,
            value: try_into_expression_anchor(inner.next_node()?)?,
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
            left: Box::new(try_into_expression_anchor(inner.next_node()?)?),
            right: Box::new(try_into_expression_anchor(inner.next_node()?)?),
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for ObjectField {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_node().try_into()?,
            expression: try_into_expression_anchor(inner.next_node()?)?,
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

impl<'a> TryFrom<PestNode<'a>> for UnaryOperator {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let oper = match node.as_rule() {
            Rule::pos => UnaryOperator::Pos,
            Rule::neg => UnaryOperator::Neg,
            Rule::not => UnaryOperator::Not,
            _ => bail!(ModelError::Grammar {
                kind: String::from("unary operator"),
                value: node.as_str().to_owned()
            }),
        };
        Ok(oper)
    }
}

impl<'a> TryFrom<PestNode<'a>> for BinaryOperator {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let oper = match node.as_rule() {
            Rule::add => Self::Add,
            Rule::sub => Self::Sub,
            Rule::mul => Self::Mul,
            Rule::div => Self::Div,
            Rule::rem => Self::Mod,
            Rule::gt => Self::Gt,
            Rule::lt => Self::Lt,
            Rule::gte => Self::Gte,
            Rule::lte => Self::Lte,
            Rule::eq => Self::Eq,
            Rule::neq => Self::Neq,
            Rule::and => Self::And,
            Rule::or => Self::Or,
            _ => bail!(ModelError::Grammar {
                kind: String::from("binary operator"),
                value: node.as_str().to_owned()
            }),
        };
        Ok(oper)
    }
}

impl<'a> TryFrom<PestNode<'a>> for Apply {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        let name = inner.next_node().try_into()?;
        let arguments: Result<Vec<Anchor<Expression>>, ModelError> = inner
            .collect_nodes()?
            .into_iter()
            .map(|node| try_into_expression_anchor(node))
            .collect();
        Ok(Self {
            name,
            arguments: arguments?,
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
            condition: Box::new(try_into_expression_anchor(inner.next_node()?)?),
            true_branch: Box::new(try_into_expression_anchor(inner.next_node()?)?),
            false_branch: Box::new(try_into_expression_anchor(inner.next_node()?)?),
        })
    }
}

fn try_into_unary<'a>(first: PestNode<'a>, second: PestNode<'a>) -> Result<Expression, ModelError> {
    let operator = first.try_into()?;
    Ok(Expression::Unary(Unary {
        operator,
        expression: Box::new(try_into_expression_anchor(second)?),
    }))
}

fn try_into_binary<'a>(
    first: PestNode<'a>,
    mut rest: PestNodes<'a>,
) -> Result<Expression, ModelError> {
    let operator = rest.next_node()?.try_into()?;
    let mut bin = Binary {
        operator,
        left: Box::new(try_into_expression_anchor(first)?),
        right: Box::new(try_into_expression_anchor(rest.next_node()?)?),
    };
    while let Some(node) = rest.next() {
        let span = Span::from_range(&bin.left.span, &bin.right.span);
        bin = Binary {
            operator: node?.try_into()?,
            left: Box::new(Anchor::new(Expression::Binary(bin), span)),
            right: Box::new(try_into_expression_anchor(rest.next_node()?)?),
        }
    }
    Ok(Expression::Binary(bin))
}

fn try_into_access_operation_anchor<'a>(
    node: PestNode<'a>,
) -> Result<Anchor<AccessOperation>, ModelError> {
    let op = match node.as_rule() {
        Rule::index => {
            let inner = node.one_inner()?;
            let inner_span = inner.as_span();
            Anchor::new(
                AccessOperation::Index(Expression::try_from(inner)?),
                inner_span,
            )
        }
        Rule::field => {
            let inner = node.one_inner()?;
            let inner_span = inner.as_span();
            Anchor::new(AccessOperation::Field(inner.try_into()?), inner_span)
        }
        _ => bail!(ModelError::parser(format!(
            "Invalid access operation {:?}",
            node
        ))),
    };
    Ok(op)
}

fn try_into_access<'a>(first: PestNode<'a>, rest: PestNodes<'a>) -> Result<Expression, ModelError> {
    let collection = try_into_expression_anchor(first)?;
    let accesses: Result<Vec<Anchor<AccessOperation>>, ModelError> = rest
        .map(|res| res.and_then(|node| try_into_access_operation_anchor(node)))
        .collect();
    Ok(Expression::Access(Access {
        collection: Box::new(collection),
        accesses: accesses?,
    }))
}

/// Creates an `Anchor<Expression>` with the correct span. The span reported by the pest `Node` for
/// `unary`, `binary`, and `access` rules includes trailing whitespace, so this function instead
/// constructs the span from the spans of the internal nodes.
pub fn try_into_expression_anchor<'a>(
    node: PestNode<'a>,
) -> Result<Anchor<Expression>, ModelError> {
    match node.as_rule() {
        Rule::expression => try_into_expression_anchor(node.one_inner()?),
        Rule::ternary => {
            let outer_span = node.as_span();
            let expression: Expression = node.try_into()?;
            let inner_span = expression.get_inner_span().unwrap();
            Ok(Anchor::new(
                expression,
                Span::from_range(&outer_span, &inner_span),
            ))
        }
        Rule::disjunction
        | Rule::conjunction
        | Rule::equality
        | Rule::comparison
        | Rule::math1
        | Rule::math2 => {
            let mut inner = node.into_inner();
            let first = inner.next_node()?;
            if inner.peek_rule().is_some() {
                let binary = try_into_binary(first, inner)?;
                let span = binary.get_inner_span().unwrap();
                Ok(Anchor::new(binary, span))
            } else {
                try_into_expression_anchor(first)
            }
        }
        Rule::unary => {
            let mut inner = node.into_inner();
            let first = inner.next_node()?;
            if inner.peek_rule().is_some() {
                let unary = try_into_unary(first, inner.next_node()?)?;
                let span = unary.get_inner_span().unwrap();
                Ok(Anchor::new(unary, span))
            } else {
                try_into_expression_anchor(first)
            }
        }
        Rule::access => {
            let mut inner = node.into_inner();
            let first = inner.next_node()?;
            if inner.peek_rule().is_some() {
                let access = try_into_access(first, inner)?;
                let span = access.get_inner_span().unwrap();
                Ok(Anchor::new(access, span))
            } else {
                try_into_expression_anchor(first)
            }
        }
        _ => node.try_into(),
    }
}

impl<'a> TryFrom<PestNode<'a>> for Expression {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let e = match node.as_rule() {
            Rule::expression => node.one_inner()?.try_into()?,
            Rule::ternary => Self::Ternary(node.try_into()?),
            Rule::disjunction
            | Rule::conjunction
            | Rule::equality
            | Rule::comparison
            | Rule::math1
            | Rule::math2 => {
                let mut inner = node.into_inner();
                let first = inner.next_node()?;
                if inner.peek_rule().is_some() {
                    try_into_binary(first, inner)?
                } else {
                    first.try_into()?
                }
            }
            Rule::unary => {
                let mut inner = node.into_inner();
                let first = inner.next_node()?;
                if inner.peek_rule().is_some() {
                    try_into_unary(first, inner.next_node()?)?
                } else {
                    first.try_into()?
                }
            }
            Rule::access => {
                let mut inner = node.into_inner();
                let first = inner.next_node()?;
                if inner.peek_rule().is_some() {
                    try_into_access(first, inner)?
                } else {
                    first.try_into()?
                }
            }
            Rule::apply => Self::Apply(node.try_into()?),
            Rule::none => Self::None,
            Rule::boolean => Self::Boolean(node.try_into()?),
            Rule::hex_int | Rule::oct_int | Rule::dec_int => Self::Int(node.try_into()?),
            Rule::float => Self::Float(node.try_into()?),
            Rule::dquote_string | Rule::squote_string => Self::String(node.try_into()?),
            Rule::array => Self::Array(node.try_into()?),
            Rule::map => Self::Map(node.try_into()?),
            Rule::pair => Self::Pair(node.try_into()?),
            Rule::object => Self::Object(node.try_into()?),
            Rule::identifier => Self::Identifier(node.try_into()?),
            Rule::group => Self::Group(node.one_inner()?.try_into_boxed_anchor()?),
            _ => bail!(ModelError::parser(format!("Invalid expression {:?}", node))),
        };
        Ok(e)
    }
}
