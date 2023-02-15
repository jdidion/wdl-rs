use crate::{
    model::{Anchor, Comments, ModelError, SourceFragment, Span},
    parsers::tree_sitter::syntax::rules,
};
use error_stack::{bail, report, IntoReport, Report, Result, ResultExt};
use std::{cell::RefCell, fmt::Debug, ops::DerefMut, rc::Rc, str::FromStr};
use tree_sitter as ts;

fn node_as_str<'a>(node: ts::Node<'a>, text: &'a [u8]) -> Result<&'a str, ModelError> {
    node.utf8_text(text)
        .into_report()
        .change_context(ModelError::parser(format!(
            "error getting node contents as str {:?}",
            node
        )))
}

/// Adds a comment node to `comments`.
fn add_comment<'a, C: DerefMut<Target = Comments>>(
    node: ts::Node<'a>,
    text: &'a [u8],
    mut comments: C,
) -> Result<(), ModelError> {
    let element = node_as_str(node, text)?;
    let comment = Anchor {
        element: element.to_owned(),
        span: (&node).into(),
    };
    comments
        .deref_mut()
        .try_insert(node.start_position().row, comment)?;
    Ok(())
}

pub struct TSNode<'a> {
    node: ts::Node<'a>,
    field: Option<&'a str>,
    cursor: Rc<RefCell<ts::TreeCursor<'a>>>,
    text: &'a [u8],
    comments: Rc<RefCell<Comments>>,
}

impl<'a> TSNode<'a> {
    fn new(
        node: ts::Node<'a>,
        field: Option<&'a str>,
        cursor: Rc<RefCell<ts::TreeCursor<'a>>>,
        text: &'a [u8],
        comments: Rc<RefCell<Comments>>,
    ) -> Self {
        Self {
            node,
            field,
            cursor,
            text,
            comments,
        }
    }

    pub fn from_cursor(
        cursor: Rc<RefCell<ts::TreeCursor<'a>>>,
        text: &'a [u8],
        comments: Rc<RefCell<Comments>>,
    ) -> Self {
        let (node, field) = {
            let cursor_ref = cursor.borrow();
            (cursor_ref.node(), cursor_ref.field_name())
        };
        Self {
            node,
            field,
            cursor,
            text,
            comments,
        }
    }

    pub fn kind(&self) -> &'a str {
        self.node.kind()
    }

    pub fn as_span(&self) -> Span {
        (&self.node).into()
    }

    pub fn try_as_str(&self) -> Result<&'a str, ModelError> {
        node_as_str(self.node, self.text)
    }

    pub fn try_into_anchor_from_str<T: FromStr<Err = Report<ModelError>>>(
        self,
    ) -> Result<Anchor<T>, ModelError> {
        Ok(Anchor {
            span: self.as_span(),
            element: T::from_str(self.try_as_str()?)?,
        })
    }

    pub fn get_field(&self) -> Option<&'a str> {
        self.field
    }

    pub fn try_field(&self) -> Result<&'a str, ModelError> {
        self.field.map(|field| Ok(field)).unwrap_or_else(|| {
            Err(report!(ModelError::parser(String::from(
                "Expected node to be a field"
            ))))
        })
    }

    pub fn ensure_field(&self, name: &'a str) -> Result<(), ModelError> {
        self.try_field().and_then(|field| {
            if name == field {
                Ok(())
            } else {
                Err(report!(ModelError::parser(format!(
                    "Expected node to be a field with name {} but was {}",
                    name, field
                ))))
            }
        })
    }

    pub fn try_into_boxed_anchor<T: TryFrom<TSNode<'a>, Error = Report<ModelError>>>(
        self,
    ) -> Result<Box<Anchor<T>>, ModelError> {
        Ok(Box::new(self.try_into()?))
    }

    /// Consumes this node and returns an iterator over the non-comment children of this node,
    /// regardless of their kind or whether they are fields.
    pub fn into_children(self) -> TSNodeIterator<'a> {
        TSNodeIterator::from_children(self.cursor, self.node.id(), self.text, self.comments)
    }

    pub fn into_block(self, start: &'a str, end: &'a str) -> TSBlockIterator<'a> {
        TSBlockIterator::new(self.into_children(), Some(start), Some(end), None)
    }

    pub fn into_list(
        self,
        sep: &'a str,
        start: Option<&'a str>,
        end: Option<&'a str>,
    ) -> TSBlockIterator<'a> {
        TSBlockIterator::new(self.into_children(), start, end, Some(sep))
    }

    /// Consumes this node and returns a single child of this node with the specified field name.
    /// Returns `Err` if this node does not contain exactly one non-comment child with specified
    /// field name.
    pub fn try_into_child_field(self, name: &'a str) -> Result<TSNode<'a>, ModelError> {
        let mut children = self.into_children();
        let field = children.next_field(name)?;
        children.drain(true)?;
        return Ok(field);
    }

    pub fn clone_comments(&self) -> Rc<RefCell<Comments>> {
        self.comments.clone()
    }

    pub fn into_err<F: FnOnce(Self) -> String>(self, f: F) -> Result<(), ModelError> {
        let span = self.as_span();
        let text = node_as_str(self.node, self.text)?;
        Err(Report::from(ModelError::parser(f(self)))
            .attach_printable(span)
            .attach_printable(SourceFragment(text.to_owned())))
    }
}

