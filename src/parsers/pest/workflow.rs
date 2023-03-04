use crate::{
    model::{
        Anchor, Call, CallInput, Conditional, InnerSpan, ModelError, QualifiedIdentifier, Scatter,
        Span, Workflow, WorkflowElement, WorkflowNestedElement,
    },
    parsers::pest::{
        expressions,
        node::{PestNode, PestNodes},
        Rule,
    },
};
use error_stack::{bail, Report, Result};
use std::convert::TryFrom;

impl<'a> TryFrom<PestNode<'a>> for QualifiedIdentifier {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let inner = node.into_inner();
        let parts: Result<Vec<Anchor<String>>, ModelError> = inner
            .map(|res| {
                res.and_then(|node| {
                    let mut span = node.as_span();
                    let s = node.as_str();
                    let trimmed = s.trim_end().to_owned();
                    let ws_len = s.len() - trimmed.len();
                    if ws_len > 0 {
                        span.trim_end(ws_len);
                    }
                    Ok(Anchor::new(trimmed, span))
                })
            })
            .collect();
        Ok(Self { parts: parts? })
    }
}

impl<'a> TryFrom<PestNode<'a>> for CallInput {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        let name = inner.next_node().try_into()?;
        let expression = inner
            .next()
            .map(|expr_node| expressions::try_into_expression_anchor(expr_node?))
            .transpose()?;
        Ok(Self { name, expression })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Call {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        let target = inner.next_node()?.try_into_anchor_with_inner_span()?;
        let alias = if let Some(Rule::call_alias) = inner.peek_rule() {
            let alias_node = inner.next_node()?;
            Some(alias_node.one_inner().try_into()?)
        } else {
            None
        };
        let inputs = if let Some(Rule::call_inputs) = inner.peek_rule() {
            let inputs_node = inner.next_node()?;
            Some(
                inputs_node
                    .into_inner()
                    .collect_anchors_with_inner_spans()?,
            )
        } else {
            None
        };
        Ok(Self {
            target,
            alias,
            inputs,
        })
    }
}

fn try_into_nested_body<'a>(
    nodes: PestNodes<'a>,
) -> Result<Vec<Anchor<WorkflowNestedElement>>, ModelError> {
    nodes
        .map(|res| {
            res.and_then(|node| match node.as_rule() {
                Rule::bound_declaration => {
                    let element: WorkflowNestedElement = node.try_into()?;
                    if let WorkflowNestedElement::Declaration(decl) = &element {
                        let span = decl.get_inner_span().unwrap();
                        Ok(Anchor::new(element, span))
                    } else {
                        bail!(ModelError::parser(format!(
                            "expected declaration not {:?}",
                            element
                        )))
                    }
                }
                Rule::call => {
                    let span = node.as_span();
                    let call: Call = node.try_into()?;
                    let span = if call.inputs.is_some() {
                        span
                    } else {
                        Span::from_range(&span, &call.get_inner_span().unwrap())
                    };
                    Ok(Anchor::new(WorkflowNestedElement::Call(call), span))
                }
                _ => node.try_into(),
            })
        })
        .collect()
}

impl<'a> TryFrom<PestNode<'a>> for Scatter {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            name: inner.next_node().try_into()?,
            expression: expressions::try_into_expression_anchor(inner.next_node()?)?,
            body: try_into_nested_body(inner)?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for Conditional {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        Ok(Self {
            expression: expressions::try_into_expression_anchor(inner.next_node()?)?,
            body: try_into_nested_body(inner)?,
        })
    }
}

impl<'a> TryFrom<PestNode<'a>> for WorkflowNestedElement {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let e = match node.as_rule() {
            Rule::bound_declaration => Self::Declaration(node.try_into()?),
            Rule::call => Self::Call(node.try_into()?),
            Rule::scatter => Self::Scatter(node.try_into()?),
            Rule::conditional => Self::Conditional(node.try_into()?),
            _ => bail!(ModelError::parser(format!(
                "Invalid task element {:?}",
                node
            ))),
        };
        Ok(e)
    }
}

impl<'a> TryFrom<PestNode<'a>> for WorkflowElement {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let e = match node.as_rule() {
            Rule::input => Self::Input(node.try_into()?),
            Rule::output => Self::Output(node.try_into()?),
            Rule::bound_declaration => Self::Declaration(node.try_into()?),
            Rule::meta => Self::Meta(node.try_into()?),
            Rule::parameter_meta => Self::ParameterMeta(node.try_into()?),
            Rule::call => Self::Call(node.try_into()?),
            Rule::scatter => Self::Scatter(node.try_into()?),
            Rule::conditional => Self::Conditional(node.try_into()?),
            _ => bail!(ModelError::parser(format!(
                "Invalid task element {:?}",
                node
            ))),
        };
        Ok(e)
    }
}

impl<'a> TryFrom<PestNode<'a>> for Workflow {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let mut inner = node.into_inner();
        let name = inner.next_node().try_into()?;
        let body: Result<Vec<Anchor<WorkflowElement>>, ModelError> = inner
            .map(|res| {
                res.and_then(|node| match node.as_rule() {
                    Rule::bound_declaration => {
                        let element: WorkflowElement = node.try_into()?;
                        if let WorkflowElement::Declaration(decl) = &element {
                            let span = decl.get_inner_span().unwrap();
                            Ok(Anchor::new(element, span))
                        } else {
                            bail!(ModelError::parser(format!(
                                "expected declaration not {:?}",
                                element
                            )))
                        }
                    }
                    Rule::call => {
                        let span = node.as_span();
                        let call: Call = node.try_into()?;
                        let span = if call.inputs.is_some() {
                            span
                        } else {
                            Span::from_range(&span, &call.get_inner_span().unwrap())
                        };
                        Ok(Anchor::new(WorkflowElement::Call(call), span))
                    }
                    _ => node.try_into(),
                })
            })
            .collect();
        Ok(Self { name, body: body? })
    }
}
