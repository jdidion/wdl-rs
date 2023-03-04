use crate::{
    model::{Anchor, Comments, ModelError, Span},
    parsers::tree_sitter::syntax,
};
use error_stack::{bail, IntoReport, Report, Result, ResultExt};
use std::{cell::RefCell, rc::Rc, str::FromStr};
use tree_sitter as ts;

fn node_as_str<'a>(node: ts::Node<'a>, text: &'a [u8]) -> Result<&'a str, ModelError> {
    node.utf8_text(text)
        .into_report()
        .change_context(ModelError::parser(format!(
            "error getting node contents as str {:?}",
            node
        )))
}

#[derive(Debug)]
pub(super) struct TSNode<'a> {
    node: ts::Node<'a>,
    text: &'a [u8],
    comments: Rc<RefCell<Comments>>,
}

impl<'a> TSNode<'a> {
    pub fn new(
        node: ts::Node<'a>,
        text: &'a [u8],
        comments: Rc<RefCell<Comments>>,
    ) -> Result<Self, ModelError> {
        let cursor = &mut node.walk();
        let mut comments_mut = comments.borrow_mut();
        for child in node.named_children(cursor) {
            if child.kind() == syntax::COMMENT {
                let element = node_as_str(child, text)?;
                let comment = Anchor {
                    element: element.to_owned(),
                    span: child.into(),
                };
                comments_mut.try_insert(child.start_position().row, comment)?;
            }
        }
        Ok(Self {
            node,
            text,
            comments: comments.clone(),
        })
    }

    pub fn children<T: TryFrom<TSNode<'a>, Error = Report<ModelError>>>(
        &self,
    ) -> Result<Vec<T>, ModelError> {
        let cursor = &mut self.node.walk();
        self.node
            .named_children(cursor)
            .map(|node| TSNode::new(node, self.text, self.comments.clone()).unwrap())
            .map(|node| node.try_into())
            .collect()
    }

    pub fn child_anchors<T: TryFrom<TSNode<'a>, Error = Report<ModelError>>>(
        &self,
    ) -> Result<Vec<Anchor<T>>, ModelError> {
        self.children()
    }

    pub fn get_field(&mut self, name: &str) -> Option<TSNode<'a>> {
        self.node
            .child_by_field_name(name)
            .map(|node| TSNode::new(node, self.text, self.comments.clone()).unwrap())
    }

    pub fn field(&mut self, name: &str) -> Result<TSNode<'a>, ModelError> {
        if let Some(node) = self.get_field(name) {
            Ok(node)
        } else {
            bail!(ModelError::parser(format!(
                "Missing field {} in node {:?}",
                name, self.node
            )))
        }
    }

    pub fn field_node<T: TryFrom<TSNode<'a>, Error = Report<ModelError>>>(
        &mut self,
        name: &str,
    ) -> Result<Anchor<T>, ModelError> {
        let field_node = self.field(name)?;
        field_node.try_into()
    }

    pub fn get_field_node<T: TryFrom<TSNode<'a>, Error = Report<ModelError>>>(
        &mut self,
        name: &str,
    ) -> Result<Option<Anchor<T>>, ModelError> {
        let field_node = self.get_field(name);
        field_node.map(|node| node.try_into()).transpose()
    }

    pub fn field_boxed_node<T: TryFrom<TSNode<'a>, Error = Report<ModelError>>>(
        &mut self,
        name: &str,
    ) -> Result<Box<Anchor<T>>, ModelError> {
        Ok(Box::new(self.field_node(name)?))
    }

    pub fn field_string(&mut self, name: &str) -> Result<String, ModelError> {
        let ts_node = self.field(name)?;
        ts_node.try_into().map(|s: &str| s.to_owned())
    }

    pub fn field_string_node(&mut self, name: &str) -> Result<Anchor<String>, ModelError> {
        let ts_node = self.field(name)?;
        ts_node.try_into()
    }

    pub fn field_string_into_node<T: FromStr<Err = Report<ModelError>>>(
        &mut self,
        name: &str,
    ) -> Result<Anchor<T>, ModelError> {
        let ts_node = self.field(name)?;
        let element_str = ts_node.try_into()?;
        Ok(Anchor {
            element: T::from_str(element_str)?,
            span: ts_node.into(),
        })
    }

    pub fn field_child_nodes<T: TryFrom<TSNode<'a>, Error = Report<ModelError>>>(
        &mut self,
        name: &str,
    ) -> Result<Vec<Anchor<T>>, ModelError> {
        let field_node = self.field(name)?;
        field_node.children()
    }

    pub fn get_field_child_nodes<T: TryFrom<TSNode<'a>, Error = Report<ModelError>>>(
        &mut self,
        name: &str,
    ) -> Result<Vec<Anchor<T>>, ModelError> {
        if let Some(field_node) = self.get_field(name) {
            field_node.children()
        } else {
            Ok(Vec::new())
        }
    }

    pub fn clone_comments(&self) -> Rc<RefCell<Comments>> {
        self.comments.clone()
    }
}

impl<'a> From<TSNode<'a>> for Span {
    fn from(node: TSNode<'a>) -> Self {
        node.node.into()
    }
}

impl<'a> TryFrom<TSNode<'a>> for &'a str {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<&'a str, ModelError> {
        node_as_str(node.node, node.text)
    }
}

impl<'a> TryFrom<TSNode<'a>> for String {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        TryInto::<&str>::try_into(node).and_then(|s| Ok(s.to_owned()))
    }
}

impl<'a, T: TryFrom<TSNode<'a>, Error = Report<ModelError>>> TryFrom<TSNode<'a>> for Anchor<T> {
    type Error = Report<ModelError>;

    fn try_from(node: TSNode<'a>) -> Result<Self, ModelError> {
        let span = node.into();
        Ok(Self {
            element: node.try_into()?,
            span,
        })
    }
}