impl<'a> Debug for TSNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TSNode")
            .field("node", &self.node)
            .field("field", &self.field)
            .field("text", &self.text)
            .field("comments", &self.comments)
            .finish()
    }
}

impl<'a> TryFrom<TSNode<'a>> for String {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        node.try_as_str().map(|s| s.to_owned())
    }
}

impl<'a, T: TryFrom<TSNode<'a>, Error = Report<ModelError>>> TryFrom<TSNode<'a>> for Anchor<T> {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        Ok(Self {
            span: node.as_span(),
            element: node.try_into()?,
        })
    }
}

impl<'a, T: TryFrom<TSNode<'a>, Error = Report<ModelError>>> TryFrom<Result<TSNode<'a>, ModelError>>
    for Anchor<T>
{
    type Error = Report<ModelError>;

    fn try_from(res: Result<TSNode<'a>, ModelError>) -> Result<Self, ModelError> {
        res.and_then(|node| Ok(node.try_into()?))
    }
}

/// TSNodeIterator state.
#[derive(Debug)]
enum NodeIterState {
    /// Pending iteration of cursor.
    //Pending,
    /// Iterating siblings of cursor's initial position.
    //Iterating,
    /// Pending iteration of the node (with given id) at the cursor's current position.
    PendingChildren(usize),
    /// Iterating children of the node (with given id) at the cursor's initial position.
    IteratingChildren(usize),
    /// Done iterating.
    Done,
}

/// An iterator over sibling nodes.
pub struct TSNodeIterator<'a> {
    cursor: Rc<RefCell<ts::TreeCursor<'a>>>,
    state: NodeIterState,
    text: &'a [u8],
    comments: Rc<RefCell<Comments>>,
}

impl<'a> TSNodeIterator<'a> {
    /// Creates an iterator over the children of the cursor's current node. When the iterator
    /// is dropped, it will exhaust itself (so that all comment nodes are handled) and then
    /// return to the parent node.
    fn from_children(
        cursor: Rc<RefCell<ts::TreeCursor<'a>>>,
        parent_id: usize,
        text: &'a [u8],
        comments: Rc<RefCell<Comments>>,
    ) -> Self {
        Self {
            cursor,
            state: NodeIterState::PendingChildren(parent_id),
            text,
            comments,
        }
    }

