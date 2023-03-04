use crate::{
    model::{Anchor, Comments, ModelError, SourceFragment, Span},
    parsers::tree_sitter::syntax::rules,
};
use error_stack::{bail, report, IntoReport, Report, Result, ResultExt};
use std::{cell::RefCell, fmt::Debug, ops::DerefMut, rc::Rc, str::FromStr};
use tree_sitter as ts;

use super::syntax::symbols;

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
    let comment = Anchor::new(element.to_owned(), (&node).into());
    comments
        .deref_mut()
        .try_insert(node.start_position().row, comment)?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub enum BlockEnds {
    Braces,
    Brackets,
    Parens,
    Quotes,
    DoubleQuotes,
    SingleQuotes,
    None,
}

impl BlockEnds {
    fn get_open(symbol: &str) -> Self {
        match symbol {
            symbols::LBRACE => Self::Braces,
            symbols::LBRACK => Self::Brackets,
            symbols::LPAREN => Self::Parens,
            symbols::DQUOTE => Self::DoubleQuotes,
            symbols::SQUOTE => Self::SingleQuotes,
            _ => Self::None,
        }
    }

    fn get_close(symbol: &str) -> Self {
        match symbol {
            symbols::RBRACE => Self::Braces,
            symbols::RBRACK => Self::Brackets,
            symbols::RPAREN => Self::Parens,
            symbols::DQUOTE => Self::DoubleQuotes,
            symbols::SQUOTE => Self::SingleQuotes,
            _ => Self::None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BlockDelim {
    Comma,
    Dot,
    None,
}

impl BlockDelim {
    fn get(symbol: &str) -> Self {
        match symbol {
            symbols::COMMA => Self::Comma,
            symbols::DOT => Self::Dot,
            _ => Self::None,
        }
    }
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
        let span = self.as_span();
        Ok(Anchor::new(T::from_str(self.try_as_str()?)?, span))
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
        TSNodeIterator::from_children(
            self.cursor,
            self.node.id(),
            BlockEnds::None,
            BlockDelim::None,
            self.text,
            self.comments,
        )
    }

    /// Like `into_children` but skips over the specified end and delimiter nodes.
    pub fn into_block(self, ends: BlockEnds, delim: BlockDelim) -> TSNodeIterator<'a> {
        TSNodeIterator::from_children(
            self.cursor,
            self.node.id(),
            ends,
            delim,
            self.text,
            self.comments,
        )
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
        let span = node.as_span();
        Ok(Self::new(node.try_into()?, span))
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

trait TSCursorExt<'a> {
    fn get_next(
        &mut self,
        text: &'a [u8],
        comments: Rc<RefCell<Comments>>,
    ) -> Result<Option<(ts::Node<'a>, Option<&'a str>)>, ModelError>;
}

impl<'a> TSCursorExt<'a> for ts::TreeCursor<'a> {
    fn get_next(
        &mut self,
        text: &'a [u8],
        comments: Rc<RefCell<Comments>>,
    ) -> Result<Option<(ts::Node<'a>, Option<&'a str>)>, ModelError> {
        loop {
            let node = self.node();
            if node.is_error() || node.is_missing() {
                bail!(ModelError::parser(format!("Parser error {:?}", node)));
            } else if node.kind() == rules::COMMENT {
                add_comment(node, text, (*comments).borrow_mut())?;
                if !self.goto_next_sibling() {
                    return Ok(None);
                }
            } else if node.is_extra() {
                bail!(ModelError::parser(format!(
                    "Unexpected extra node {:?}",
                    node
                )));
            } else {
                return Ok(Some((node, self.field_name())));
            }
        }
    }
}

/// TSNodeIterator state.
#[derive(Debug, PartialEq)]
enum State {
    Enter,
    Open,
    Item,
    NextItem,
    Delim,
    NextDelim,
    Exhaust,
    Exit,
    Done,
}

/// An iterator over sibling nodes.
pub struct TSNodeIterator<'a> {
    cursor: Rc<RefCell<ts::TreeCursor<'a>>>,
    state: State,
    parent_id: usize,
    ends: BlockEnds,
    delim: BlockDelim,
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
        ends: BlockEnds,
        delim: BlockDelim,
        text: &'a [u8],
        comments: Rc<RefCell<Comments>>,
    ) -> Self {
        Self {
            cursor,
            state: State::Enter,
            parent_id,
            ends,
            delim,
            text,
            comments,
        }
    }

    /// Advances the iterator to the next non-comment node and returns `Some((node, field_name))`,
    /// or `None` if there are no more non-comment nodes. If this is an iterator over child nodes,
    /// then the iterator is returned to the parent node the first time this method returns `None`.
    fn advance(&mut self) -> Result<Option<(ts::Node<'a>, Option<&'a str>)>, ModelError> {
        loop {
            match self.state {
                State::Enter if (*self.cursor).borrow_mut().goto_first_child() => {
                    self.state = match self.ends {
                        BlockEnds::None => State::Item,
                        _ => State::Open,
                    };
                }
                State::Enter => {
                    // there are no children - the cursor should not have moved
                    assert_eq!((*self.cursor).borrow().node().id(), self.parent_id);
                    self.state = State::Done;
                }
                State::Open => {
                    let mut cursor = (*self.cursor).borrow_mut();
                    match cursor.get_next(self.text, self.comments.clone())? {
                        Some((node, _)) => {
                            match BlockEnds::get_open(node.kind()) {
                                BlockEnds::SingleQuotes if self.ends == BlockEnds::Quotes => {
                                    self.ends = BlockEnds::SingleQuotes;
                                }
                                BlockEnds::DoubleQuotes if self.ends == BlockEnds::Quotes => {
                                    self.ends = BlockEnds::DoubleQuotes;
                                }
                                actual if self.ends != actual => {
                                    bail!(ModelError::parser(format!(
                                        "Expected open symbol {:?} but found {:?}",
                                        self.ends, actual
                                    )));
                                }
                                _ => (),
                            }
                            if !cursor.goto_next_sibling() {
                                bail!(ModelError::parser(format!(
                                    "Expected close symbol {:?} but iterator is exhausted",
                                    self.ends,
                                )));
                            }
                            self.state = State::Item;
                        }
                        None => {
                            self.state = State::Exit;
                        }
                    }
                }
                State::Item | State::Delim => {
                    let mut cursor = (*self.cursor).borrow_mut();
                    let has_ends = self.ends != BlockEnds::None;
                    let (node, field_name) =
                        match cursor.get_next(self.text, self.comments.clone())? {
                            Some((node, _))
                                if has_ends && self.ends == BlockEnds::get_close(node.kind()) =>
                            {
                                if cursor.goto_next_sibling() {
                                    self.state = State::Exhaust;
                                } else {
                                    self.state = State::Exit;
                                }
                                continue;
                            }
                            Some(node_field) => node_field,
                            None if has_ends => bail!(ModelError::parser(format!(
                                "Expected close symbol {:?} but iterator is exhausted",
                                self.ends,
                            ))),
                            None => {
                                self.state = State::Exit;
                                continue;
                            }
                        };

                    if self.state == State::Delim {
                        let delim = BlockDelim::get(node.kind());

                        if self.delim != delim {
                            bail!(ModelError::parser(format!(
                                "Expected delimiter {:?} but found {:?}",
                                self.delim, delim
                            )));
                        }
                        self.state = State::NextItem;
                    } else {
                        if self.delim != BlockDelim::None {
                            self.state = State::NextDelim;
                        } else {
                            self.state = State::NextItem;
                        }
                        return Ok(Some((node, field_name)));
                    }
                }
                State::NextItem | State::NextDelim => {
                    let mut cursor = (*self.cursor).borrow_mut();
                    if cursor.goto_next_sibling() {
                        if self.state == State::NextDelim {
                            self.state = State::Delim;
                        } else {
                            self.state = State::Item;
                        }
                    } else if self.ends != BlockEnds::None {
                        bail!(ModelError::parser(format!(
                            "Expected close symbol {:?} but iterator is exhausted",
                            self.ends,
                        )));
                    } else {
                        self.state = State::Exit
                    }
                }
                State::Exhaust => {
                    match (*self.cursor)
                        .borrow_mut()
                        .get_next(self.text, self.comments.clone())?
                    {
                        Some((node, _)) => bail!(ModelError::parser(format!(
                            "Expected iterator to be exhausted but found {:?}",
                            node
                        ))),
                        None => self.state = State::Exit,
                    }
                }
                State::Exit => {
                    let mut cursor = (*self.cursor).borrow_mut();
                    if cursor.goto_next_sibling() {
                        bail!(ModelError::parser(format!(
                            "Expected iterator to be exhausted but found next node {:?}",
                            cursor.node()
                        )));
                    }
                    if !cursor.goto_parent() {
                        bail!(ModelError::parser(String::from(
                            "Could not return cursor to parent node",
                        )));
                    }
                    let parent_id = cursor.node().id();
                    if parent_id != self.parent_id {
                        bail!(ModelError::parser(format!(
                            "Node iterator returned to different parent {} than it started from {}",
                            parent_id, self.parent_id
                        )));
                    }
                    self.state = State::Done;
                }
                State::Done => return Ok(None),
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

    /// Asserts that there is a next node and it has the given kind.
    pub fn skip_terminal(&mut self, kind: &'a str) -> Result<(), ModelError> {
        match self.advance()? {
            Some((node, None)) if node.kind() == kind => Ok(()),
            Some((node, field)) => Err(report!(ModelError::parser(format!(
                "Expected next node to be a terminal with kind '{}' but was {:?} (field: {:?})",
                kind, node, field
            )))),
            None => Err(report!(ModelError::parser(format!(
                "Expected next node to be a terminal with kind '{}' but iterator is exhausted",
                kind
            )))),
        }
    }

    /// Returns `true` if there is a next node and it has the given kind. Returns `false` if there
    /// is no next node. Otherwise returns an error. This should only be called to skip a node at
    /// the end of a rule since it does consume the node.
    pub fn skip_optional_last(&mut self, kind: &'a str) -> Result<bool, ModelError> {
        match self.advance()? {
            Some((node, None)) if node.kind() == kind => Ok(true),
            Some((node, field)) => Err(report!(ModelError::parser(format!(
                "Expected next node to be a terminal with kind '{}' but was {:?} (field: {:?})",
                kind, node, field
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

    pub fn set_delim(&mut self, delim: BlockDelim) -> Result<(), ModelError> {
        match self.state {
            State::NextItem | State::NextDelim => self.delim = delim,
            _ => bail!(ModelError::parser(format!(
                "Cannot initiate list from state {:?}",
                self.state
            ))),
        }
        Ok(())
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
