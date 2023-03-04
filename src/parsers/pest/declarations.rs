use crate::{
    model::{
        Anchor, BoundDeclaration, Input, InputDeclaration, ModelError, Output, Type,
        UnboundDeclaration,
    },
    parsers::pest::{expressions, node::PestNode, Rule},
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<PestNode<'a>> for Type {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let t = match node.as_rule() {
            Rule::typedef => {
                let mut inner = node.into_inner();
                let type_node = inner.next_node()?;
                match inner.next().transpose()? {
                    Some(opt_node) if opt_node.as_rule() == Rule::optional => {
                        try_into_optional(type_node)?
                    }
                    Some(node) => bail!(ModelError::parser(format!(
                        "Expected optional token '?' but found {:?}",
                        node
                    ))),
                    None => Self::try_from(type_node)?,
                }
            }
            Rule::primitive_type => match node.as_str() {
                "Boolean" => Self::Boolean,
                "Int" => Self::Int,
                "Float" => Self::Float,
                "String" => Self::String,
                "File" => Self::File,
                "Object" => Self::Object,
                _ => bail!(ModelError::parser(format!(
                    "Invalid primitive type {:?}",
                    node
                ))),
            },
            Rule::non_empty_array_type => match node.one_inner()?.try_into()? {
                Self::Array {
                    item,
                    non_empty: false,
                } => Self::Array {
                    item,
                    non_empty: true,
                },
                other => bail!(ModelError::parser(format!(
                    "Expected array type but found {:?}",
                    other
                ))),
            },
            Rule::array_type => Self::Array {
                item: Box::new(try_into_type_anchor(node.one_inner()?)?),
                non_empty: false,
            },
            Rule::map_type => {
                let mut inner = node.into_inner();
                let key = Box::new(try_into_type_anchor(inner.next_node()?)?);
                let value = Box::new(try_into_type_anchor(inner.next_node()?)?);
                Self::Map { key, value }
            }
            Rule::pair_type => {
                let mut inner = node.into_inner();
                let left = Box::new(try_into_type_anchor(inner.next_node()?)?);
                let right = Box::new(try_into_type_anchor(inner.next_node()?)?);
                Self::Pair { left, right }
            }
            Rule::user_type => Self::User(node.one_inner()?.try_into()?),
            _ => return node.into_err(|node| format!("Invalid typedef {:?}", node)),
        };
        Ok(t)
    }
}

fn try_into_optional<'a>(node: PestNode<'a>) -> Result<Type, ModelError> {
    let inner_span = node.as_span();
    Ok(Type::Optional(Box::new(Anchor::new(
        node.try_into()?,
        inner_span,
    ))))
}

// fn try_into_array_anchor<'a>(node: PestNode<'a>) -> Result<Anchor<Type>, ModelError> {
//     let outer_span = node.as_span();
//     let mut inner = node.into_inner();
//     let item = Box::new(try_into_type_anchor(inner.next_node()?)?);
//     match inner.next().transpose()? {
//         Some(non_empty_node) if non_empty_node.as_rule() == Rule::non_empty => Ok(Anchor::new(
//             Type::Array {
//                 item,
//                 non_empty: true,
//             },
//             outer_span,
//         )),
//         Some(node) => bail!(ModelError::parser(format!(
//             "Expected non-empty token '+' but found {:?}",
//             node
//         ))),
//         None => {
//             // HACK: there is no way to determine from the parse tree what is the range of the
//             // array typedef sans modifier, so we just assume the span is extended by 1
//             let new_span = Span::new(
//                 outer_span.start,
//                 Position::new(
//                     outer_span.end.line,
//                     outer_span.end.column - 1,
//                     outer_span.end.offset - 1,
//                 ),
//             );
//             Ok(Anchor::new(
//                 Type::Array {
//                     item,
//                     non_empty: false,
//                 },
//                 new_span,
//             ))
//         }
//     }
// }

fn try_into_type_anchor<'a>(node: PestNode<'a>) -> Result<Anchor<Type>, ModelError> {
    let outer_span = node.as_span();
    let mut inner = node.into_inner();
    let type_node = inner.next_node()?;
    match inner.next().transpose()? {
        Some(opt_node) if opt_node.as_rule() == Rule::optional => {
            Ok(Anchor::new(try_into_optional(type_node)?, outer_span))
        }
        Some(node) => bail!(ModelError::parser(format!(
            "Expected optional token '?' but found {:?}",
            node
        ))),
        None => Ok(type_node.try_into()?),
    }
}

impl<'a> TryFrom<PestNode<'a>> for UnboundDeclaration {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            type_: try_into_type_anchor(inner.next_node()?)?,
            name: inner.next_node().try_into()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for BoundDeclaration {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            type_: try_into_type_anchor(inner.next_node()?)?,
            name: inner.next_node().try_into()?,
            expression: expressions::try_into_expression_anchor(inner.next_node()?)?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for InputDeclaration {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let decl = match node.as_rule() {
            Rule::unbound_declaration => Self::Unbound(node.try_into()?),
            Rule::bound_declaration => Self::Bound(node.try_into()?),
            _ => return node.into_err(|node| format!("Invalid declaration {:?}", node)),
        };
        Ok(decl)
    }
}

impl<'a> TryFrom<PestNode<'a>> for Input {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(Input {
            declarations: node.into_inner().collect_anchors_with_inner_spans()?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Output {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(Output {
            declarations: node.into_inner().collect_anchors_with_inner_spans()?,
        })
    }
}