    /// Advances the iterator to the next non-comment node and returns `Some((node, field_name))`,
    /// or `None` if there are no more non-comment nodes. If this is an iterator over child nodes,
    /// then the iterator is returned to the parent node the first time this method returns `None`.
    fn advance(&mut self) -> Result<Option<(ts::Node<'a>, Option<&'a str>)>, ModelError> {
        let mut cursor = (*self.cursor).borrow_mut();
        println!("starting state {:?}", self.state);
        loop {
            match self.state {
                // State::Pending => self.state = State::Iterating,
                // State::Iterating if !cursor.goto_next_sibling() => {
                //     self.state = State::Done;
                //     return Ok(None);
                // }
                NodeIterState::PendingChildren(parent_id) if cursor.goto_first_child() => {
                    println!("pending {} -> iterating {}", parent_id, cursor.node().id());
                    self.state = NodeIterState::IteratingChildren(parent_id)
                }
                NodeIterState::PendingChildren(parent_id) => {
                    println!("pending -> done {}", parent_id);
                    self.state = NodeIterState::Done;
                    return Ok(None);
                }
                NodeIterState::IteratingChildren(initial_parent_id)
                    if !cursor.goto_next_sibling() =>
                {
                    println!("iterating -> done {}", initial_parent_id);
                    if !cursor.goto_parent() {
                        bail!(ModelError::parser(String::from(
                            "Could not return cursor to parent node",
                        )));
                    }
                    let parent_id = cursor.node().id();
                    println!("  returned to {}", parent_id);
                    if parent_id != initial_parent_id {
                        bail!(ModelError::parser(format!(
                            "Node iterator returned to different parent {} than it started from {}",
                            parent_id, initial_parent_id
                        )));
                    }
                    self.state = NodeIterState::Done;
                    return Ok(None);
                }
                NodeIterState::Done => {
                    println!("done");
                    return Ok(None);
                }
                _ => (),
            }
            let node = cursor.node();
            println!("node {:?} field {:?}", node, cursor.field_name());
            if node.is_error() || node.is_missing() {
                return Err(report!(ModelError::parser(format!(
                    "Parser error {:?}",
                    node
                ))));
            } else if node.kind() == rules::COMMENT {
                add_comment(node, self.text, (*self.comments).borrow_mut())?;
            } else if node.is_extra() {
                return Err(report!(ModelError::parser(format!(
                    "Unexpected extra node {:?}",
                    node
                ))));
            } else {
                return Ok(Some((node, cursor.field_name())));
            };
        }
    }

