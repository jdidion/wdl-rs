use crate::{
    model::{Anchor, Comments, ModelError, SourceFragment, Span},
    parsers::tree_sitter::syntax,
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

    /// Consumes this node and returns an iterator over the non-comment children of this node,
    /// regardless of their kind or whether they are fields.
    pub fn into_children(self) -> TSNodeIterator<'a> {
        TSNodeIterator::from_children(self.cursor, self.node.id(), self.text, self.comments)
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

pub trait TSNodeResultExt<'a> {
    fn into_string(self) -> Result<String, ModelError>;

    fn into_element<T: TryFrom<TSNode<'a>, Error = Report<ModelError>>>(
        self,
    ) -> Result<T, ModelError>;

    fn into_children(self) -> Result<TSNodeIterator<'a>, ModelError>;

    fn into_anchor_from_str<T: FromStr<Err = Report<ModelError>>>(
        self,
    ) -> Result<Anchor<T>, ModelError>;

    fn into_boxed_anchor<T: TryFrom<TSNode<'a>, Error = Report<ModelError>>>(
        self,
    ) -> Result<Box<Anchor<T>>, ModelError>;
}

impl<'a> TSNodeResultExt<'a> for Result<TSNode<'a>, ModelError> {
    fn into_string(self) -> Result<String, ModelError> {
        self.and_then(|node| Ok(node.try_into()?))
    }

    fn into_element<T: TryFrom<TSNode<'a>, Error = Report<ModelError>>>(
        self,
    ) -> Result<T, ModelError> {
        self.and_then(|node| node.try_into())
    }

    fn into_children(self) -> Result<TSNodeIterator<'a>, ModelError> {
        self.map(|node| node.into_children())
    }

    fn into_anchor_from_str<T: FromStr<Err = Report<ModelError>>>(
        self,
    ) -> Result<Anchor<T>, ModelError> {
        self.and_then(|node| {
            Ok(Anchor {
                span: node.as_span(),
                element: T::from_str(node.try_as_str()?)?,
            })
        })
    }

    fn into_boxed_anchor<T: TryFrom<TSNode<'a>, Error = Report<ModelError>>>(
        self,
    ) -> Result<Box<Anchor<T>>, ModelError> {
        self.and_then(|node| Ok(Box::new(node.try_into()?)))
    }
}

/// TSNodeIterator state.
enum State {
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
    state: State,
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
            state: State::PendingChildren(parent_id),
            text,
            comments,
        }
    }

    /// Advances the iterator to the next non-comment node and returns `Some((node, field_name))`,
    /// or `None` if there are no more non-comment nodes. If this is an iterator over child nodes,
    /// then the iterator is returned to the parent node the first time this method returns `None`.
    fn advance(&mut self) -> Result<Option<(ts::Node<'a>, Option<&'a str>)>, ModelError> {
        let mut cursor = (*self.cursor).borrow_mut();
        loop {
            match self.state {
                // State::Pending => self.state = State::Iterating,
                // State::Iterating if !cursor.goto_next_sibling() => {
                //     self.state = State::Done;
                //     return Ok(None);
                // }
                State::PendingChildren(parent_id) if cursor.goto_first_child() => {
                    self.state = State::IteratingChildren(parent_id)
                }
                State::PendingChildren(_) => {
                    self.state = State::Done;
                    return Ok(None);
                }
                State::IteratingChildren(initial_parent_id) if !cursor.goto_next_sibling() => {
                    if !cursor.goto_parent() {
                        bail!(ModelError::parser(String::from(
                            "Could not return cursor to parent node",
                        )));
                    }
                    let parent_id = cursor.node().id();
                    if parent_id != initial_parent_id {
                        bail!(ModelError::parser(format!(
                            "Node iterator returned to different parent {} than it started from {}",
                            parent_id, initial_parent_id
                        )));
                    }
                    self.state = State::Done;
                    return Ok(None);
                }
                State::Done => return Ok(None),
                _ => (),
            }
            let node = cursor.node();
            if node.is_error() || node.is_missing() {
                return Err(report!(ModelError::parser(format!(
                    "Parser error {:?}",
                    node
                ))));
            } else if node.kind() == syntax::COMMENT {
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

    /// Same as `next()` but returns an `Err` if the iterator is exausted or if the next node
    /// is not a field with the specified `name`.
    pub fn next_field(&mut self, name: &'a str) -> Result<TSNode<'a>, ModelError> {
        match self.advance()? {
            Some((node, Some(field))) if field == name => Ok(TSNode::new(
                node,
                Some(field),
                self.cursor.clone(),
                self.text,
                self.comments.clone(),
            )),
            Some((node, field)) => bail!(ModelError::parser(format!(
                "Expected next node to be a field with name {} but was {:?} (field: {:?})",
                name, node, field
            ))),
            None => bail!(ModelError::parser(format!(
                "Expected next node to be a field with name {} but iterator is exhausted",
                name
            ))),
        }
    }

    pub fn collect_anchors<T: TryFrom<TSNode<'a>, Error = Report<ModelError>>>(
        &mut self,
    ) -> Result<Vec<Anchor<T>>, ModelError> {
        self.map(|res| res.try_into()).collect()
    }

    /// Consumes the remaining items in the iterator. Returns an `Err` if `ensure_exhausted` is
    /// `true` and the iterator contains any non-comment nodes.
    fn drain(&mut self, ensure_exhausted: bool) -> Result<(), ModelError> {
        // advance the iterator to the end to ensure all comment nodes are handled and the cursor
        // is returned to the parent node
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

pub trait TSNodeIteratorResultExt<'a> {
    fn collect_anchors<T: TryFrom<TSNode<'a>, Error = Report<ModelError>>>(
        self,
    ) -> Result<Vec<Anchor<T>>, ModelError>;
}

impl<'a> TSNodeIteratorResultExt<'a> for Result<TSNodeIterator<'a>, ModelError> {
    fn collect_anchors<T: TryFrom<TSNode<'a>, Error = Report<ModelError>>>(
        self,
    ) -> Result<Vec<Anchor<T>>, ModelError> {
        self.and_then(|mut itr| itr.collect_anchors())
    }
}
