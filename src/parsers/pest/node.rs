use crate::{
    model::{Anchor, Comments, InnerSpan, ModelError, SourceFragment, Span},
    parsers::pest::Rule,
};
use error_stack::{report, Report, Result};
use pest::iterators::{Pair, Pairs};
use std::{cell::RefCell, rc::Rc, str::FromStr};

/// Wraps a pest `Pair`, and a shared `Comments` that is added to as the model is built.
#[derive(Debug)]
pub struct PestNode<'a> {
    pair: Pair<'a, Rule>,
    comments: Rc<RefCell<Comments>>,
}

impl<'a> PestNode<'a> {
    pub fn new(pair: Pair<'a, Rule>, comments: Rc<RefCell<Comments>>) -> Self {
        Self { pair, comments }
    }

    pub fn as_str(&self) -> &'a str {
        self.pair.as_str()
    }

    pub fn as_rule(&self) -> Rule {
        self.pair.as_rule()
    }

    pub fn as_span(&self) -> Span {
        (&self.pair.as_span()).into()
    }

    /// Tries to convert this node's string value to an `Anchor<T>`.
    pub fn try_into_anchor_from_str<T: FromStr<Err = Report<ModelError>>>(
        &self,
    ) -> Result<Anchor<T>, ModelError> {
        Ok(Anchor::new(
            T::from_str(self.pair.as_str())?,
            self.as_span(),
        ))
    }

    pub fn try_into_boxed_anchor<T: TryFrom<PestNode<'a>, Error = Report<ModelError>>>(
        self,
    ) -> Result<Box<Anchor<T>>, ModelError> {
        Ok(Box::new(self.try_into()?))
    }

    pub fn try_into_anchor_with_inner_span<
        T: TryFrom<PestNode<'a>, Error = Report<ModelError>> + InnerSpan + std::fmt::Debug,
    >(
        self,
    ) -> Result<Anchor<T>, ModelError> {
        let element: T = T::try_from(self)?;
        let span: Result<Span, ModelError> = element.get_inner_span().ok_or_else(|| {
            report!(ModelError::parser(format!(
                "element has no inner span {:?}",
                element
            )))
        });
        Ok(Anchor::new(element, span?))
    }

    /// Returns an iterator over this node's inner nodes.
    pub fn into_inner(self) -> PestNodes<'a> {
        PestNodes {
            pairs: self.pair.into_inner(),
            comments: self.comments.clone(),
        }
    }

    /// Convenience function to get the inner node when there is expected to be exactly one.
    pub fn one_inner(self) -> Result<PestNode<'a>, ModelError> {
        self.into_inner().next_node()
    }

    pub fn clone_comments(&self) -> Rc<RefCell<Comments>> {
        self.comments.clone()
    }

    pub fn into_err<T, F: FnOnce(Self) -> String>(self, f: F) -> Result<T, ModelError> {
        let span = self.as_span();
        let text = self.as_str().to_owned();
        Err(Report::from(ModelError::parser(f(self)))
            .attach_printable(span)
            .attach_printable(SourceFragment(text)))
    }
}

impl<'a> TryFrom<PestNode<'a>> for String {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        Ok(node.pair.as_str().to_owned())
    }
}

impl<'a, T: TryFrom<PestNode<'a>, Error = Report<ModelError>>> TryFrom<PestNode<'a>> for Anchor<T> {
    type Error = Report<ModelError>;

    fn try_from(node: PestNode<'a>) -> Result<Self, ModelError> {
        let span = node.as_span();
        Ok(Self::new(node.try_into()?, span))
    }
}

impl<'a, T: TryFrom<PestNode<'a>, Error = Report<ModelError>>>
    TryFrom<Result<PestNode<'a>, ModelError>> for Anchor<T>
{
    type Error = Report<ModelError>;

    fn try_from(res: Result<PestNode<'a>, ModelError>) -> Result<Self, ModelError> {
        res.and_then(|node| Ok(node.try_into()?))
    }
}

/// Wraps a `Pairs`, and a shared `Comments` that is added to as the model is built.
pub struct PestNodes<'a> {
    pairs: Pairs<'a, Rule>,
    comments: Rc<RefCell<Comments>>,
}

impl<'a> PestNodes<'a> {
    /// Returns the rule of the next pair without advancing the iterator.
    pub fn peek_rule(&self) -> Option<Rule> {
        self.pairs.peek().map(|pair| pair.as_rule())
    }

    pub fn has_next(&self) -> bool {
        match self.peek_rule() {
            Some(Rule::COMMENT) | Some(Rule::EOI) => false,
            Some(_) => true,
            None => false,
        }
    }

    fn get_next_pair(&mut self) -> Result<Option<Pair<'a, Rule>>, ModelError> {
        while let Some(pair) = self.pairs.next() {
            match pair.as_rule() {
                Rule::COMMENT => {
                    let span: Span = (&pair.as_span()).into();
                    let mut comments = self.comments.borrow_mut();
                    comments
                        .try_insert(span.start.line, Anchor::new(pair.as_str().to_owned(), span))?;
                }
                Rule::EOI => continue,
                _ => return Ok(Some(pair)),
            }
        }
        Ok(None)
    }

    /// Like `next`, but returns an `Err` instead of `None` if there is no next node.
    pub fn next_node(&mut self) -> Result<PestNode<'a>, ModelError> {
        self.next().unwrap_or_else(|| {
            Err(report!(ModelError::parser(format!(
                "A next Pair was expected at {} but is missing; this indicates a parser \
                        bug and should be reported",
                self.pairs.to_string()
            ))))
        })
    }

    pub fn collect_nodes(self) -> Result<Vec<PestNode<'a>>, ModelError> {
        self.collect()
    }

    /// Collects `Anchor<T>`s for all remaining pairs into a `Vec`.
    pub fn collect_anchors<T: TryFrom<PestNode<'a>, Error = Report<ModelError>>>(
        self,
    ) -> Result<Vec<Anchor<T>>, ModelError> {
        self.map(|node| node.try_into()).collect()
    }

    /// Collects `Anchor<T>`s for all remaining pairs into a `Vec`.
    pub fn collect_anchors_with_inner_spans<
        T: TryFrom<PestNode<'a>, Error = Report<ModelError>> + std::fmt::Debug + InnerSpan,
    >(
        self,
    ) -> Result<Vec<Anchor<T>>, ModelError> {
        self.map(|res| res.and_then(|node| node.try_into_anchor_with_inner_span()))
            .collect()
    }
}

impl<'a> Iterator for PestNodes<'a> {
    type Item = Result<PestNode<'a>, ModelError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.get_next_pair().transpose().map(|res| {
            res.and_then(|pair| {
                Ok(PestNode {
                    pair,
                    comments: self.comments.clone(),
                })
            })
        })
    }
}

impl<'a> Drop for PestNodes<'a> {
    fn drop(&mut self) {
        // drains the iterator to ensure all comment pairs are added to `self.comments`
        loop {
            match self.get_next_pair() {
                Ok(Some(_)) => (),
                Ok(None) => break,
                Err(_) => break, // TODO: log this
            }
        }
    }
}