    /// Same as `next()` but returns an `Err` if the iterator is exausted.
    pub fn next_node(&mut self) -> Result<TSNode<'a>, ModelError> {
        match self.advance()? {
            Some((node, field)) => Ok(TSNode::new(
                node,
                field,
                self.cursor.clone(),
                self.text,
                self.comments.clone(),
            )),
            None => bail!(ModelError::parser(String::from(
                "Expected next node but iterator is exhausted",
            ))),
        }
    }

    pub fn skip_terminal(&mut self, name: &'a str) -> Result<(), ModelError> {
        let node = self.advance()?;
        println!("skip_terminal {} {:?}", name, node);
        match node {
            Some((node, None)) if node.kind() == name => Ok(()),
            Some((node, field)) => Err(report!(ModelError::parser(format!(
                "Expected next node to be a terminal with kind '{}' but was {:?} (field: {:?})",
                name, node, field
            )))),
            None => Err(report!(ModelError::parser(format!(
                "Expected next node to be a terminal with kind '{name}' but iterator is exhausted",
            )))),
        }
    }

    pub fn skip_optional(&mut self, name: &'a str) -> Result<bool, ModelError> {
        match self.advance()? {
            Some((node, None)) if node.kind() == name => Ok(true),
            Some((node, field)) => Err(report!(ModelError::parser(format!(
                "Expected next node to be a terminal with kind '{}' but was {:?} (field: {:?})",
                name, node, field
            )))),
            None => Ok(false),
        }
    }

    /// Same as `next()` but returns an `Err` if the next node is not `None` or a field with the
    /// specified `name`.
    pub fn get_next_field(&mut self, name: &'a str) -> Result<Option<TSNode<'a>>, ModelError> {
        match self.advance()? {
            Some((node, Some(field))) if field == name => Ok(Some(TSNode::new(
                node,
                Some(field),
                self.cursor.clone(),
                self.text,
                self.comments.clone(),
            ))),
            Some((node, field)) => Err(report!(ModelError::parser(format!(
                "Expected next node to be a field with name '{}' but was {:?} (field: {:?})",
                name, node, field
            )))),
            None => Ok(None),
        }
    }

    /// Same as `next()` but returns an `Err` if the iterator is exausted or if the next node
    /// is not a field with the specified `name`.
    pub fn next_field(&mut self, name: &'a str) -> Result<TSNode<'a>, ModelError> {
        self.get_next_field(name).and_then(|opt| {
            opt.ok_or_else(|| {
                report!(ModelError::parser(format!(
                    "Expected next node to be a field with name '{}' but iterator is exhausted",
                    name
                )))
            })
        })
    }

    /// Consumes the remaining items in the iterator. Returns an `Err` if `ensure_exhausted` is
    /// `true` and the iterator contains any non-comment nodes.
    fn drain(&mut self, ensure_exhausted: bool) -> Result<(), ModelError> {
        // advance the iterator to the end to ensure all comment nodes are handled and the cursor
        // is returned to the parent node
        println!("drain {}", ensure_exhausted);
        let mut remaining = Vec::new();
        while let Some(next) = self.advance()? {
            remaining.push(next);
        }
        if ensure_exhausted && !remaining.is_empty() {
            bail!(ModelError::parser(format!(
                "Expected iterator to be exhausted but found one or more non-comment node(s) {:?}",
                remaining
                    .into_iter()
                    .map(|(node, field)| format!("{:?} (field: {:?})", node, field))
                    .collect::<Vec<_>>()
                    .join(", ")
            )))
        }
        Ok(())
    }
}

impl<'a> Iterator for TSNodeIterator<'a> {
    type Item = Result<TSNode<'a>, ModelError>;

    /// Starting at the cursor's current node, adds a comment for each comment node (i.e. kind ==
    /// `syntax::COMMENT`) and moves to the next sibling node until a non-comment node is found.
    /// Returns the first non-comment node, or `None` if all remaining nodes are comments.
    fn next(&mut self) -> Option<Self::Item> {
        self.advance().transpose().map(|res| {
            res.and_then(|(node, field)| {
                Ok(TSNode::new(
                    node,
                    field,
                    self.cursor.clone(),
                    self.text,
                    self.comments.clone(),
                ))
            })
        })
    }
}

impl<'a> Drop for TSNodeIterator<'a> {
    fn drop(&mut self) {
        self.drain(false).unwrap();
    }
}

#[derive(Clone, Debug)]
enum BlockIterState<'a> {
    Start {
        start: Option<&'a str>,
        end: Option<&'a str>,
        sep: Option<&'a str>,
    },
    Item {
        end: Option<&'a str>,
        sep: Option<&'a str>,
    },
    Sep {
        end: Option<&'a str>,
        sep: Option<&'a str>,
    },
    Done,
}

pub struct TSBlockIterator<'a> {
    nodes: TSNodeIterator<'a>,
    state: BlockIterState<'a>,
}

impl<'a> TSBlockIterator<'a> {
    fn new(
        nodes: TSNodeIterator<'a>,
        start: Option<&'a str>,
        end: Option<&'a str>,
        sep: Option<&'a str>,
    ) -> Self {
        let state = match start {
            Some(_) => BlockIterState::Start { start, end, sep },
            None => BlockIterState::Item { end, sep },
        };
        Self { nodes, state }
    }
}

