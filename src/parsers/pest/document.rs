use crate::{
    ast::{Alias, Document, DocumentElement, DocumentSource, Import, Namespace, Struct, Version},
    parsers::pest::{PairExt, PairsExt, Rule},
};
use anyhow::{bail, Error, Result};
use pest::iterators::Pair;
use std::convert::TryFrom;

impl<'a> TryFrom<Pair<'a, Rule>> for Version {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        Ok(Self {
            identifier: pair.into_inner().next_str_into_node()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Namespace {
    type Error = Error;

    fn try_from(pair: Pair<'a, Rule>) -> Result<Self> {
        Ok(Self::Explicit(pair.try_into_string_node()?))
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Alias {
    type Error = Error;

    fn try_from(pair: Pair<'a, Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            from: inner.next_string_node()?,
            to: inner.next_string_node()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Import {
    type Error = Error;

    fn try_from(pair: Pair<'a, Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        let uri = inner.next_string_node()?;
        let namespace = if let Some(Rule::namespace) = inner.peek().map(|node| node.as_rule()) {
            Namespace::try_from(inner.next_pair()?)?
        } else {
            Namespace::try_from_uri(&uri.element)?
        };
        let aliases = inner.collect_nodes()?;
        Ok(Self {
            uri,
            namespace,
            aliases,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Struct {
    type Error = Error;

    fn try_from(pair: Pair<'a, Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        Ok(Self {
            name: inner.next_string_node()?,
            fields: inner.collect_nodes()?,
        })
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for DocumentElement {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let e = match pair.as_rule() {
            Rule::import => Self::Import(pair.try_into()?),
            Rule::structdef => Self::Struct(pair.try_into()?),
            Rule::task => Self::Task(pair.try_into()?),
            Rule::workflow => Self::Workflow(pair.try_into()?),
            _ => bail!("Invalid pair {}", pair),
        };
        Ok(e)
    }
}

impl<'a> TryFrom<Pair<'a, Rule>> for Document {
    type Error = Error;

    fn try_from(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        let doc = Self {
            source: DocumentSource::default(),
            version: inner.next_node()?,
            body: inner.collect_nodes()?,
        };
        doc.validate()?;
        Ok(doc)
    }
}