impl<'a> TSBlockIterator<'a> {
    pub fn next_field(&mut self, name: &'a str) -> Result<TSNode<'a>, ModelError> {
        let node = self.next().unwrap_or_else(|| {
            Err(report!(ModelError::parser(format!(
                "Expected next node to be a field with name '{}' but iterator is exhausted",
                name
            ))))
        })?;
        node.ensure_field(name)?;
        Ok(node)
    }
}

impl<'a> Iterator for TSBlockIterator<'a> {
    type Item = Result<TSNode<'a>, ModelError>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.nodes.next(), &self.state) {
            (
                Some(Ok(next)),
                BlockIterState::Start {
                    start: Some(delim),
                    end,
                    sep,
                },
            ) if next.kind() == *delim => {
                self.state = BlockIterState::Item {
                    end: end.clone(),
                    sep: sep.clone(),
                };
                self.next()
            }
            (
                Some(Ok(next)),
                BlockIterState::Start {
                    start: Some(delim),
                    end: _,
                    sep: _,
                },
            ) => Some(Err(report!(ModelError::parser(format!(
                "Expected first item to be start token {} but was {:?}",
                delim, next
            ))))),
            (
                Some(Ok(next)),
                BlockIterState::Item {
                    end: Some(delim),
                    sep: _,
                },
            ) if next.kind() == *delim => {
                self.state = BlockIterState::Done;
                None
            }
            (Some(Ok(next)), BlockIterState::Item { end, sep }) if sep.is_some() => {
                self.state = BlockIterState::Sep {
                    end: end.clone(),
                    sep: sep.clone(),
                };
                Some(Ok(next))
            }
            (Some(Ok(next)), BlockIterState::Item { end: _, sep: _ }) => Some(Ok(next)),
            (
                Some(Ok(next)),
                BlockIterState::Sep {
                    end: Some(delim),
                    sep: _,
                },
            ) if next.kind() == *delim => {
                self.state = BlockIterState::Done;
                None
            }
            (
                Some(Ok(next)),
                BlockIterState::Sep {
                    end,
                    sep: Some(delim),
                },
            ) if next.kind() == *delim => {
                self.state = BlockIterState::Item {
                    end: end.clone(),
                    sep: Some(delim),
                };
                self.next()
            }
            (Some(Ok(next)), BlockIterState::Done) => {
                Some(Err(report!(ModelError::parser(format!(
                    "Expected iterator to be exhausted but found next node {:?}",
                    next
                )))))
            }
            (Some(Err(err)), _) => Some(Err(err)),
            (
                None,
                BlockIterState::Start {
                    start: Some(delim),
                    end: _,
                    sep: _,
                },
            ) => Some(Err(report!(ModelError::parser(format!(
                "Expected next item to be start token token {} but was None",
                delim
            ))))),
            (
                None,
                BlockIterState::Start {
                    start: None,
                    end: Some(delim),
                    sep: _,
                },
            ) => Some(Err(report!(ModelError::parser(format!(
                "Expected next item to be a block element or end token {} but was None",
                delim
            ))))),
            (
                None,
                BlockIterState::Start {
                    start: _,
                    end: _,
                    sep: _,
                },
            ) => {
                self.state = BlockIterState::Done;
                None
            }
            (
                None,
                BlockIterState::Item {
                    end: Some(delim),
                    sep: _,
                },
            ) => Some(Err(report!(ModelError::parser(format!(
                "Expected next item to be a block element or end token {} but was None",
                delim
            ))))),
            (
                None,
                BlockIterState::Sep {
                    end: Some(end),
                    sep: Some(sep),
                },
            ) => Some(Err(report!(ModelError::parser(format!(
                "Expected next item to be a separator {} or end token {} but was None",
                sep, end
            ))))),
            (None, BlockIterState::Done) => None,
            (None, _) => {
                self.state = BlockIterState::Done;
                None
            }
            _ => Some(Err(report!(ModelError::parser(format!(
                "Invalid iterator state {:?}",
                self.state
            ))))),
        }
    }
}

pub trait TSIteratorExt<'a>: Iterator<Item = Result<TSNode<'a>, ModelError>> {
    fn collect_anchors<T: TryFrom<TSNode<'a>, Error = Report<ModelError>>>(
        &mut self,
    ) -> Result<Vec<Anchor<T>>, ModelError> {
        self.map(|res| res.try_into()).collect()
    }
}

impl<'a> TSIteratorExt<'a> for TSNodeIterator<'a> {}
impl<'a> TSIteratorExt<'a> for TSBlockIterator<'a> {}
