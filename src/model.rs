use error_stack::{bail, ensure, IntoReport, Report, Result, ResultExt};
use regex::Regex;
use std::{
    cmp::Ordering,
    collections::{btree_map::Range as CommentIterator, BTreeMap, HashSet},
    fmt::{Display, Formatter, Result as FmtResult, Write},
    ops::Deref,
    ops::Range,
    path::PathBuf,
    str::FromStr,
};
use thiserror::Error;

/// Syntax errors that may be returned when creating model elements.
#[derive(Error, Debug)]
pub enum ModelError {
    #[error(
        "The parser allowed invalid syntax: {kind} {value}; this indicates a bug in the parser \
        grammar and should be reported"
    )]
    Grammar { kind: String, value: String },
    #[error("Invalid integer literal {0}")]
    Integer(String),
    #[error("Invalid float literal {0}")]
    Float(String),
    #[error("Invalid version identifier {0} (only WDL 1.x is supported)")]
    Version(String),
    #[error("Task {kind} contains more than one of the same element type {kind}")]
    TaskRepeatedElement { task: String, kind: String },
    #[error("Task {0} is missing required 'command' element")]
    TaskMissingCommand(String),
    #[error("Workflow {workflow} contains more than one of the same element type {kind}")]
    WorkflowRepeatedElement { workflow: String, kind: String },
    #[error("Document is missing at least one element of kind Struct, Task, or Workflow")]
    DocumentIncomplete,
    #[error("Document has more than one Workflow element")]
    DocumentMultipleWorkflows,
    #[error("Comment already exists for line {0}")]
    CommentRepeatedLine(usize),
}

impl ModelError {
    pub fn parser(msg: String) -> Self {
        Self::Grammar {
            kind: String::from("parser"),
            value: msg,
        }
    }
}

/// A position in the source document. Includes both 1D (byte offset) and 2D (line and column)
/// coordinates. Coordinates are zero-based, end-exclusive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    /// Line of the source file
    pub line: usize,
    /// Column of the source line
    pub column: usize,
    /// Absolute byte offset
    pub offset: usize,
}

impl Position {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }

    pub fn shift_left(&mut self, n: usize) {
        assert!(self.column >= n);
        assert!(self.offset >= n);
        self.column -= n;
        self.offset -= n;
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}:{}", self.line, self.column)
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.offset.cmp(&other.offset)
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// The span (start and end positions) in the source document from which a model element was derived.
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        if start < end {
            Self { start, end }
        } else {
            panic!("Span start position must be less than end position")
        }
    }

    pub fn from_range(left: &Self, right: &Self) -> Self {
        Self {
            start: left.start.clone(),
            end: right.end.clone(),
        }
    }

    pub fn from_components(
        start_line: usize,
        start_column: usize,
        start_offset: usize,
        end_line: usize,
        end_column: usize,
        end_offset: usize,
    ) -> Self {
        Self {
            start: Position::new(start_line, start_column, start_offset),
            end: Position::new(end_line, end_column, end_offset),
        }
    }

    pub fn len(&self) -> usize {
        self.end.offset - self.start.offset - 1
    }

    pub fn trim_end(&mut self, n: usize) {
        self.end.shift_left(n)
    }
}

impl Display for Span {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}..{}", self.start, self.end)
    }
}

impl Into<Range<usize>> for Span {
    fn into(self) -> Range<usize> {
        self.start.offset..self.end.offset
    }
}

pub trait InnerSpan {
    fn get_inner_span(&self) -> Option<Span>;
}

impl<T> InnerSpan for Vec<Anchor<T>> {
    fn get_inner_span(&self) -> Option<Span> {
        match self.len() {
            0 => None,
            1 => self.first().map(|a| a.span.clone()),
            _ => {
                let left = self.first().unwrap();
                let right = self.last().unwrap();
                Some(Span::from_range(&left.span, &right.span))
            }
        }
    }
}

#[derive(Debug)]
pub struct SourceFragment(pub String);

impl Display for SourceFragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

/// Wrapper around a model element of type `T` that also encapsulates source code span information.
/// `Deref`s to `T`.
#[derive(Debug, PartialEq)]
pub struct Anchor<T> {
    /// The model element.
    element: T,
    /// The span of the source document from which the element was derived.    
    pub span: Span,
}

impl<T> Anchor<T> {
    pub fn new(element: T, span: Span) -> Self {
        Self { element, span }
    }
}

impl<'a, T> Deref for Anchor<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.element
    }
}

impl<T: Display> Display for Anchor<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} ({})", self.element, self.span)
    }
}

/// Mapping of source lines to comments. In WDL there are no block comments, so there can be a
/// maximum of one comment per line.
#[derive(Debug, PartialEq)]
pub struct Comments(BTreeMap<usize, Anchor<String>>);

impl Comments {
    /// Adds a comment to the map. Returns a `ModelError::DuplicateComment` if a comment has
    /// already been added at `line`.
    pub fn try_insert(&mut self, line: usize, comment: Anchor<String>) -> Result<(), ModelError> {
        ensure!(
            !self.0.contains_key(&line),
            ModelError::CommentRepeatedLine(line)
        );
        self.0.insert(line, comment);
        Ok(())
    }

    /// Returns the comment on the specified line, or `None` if there is no comment on that line.
    pub fn get(&self, line: usize) -> Option<&Anchor<String>> {
        self.0.get(&line)
    }

    //// Returns an iterator over all comments in line order.
    pub fn values(&self) -> impl Iterator<Item = &Anchor<String>> {
        self.0.values()
    }

    /// Returns an iterator over comments within the given source line range (start-inclusive,
    /// end-exclusive).
    pub fn range(&self, lines: Range<usize>) -> CommentIterator<usize, Anchor<String>> {
        self.0.range(lines)
    }
}

impl Default for Comments {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[derive(Debug, PartialEq)]
pub enum Integer {
    Decimal(i64),
    Octal(i64),
    Hex(i64),
}

impl Integer {
    pub fn negate(&self) -> Self {
        match self {
            Self::Decimal(i) => Self::Decimal(i * -1),
            Self::Octal(i) => Self::Octal(i * -1),
            Self::Hex(i) => Self::Hex(i * -1),
        }
    }
}

impl FromStr for Integer {
    type Err = Report<ModelError>;

    fn from_str(s: &str) -> Result<Self, ModelError> {
        if s.starts_with("0") && s.len() > 1 {
            match s.chars().nth(1).unwrap() {
                'x' | 'X' => {
                    let int_value = i64::from_str_radix(&s[2..], 16)
                        .into_report()
                        .change_context(ModelError::Integer(s.to_owned()))?;
                    Ok(Self::Hex(int_value))
                }
                '0'..='7' => {
                    let int_value = i64::from_str_radix(&s[1..], 8)
                        .into_report()
                        .change_context(ModelError::Integer(s.to_owned()))?;
                    Ok(Self::Octal(int_value))
                }
                _ => bail!(ModelError::Grammar {
                    kind: String::from("integer"),
                    value: s.to_owned(),
                }),
            }
        } else {
            let int_value = s
                .parse::<i64>()
                .into_report()
                .change_context(ModelError::Integer(s.to_owned()))?;
            Ok(Self::Decimal(int_value))
        }
    }
}

impl Into<i64> for Integer {
    fn into(self) -> i64 {
        match self {
            Integer::Decimal(i) => i,
            Integer::Octal(i) => i,
            Integer::Hex(i) => i,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Float {
    Decimal(f64),
    Scientific(f64),
}

impl Float {
    pub fn negate(&self) -> Self {
        match self {
            Self::Decimal(f) => Self::Decimal(f * -1.0),
            Self::Scientific(f) => Self::Scientific(f * -1.0),
        }
    }
}

impl FromStr for Float {
    type Err = Report<ModelError>;

    fn from_str(s: &str) -> Result<Self, ModelError> {
        let float_value = s
            .parse()
            .into_report()
            .change_context(ModelError::Float(s.to_owned()))?;
        let f = if s.contains('e') || s.contains('E') {
            Self::Scientific(float_value)
        } else {
            Self::Decimal(float_value)
        };
        Ok(f)
    }
}

impl Into<f64> for Float {
    fn into(self) -> f64 {
        match self {
            Float::Decimal(f) => f,
            Float::Scientific(f) => f,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum StringPart {
    Content(String),
    Escape(String),
    Placeholder(Expression),
}

impl Display for StringPart {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            StringPart::Content(s) => write!(f, "{}", s),
            StringPart::Escape(s) => write!(f, "{}", s),
            StringPart::Placeholder(_) => write!(f, "~{{..}}"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct StringLiteral {
    pub parts: Vec<Anchor<StringPart>>,
}

impl Display for StringLiteral {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_char('"')?;
        for part in self.parts.iter() {
            write!(f, "{}", *part)?;
        }
        f.write_char('"')?;
        Ok(())
    }
}

impl InnerSpan for StringLiteral {
    fn get_inner_span(&self) -> Option<Span> {
        self.parts.get_inner_span()
    }
}

#[derive(Debug, PartialEq)]
pub struct ArrayLiteral {
    pub elements: Vec<Anchor<Expression>>,
}

impl InnerSpan for ArrayLiteral {
    fn get_inner_span(&self) -> Option<Span> {
        self.elements.get_inner_span()
    }
}

#[derive(Debug, PartialEq)]
pub struct MapEntry {
    pub key: Anchor<Expression>,
    pub value: Anchor<Expression>,
}

#[derive(Debug, PartialEq)]
pub struct MapLiteral {
    pub entries: Vec<Anchor<MapEntry>>,
}

impl InnerSpan for MapLiteral {
    fn get_inner_span(&self) -> Option<Span> {
        self.entries.get_inner_span()
    }
}

#[derive(Debug, PartialEq)]
pub struct PairLiteral {
    pub left: InnerExpression,
    pub right: InnerExpression,
}

impl InnerSpan for PairLiteral {
    fn get_inner_span(&self) -> Option<Span> {
        Some(Span::from_range(&self.left.span, &self.right.span))
    }
}

#[derive(Debug, PartialEq)]
pub struct ObjectField {
    pub name: Anchor<String>,
    pub expression: Anchor<Expression>,
}

#[derive(Debug, PartialEq)]
pub struct ObjectLiteral {
    pub type_name: Anchor<String>,
    pub fields: Vec<Anchor<ObjectField>>,
}

impl InnerSpan for ObjectLiteral {
    fn get_inner_span(&self) -> Option<Span> {
        let field_span = self.fields.get_inner_span().unwrap();
        Some(Span::from_range(&self.type_name.span, &field_span))
    }
}

const POS: &str = "+";
const NEG: &str = "-";
const NOT: &str = "!";

#[derive(Debug, PartialEq)]
pub enum UnaryOperator {
    Pos,
    Neg,
    Not,
}

impl FromStr for UnaryOperator {
    type Err = Report<ModelError>;

    fn from_str(s: &str) -> Result<Self, ModelError> {
        let oper = match s {
            POS => Self::Pos,
            NEG => Self::Neg,
            NOT => Self::Not,
            _ => bail!(ModelError::Grammar {
                kind: String::from("unary operator"),
                value: s.to_owned()
            }),
        };
        Ok(oper)
    }
}

impl Display for UnaryOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let s = match self {
            Self::Pos => POS,
            Self::Neg => NEG,
            Self::Not => NOT,
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, PartialEq)]
pub struct Unary {
    pub operator: UnaryOperator,
    pub expression: InnerExpression,
}

impl InnerSpan for Unary {
    fn get_inner_span(&self) -> Option<Span> {
        Some(self.expression.span.clone())
    }
}

const ADD: &str = "+";
const SUB: &str = "-";
const MUL: &str = "*";
const DIV: &str = "/";
const MOD: &str = "%";
const GT: &str = ">";
const LT: &str = "<";
const GTE: &str = ">=";
const LTE: &str = "<=";
const EQ: &str = "==";
const NEQ: &str = "!=";
const AND: &str = "&&";
const OR: &str = "||";

#[derive(Debug, PartialEq)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
    Neq,
    And,
    Or,
}

impl FromStr for BinaryOperator {
    type Err = Report<ModelError>;

    fn from_str(s: &str) -> Result<Self, ModelError> {
        let oper = match s {
            ADD => Self::Add,
            SUB => Self::Sub,
            MUL => Self::Mul,
            DIV => Self::Div,
            MOD => Self::Mod,
            GT => Self::Gt,
            LT => Self::Lt,
            GTE => Self::Gte,
            LTE => Self::Lte,
            EQ => Self::Eq,
            NEQ => Self::Neq,
            AND => Self::And,
            OR => Self::Or,
            _ => bail!(ModelError::Grammar {
                kind: String::from("binary operator"),
                value: s.to_owned()
            }),
        };
        Ok(oper)
    }
}

impl Display for BinaryOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let s = match self {
            Self::Add => ADD,
            Self::Sub => SUB,
            Self::Mul => MUL,
            Self::Div => DIV,
            Self::Mod => MOD,
            Self::Gt => GT,
            Self::Lt => LT,
            Self::Gte => GTE,
            Self::Lte => LTE,
            Self::Eq => EQ,
            Self::Neq => NEQ,
            Self::And => AND,
            Self::Or => OR,
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, PartialEq)]
pub struct Binary {
    pub operator: BinaryOperator,
    pub left: InnerExpression,
    pub right: InnerExpression,
}

impl InnerSpan for Binary {
    fn get_inner_span(&self) -> Option<Span> {
        Some(Span::from_range(&self.left.span, &self.right.span))
    }
}

#[derive(Debug, PartialEq)]
pub struct Apply {
    pub name: Anchor<String>,
    pub arguments: Vec<Anchor<Expression>>,
}

impl InnerSpan for Apply {
    fn get_inner_span(&self) -> Option<Span> {
        let argument_span = self.arguments.get_inner_span().unwrap();
        Some(Span::from_range(&self.name.span, &argument_span))
    }
}

#[derive(Debug, PartialEq)]
pub enum AccessOperation {
    Index(Expression),
    Field(String),
}

#[derive(Debug, PartialEq)]
pub struct Access {
    pub collection: InnerExpression,
    pub accesses: Vec<Anchor<AccessOperation>>,
}

impl InnerSpan for Access {
    fn get_inner_span(&self) -> Option<Span> {
        let access_span = self.accesses.get_inner_span().unwrap();
        Some(Span::from_range(&self.collection.span, &access_span))
    }
}

#[derive(Debug, PartialEq)]
pub struct Ternary {
    pub condition: InnerExpression,
    pub true_branch: InnerExpression,
    pub false_branch: InnerExpression,
}

impl InnerSpan for Ternary {
    fn get_inner_span(&self) -> Option<Span> {
        Some(Span::from_range(
            &self.condition.span,
            &self.false_branch.span,
        ))
    }
}

pub type InnerExpression = Box<Anchor<Expression>>;

#[derive(Debug, PartialEq)]
pub enum Expression {
    None,
    Boolean(bool),
    Int(Integer),
    Float(Float),
    String(StringLiteral),
    Array(ArrayLiteral),
    Map(MapLiteral),
    Pair(PairLiteral),
    Object(ObjectLiteral),
    Unary(Unary),
    Binary(Binary),
    Apply(Apply),
    Access(Access),
    Ternary(Ternary),
    Group(InnerExpression),
    Identifier(String),
}

impl InnerSpan for Expression {
    fn get_inner_span(&self) -> Option<Span> {
        match self {
            Self::String(s) => s.get_inner_span(),
            Self::Array(a) => a.get_inner_span(),
            Self::Map(m) => m.get_inner_span(),
            Self::Pair(p) => p.get_inner_span(),
            Self::Object(o) => o.get_inner_span(),
            Self::Unary(u) => u.get_inner_span(),
            Self::Binary(b) => b.get_inner_span(),
            Self::Apply(a) => a.get_inner_span(),
            Self::Access(a) => a.get_inner_span(),
            Self::Ternary(t) => t.get_inner_span(),
            Self::Group(g) => g.get_inner_span(),
            _ => None,
        }
    }
}

pub type InnerType = Box<Anchor<Type>>;

#[derive(Debug, PartialEq)]
pub enum Type {
    Boolean,
    Int,
    Float,
    String,
    File,
    Array { item: InnerType, non_empty: bool },
    Map { key: InnerType, value: InnerType },
    Pair { left: InnerType, right: InnerType },
    Object,
    User(String),
    Optional(InnerType),
}

#[derive(Debug, PartialEq)]
pub struct UnboundDeclaration {
    pub type_: Anchor<Type>,
    pub name: Anchor<String>,
}

impl InnerSpan for UnboundDeclaration {
    fn get_inner_span(&self) -> Option<Span> {
        Some(Span::from_range(&self.type_.span, &self.name.span))
    }
}

#[derive(Debug, PartialEq)]
pub struct BoundDeclaration {
    pub type_: Anchor<Type>,
    pub name: Anchor<String>,
    pub expression: Anchor<Expression>,
}

impl InnerSpan for BoundDeclaration {
    fn get_inner_span(&self) -> Option<Span> {
        Some(Span::from_range(&self.type_.span, &self.expression.span))
    }
}

/// Enumeration of sources from which a WDL document may be loaded.
#[derive(Clone, Debug, PartialEq)]
pub enum DocumentSource {
    File(PathBuf),
    Uri(String),
    Unknown,
}

impl Default for DocumentSource {
    fn default() -> Self {
        Self::Unknown
    }
}

impl Display for DocumentSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            DocumentSource::File(path) => write!(f, "{}", path.display()),
            DocumentSource::Uri(uri) => write!(f, "{}", uri),
            DocumentSource::Unknown => write!(f, "<unknown>"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum VersionIdentifier {
    V1_0,
    V1_1,
}

impl FromStr for VersionIdentifier {
    type Err = Report<ModelError>;

    fn from_str(s: &str) -> Result<Self, ModelError> {
        match s {
            "1.0" => Ok(VersionIdentifier::V1_0),
            "1.1" => Ok(VersionIdentifier::V1_1),
            _ => bail!(ModelError::Version(s.to_owned())),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Version {
    pub identifier: Anchor<VersionIdentifier>,
}

#[derive(Debug, PartialEq)]
pub enum Namespace {
    Explicit(Anchor<String>),
    Implicit(String),
}

impl Namespace {
    pub fn from_uri<S: AsRef<str>>(uri: S) -> Self {
        let s = uri.as_ref();
        let ns = match Regex::new(r".*/(.+?)(?:\.wdl)?").unwrap().captures(s) {
            Some(cap) => cap[1].to_owned(),
            None if s.ends_with(".wdl") => s[0..s.len() - 4].to_owned(),
            None => s.to_owned(),
        };
        Self::Implicit(ns.to_owned())
    }
}

#[derive(Debug, PartialEq)]
pub struct Alias {
    pub from: Anchor<String>,
    pub to: Anchor<String>,
}

#[derive(Debug, PartialEq)]
pub struct Import {
    pub uri: Anchor<String>,
    pub namespace: Namespace,
    pub aliases: Vec<Anchor<Alias>>,
}

#[derive(Debug, PartialEq)]
pub struct Struct {
    pub name: Anchor<String>,
    pub fields: Vec<Anchor<UnboundDeclaration>>,
}

#[derive(Debug, PartialEq)]
pub enum InputDeclaration {
    Bound(BoundDeclaration),
    Unbound(UnboundDeclaration),
}

impl InnerSpan for InputDeclaration {
    fn get_inner_span(&self) -> Option<Span> {
        match self {
            Self::Bound(decl) => decl.get_inner_span(),
            Self::Unbound(decl) => decl.get_inner_span(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Input {
    pub declarations: Vec<Anchor<InputDeclaration>>,
}

#[derive(Debug, PartialEq)]
pub struct Output {
    pub declarations: Vec<Anchor<BoundDeclaration>>,
}

#[derive(Debug, PartialEq)]
pub enum MetaStringPart {
    Content(String),
    Escape(String),
}

#[derive(Debug, PartialEq)]
pub struct MetaString {
    pub parts: Vec<Anchor<MetaStringPart>>,
}

#[derive(Debug, PartialEq)]
pub struct MetaArray {
    pub elements: Vec<Anchor<MetaValue>>,
}

#[derive(Debug, PartialEq)]
pub struct MetaObjectField {
    pub name: Anchor<String>,
    pub value: Anchor<MetaValue>,
}

#[derive(Debug, PartialEq)]
pub struct MetaObject {
    pub fields: Vec<Anchor<MetaObjectField>>,
}

#[derive(Debug, PartialEq)]
pub enum MetaValue {
    Null,
    Boolean(bool),
    Int(Integer),
    Float(Float),
    String(MetaString),
    Array(MetaArray),
    Object(MetaObject),
}

#[derive(Debug, PartialEq)]
pub struct MetaAttribute {
    pub name: Anchor<String>,
    pub value: Anchor<MetaValue>,
}

#[derive(Debug, PartialEq)]
pub struct Meta {
    pub attributes: Vec<Anchor<MetaAttribute>>,
}

#[derive(Debug, PartialEq)]
pub struct ParameterMeta {
    pub attributes: Vec<Anchor<MetaAttribute>>,
}

#[derive(Debug, PartialEq)]
pub struct Command {
    pub parts: Vec<Anchor<StringPart>>,
}

#[derive(Debug, PartialEq)]
pub struct RuntimeAttribute {
    pub name: Anchor<String>,
    pub expression: Anchor<Expression>,
}

impl InnerSpan for RuntimeAttribute {
    fn get_inner_span(&self) -> Option<Span> {
        Some(Span::from_range(&self.name.span, &self.expression.span))
    }
}

#[derive(Debug, PartialEq)]
pub struct Runtime {
    pub attributes: Vec<Anchor<RuntimeAttribute>>,
}

#[derive(Debug, PartialEq)]
pub enum TaskElement {
    Input(Input),
    Output(Output),
    Declaration(BoundDeclaration),
    Command(Command),
    Runtime(Runtime),
    Meta(Meta),
    ParameterMeta(ParameterMeta),
}

impl TaskElement {
    pub fn kind(&self) -> &str {
        match self {
            TaskElement::Input(_) => "input",
            TaskElement::Output(_) => "output",
            TaskElement::Declaration(_) => "declaration",
            TaskElement::Command(_) => "command",
            TaskElement::Runtime(_) => "runtime",
            TaskElement::Meta(_) => "meta",
            TaskElement::ParameterMeta(_) => "parameter_meta",
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Task {
    pub name: Anchor<String>,
    pub body: Vec<Anchor<TaskElement>>,
}

impl Task {
    pub fn validate(&self) -> Result<(), ModelError> {
        let mut seen = HashSet::with_capacity(6);
        for element in self.body.iter() {
            let must_be_unique = match element.deref() {
                TaskElement::Input(_)
                | TaskElement::Output(_)
                | TaskElement::Command(_)
                | TaskElement::Runtime(_)
                | TaskElement::Meta(_)
                | TaskElement::ParameterMeta(_) => true,
                _ => false,
            };
            if must_be_unique {
                let kind = element.kind();
                ensure!(
                    !seen.contains(kind),
                    Report::from(ModelError::TaskRepeatedElement {
                        task: self.name.clone(),
                        kind: kind.to_owned()
                    })
                    .attach_printable(element.span.clone())
                );
                seen.insert(kind);
            }
        }
        ensure!(
            seen.contains("command"),
            ModelError::TaskMissingCommand(self.name.clone())
        );
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct QualifiedIdentifier {
    pub parts: Vec<Anchor<String>>,
}

impl InnerSpan for QualifiedIdentifier {
    fn get_inner_span(&self) -> Option<Span> {
        self.parts.get_inner_span()
    }
}

#[derive(Debug, PartialEq)]
pub struct CallInput {
    pub name: Anchor<String>,
    pub expression: Option<Anchor<Expression>>,
}

impl InnerSpan for CallInput {
    fn get_inner_span(&self) -> Option<Span> {
        if self.expression.is_some() {
            Some(Span::from_range(
                &self.name.span,
                &self.expression.as_ref().unwrap().span,
            ))
        } else {
            Some(self.name.span.clone())
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Call {
    pub target: Anchor<QualifiedIdentifier>,
    pub alias: Option<Anchor<String>>,
    pub inputs: Option<Vec<Anchor<CallInput>>>,
}

impl InnerSpan for Call {
    fn get_inner_span(&self) -> Option<Span> {
        if self.inputs.is_some() && !self.inputs.as_ref().unwrap().is_empty() {
            Some(Span::from_range(
                &self.target.span,
                &self.inputs.as_ref().unwrap().get_inner_span().unwrap(),
            ))
        } else if self.alias.is_some() {
            Some(Span::from_range(
                &self.target.span,
                &self.alias.as_ref().unwrap().span,
            ))
        } else {
            Some(self.target.span.clone())
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Scatter {
    pub name: Anchor<String>,
    pub expression: Anchor<Expression>,
    pub body: Vec<Anchor<WorkflowNestedElement>>,
}

#[derive(Debug, PartialEq)]
pub struct Conditional {
    pub expression: Anchor<Expression>,
    pub body: Vec<Anchor<WorkflowNestedElement>>,
}

#[derive(Debug, PartialEq)]
pub enum WorkflowNestedElement {
    Declaration(BoundDeclaration),
    Call(Call),
    Scatter(Scatter),
    Conditional(Conditional),
}

#[derive(Debug, PartialEq)]
pub enum WorkflowElement {
    Input(Input),
    Output(Output),
    Declaration(BoundDeclaration),
    Call(Call),
    Scatter(Scatter),
    Conditional(Conditional),
    Meta(Meta),
    ParameterMeta(Meta),
}

impl WorkflowElement {
    pub fn kind(&self) -> &str {
        match self {
            WorkflowElement::Input(_) => "input",
            WorkflowElement::Output(_) => "output",
            WorkflowElement::Declaration(_) => "declaration",
            WorkflowElement::Call(_) => "call",
            WorkflowElement::Scatter(_) => "scatter",
            WorkflowElement::Conditional(_) => "conditional",
            WorkflowElement::Meta(_) => "meta",
            WorkflowElement::ParameterMeta(_) => "parameter_meta",
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Workflow {
    pub name: Anchor<String>,
    pub body: Vec<Anchor<WorkflowElement>>,
}

impl Workflow {
    pub fn validate(&self) -> Result<(), ModelError> {
        let mut seen = HashSet::with_capacity(4);
        for element in self.body.iter() {
            match element.deref() {
                WorkflowElement::Input(_)
                | WorkflowElement::Output(_)
                | WorkflowElement::Meta(_)
                | WorkflowElement::ParameterMeta(_) => {
                    let kind = element.kind();
                    ensure!(
                        !seen.contains(&kind),
                        Report::from(ModelError::WorkflowRepeatedElement {
                            workflow: self.name.clone(),
                            kind: kind.to_owned()
                        })
                        .attach_printable(element.span.clone())
                    );
                    seen.insert(kind);
                }
                _ => (),
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub enum DocumentElement {
    Import(Import),
    Struct(Struct),
    Task(Task),
    Workflow(Workflow),
}

#[derive(Debug, PartialEq)]
pub struct Document {
    pub source: DocumentSource,
    pub version: Anchor<Version>,
    pub body: Vec<Anchor<DocumentElement>>,
    pub comments: Comments,
}

impl Document {
    pub fn validate(&self) -> Result<(), ModelError> {
        let mut element_count = 0;
        let mut workflow_count = 0;
        for element in self.body.iter() {
            match element.deref() {
                DocumentElement::Task(task) => {
                    task.validate().attach_printable(element.span.clone())?;
                    element_count += 1;
                }
                DocumentElement::Workflow(workflow) => {
                    workflow.validate().attach_printable(element.span.clone())?;
                    element_count += 1;
                    workflow_count += 1;
                }
                DocumentElement::Struct(_) => {
                    element_count += 1;
                }
                _ => (),
            }
        }
        ensure!(element_count > 0, ModelError::DocumentIncomplete);
        ensure!(workflow_count <= 1, ModelError::DocumentMultipleWorkflows);
        Ok(())
    }

    pub fn body_iter(&self) -> impl Iterator<Item = &DocumentElement> {
        self.body.iter().map(|e| (*e).deref())
    }

    /// Returns this Document's Workflow if it contains one, or its Task if it contains exactly
    /// one, otherwise None.
    pub fn get_primary_element(&self) -> Option<&DocumentElement> {
        self.body_iter()
            .find(|e| match (*e).deref() {
                &DocumentElement::Workflow(_) => true,
                _ => false,
            })
            .or_else(|| {
                let tasks: Vec<_> = self
                    .body_iter()
                    .filter(|e| match (*e).deref() {
                        &DocumentElement::Task(_) => true,
                        _ => false,
                    })
                    .collect();
                if tasks.len() == 1 {
                    tasks.first().map(|e| *e)
                } else {
                    None
                }
            })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    pub fn test_comprehensive(doc: Document) {
        assert_eq!(
            doc.version,
            Anchor::new(
                Version {
                    identifier: Anchor::new(
                        VersionIdentifier::V1_1,
                        Span::from_components(0, 8, 8, 0, 11, 11)
                    )
                },
                Span::from_components(0, 0, 0, 0, 11, 11)
            )
        );
        let mut body = doc.body.into_iter();
        assert_eq!(
            body.next().unwrap(),
            Anchor::new(
                DocumentElement::Import(Import {
                    uri: Anchor::new(
                        "local.wdl".to_owned(),
                        Span::from_components(2, 7, 20, 2, 18, 31)
                    ),
                    namespace: Namespace::Implicit("local".to_owned()),
                    aliases: vec![Anchor::new(
                        Alias {
                            from: Anchor::new(
                                "Foo".to_owned(),
                                Span::from_components(2, 25, 38, 2, 28, 41)
                            ),
                            to: Anchor::new(
                                "Bar".to_owned(),
                                Span::from_components(2, 32, 45, 2, 35, 48)
                            ),
                        },
                        Span::from_components(2, 19, 32, 2, 35, 48),
                    )]
                }),
                Span::from_components(2, 0, 13, 2, 35, 48)
            )
        );
        assert_eq!(
            body.next().unwrap(),
            Anchor::new(
                DocumentElement::Import(Import {
                    uri: Anchor::new(
                        "https://example.com/remote.wdl".to_owned(),
                        Span::from_components(3, 7, 56, 3, 39, 88)
                    ),
                    namespace: Namespace::Explicit(Anchor::new(
                        "remote".to_owned(),
                        Span::from_components(3, 43, 92, 3, 49, 98)
                    )),
                    aliases: vec![Anchor::new(
                        Alias {
                            from: Anchor::new(
                                "Baz".to_owned(),
                                Span::from_components(3, 56, 105, 3, 59, 108)
                            ),
                            to: Anchor::new(
                                "Blorf".to_owned(),
                                Span::from_components(3, 63, 112, 3, 68, 117)
                            ),
                        },
                        Span::from_components(3, 50, 99, 3, 68, 117),
                    )]
                }),
                Span::from_components(3, 0, 49, 3, 68, 117)
            )
        );
        assert_eq!(
            body.next().unwrap(),
            Anchor::new(
                DocumentElement::Struct(Struct {
                    name: Anchor::new(
                        "Example1".to_owned(),
                        Span::from_components(5, 7, 126, 5, 15, 134),
                    ),
                    fields: vec![
                        Anchor::new(
                            UnboundDeclaration {
                                type_: Anchor::new(
                                    Type::Float,
                                    Span::from_components(6, 4, 141, 6, 9, 146)
                                ),
                                name: Anchor::new(
                                    "f".to_owned(),
                                    Span::from_components(6, 10, 147, 6, 11, 148)
                                )
                            },
                            Span::from_components(6, 4, 141, 6, 11, 148)
                        ),
                        Anchor::new(
                            UnboundDeclaration {
                                type_: Anchor::new(
                                    Type::Map {
                                        key: Box::new(Anchor::new(
                                            Type::String,
                                            Span::from_components(7, 8, 157, 7, 14, 163)
                                        )),
                                        value: Box::new(Anchor::new(
                                            Type::Int,
                                            Span::from_components(7, 16, 165, 7, 19, 168)
                                        ))
                                    },
                                    Span::from_components(7, 4, 153, 7, 20, 169)
                                ),
                                name: Anchor::new(
                                    "m".to_owned(),
                                    Span::from_components(7, 21, 170, 7, 22, 171)
                                )
                            },
                            Span::from_components(7, 4, 153, 7, 22, 171)
                        )
                    ],
                }),
                Span::from_components(5, 0, 119, 8, 1, 173)
            )
        );
        assert_eq!(
            body.next().unwrap(),
            Anchor::new(
                DocumentElement::Struct(Struct {
                    name: Anchor::new(
                        "Example2".to_owned(),
                        Span::from_components(10, 7, 182, 10, 15, 190),
                    ),
                    fields: vec![
                        Anchor::new(
                            UnboundDeclaration {
                                type_: Anchor::new(
                                    Type::String,
                                    Span::from_components(11, 4, 197, 11, 10, 203)
                                ),
                                name: Anchor::new(
                                    "s".to_owned(),
                                    Span::from_components(11, 11, 204, 11, 12, 205)
                                )
                            },
                            Span::from_components(11, 4, 197, 11, 12, 205)
                        ),
                        Anchor::new(
                            UnboundDeclaration {
                                type_: Anchor::new(
                                    Type::Optional(Box::new(Anchor::new(
                                        Type::Int,
                                        Span::from_components(12, 4, 210, 12, 7, 213)
                                    ))),
                                    Span::from_components(12, 4, 210, 12, 8, 214)
                                ),
                                name: Anchor::new(
                                    "i".to_owned(),
                                    Span::from_components(12, 9, 215, 12, 10, 216)
                                )
                            },
                            Span::from_components(12, 4, 210, 12, 10, 216)
                        ),
                        Anchor::new(
                            UnboundDeclaration {
                                type_: Anchor::new(
                                    Type::Array {
                                        item: Box::new(Anchor::new(
                                            Type::Optional(Box::new(Anchor::new(
                                                Type::File,
                                                Span::from_components(13, 10, 227, 13, 14, 231)
                                            ))),
                                            Span::from_components(13, 10, 227, 13, 15, 232)
                                        )),
                                        non_empty: true
                                    },
                                    Span::from_components(13, 4, 221, 13, 17, 234)
                                ),
                                name: Anchor::new(
                                    "a".to_owned(),
                                    Span::from_components(13, 18, 235, 13, 19, 236)
                                )
                            },
                            Span::from_components(13, 4, 221, 13, 19, 236)
                        ),
                        Anchor::new(
                            UnboundDeclaration {
                                type_: Anchor::new(
                                    Type::User("Example1".to_owned()),
                                    Span::from_components(14, 4, 241, 14, 12, 249)
                                ),
                                name: Anchor::new(
                                    "e".to_owned(),
                                    Span::from_components(14, 13, 250, 14, 14, 251)
                                )
                            },
                            Span::from_components(14, 4, 241, 14, 14, 251)
                        )
                    ],
                }),
                Span::from_components(10, 0, 175, 15, 1, 253)
            )
        );
        assert_eq!(
            body.next().unwrap(),
            Anchor::new(
                DocumentElement::Workflow(Workflow {
                    name: Anchor::new(
                        "Workflow1".to_owned(),
                        Span::from_components(17, 9, 264, 17, 18, 273)
                    ),
                    body: vec![
                        Anchor::new(
                            WorkflowElement::Input(Input {
                                declarations: vec![
                                    Anchor::new(
                                        InputDeclaration::Unbound(UnboundDeclaration {
                                            type_: Anchor::new(
                                                Type::String,
                                                Span::from_components(19, 8, 296, 19, 14, 302)
                                            ),
                                            name: Anchor::new(
                                                "s".to_owned(),
                                                Span::from_components(19, 15, 303, 19, 16, 304)
                                            )
                                        }),
                                        Span::from_components(19, 8, 296, 19, 16, 304)
                                    ),
                                    Anchor::new(
                                        InputDeclaration::Bound(BoundDeclaration {
                                            type_: Anchor::new(
                                                Type::Int,
                                                Span::from_components(20, 8, 313, 20, 11, 316)
                                            ),
                                            name: Anchor::new(
                                                "i".to_owned(),
                                                Span::from_components(20, 12, 317, 20, 13, 318)
                                            ),
                                            expression: Anchor::new(
                                                Expression::Int(Integer::Decimal(0)),
                                                Span::from_components(20, 16, 321, 20, 17, 322)
                                            ),
                                        }),
                                        Span::from_components(20, 8, 313, 20, 17, 322)
                                    ),
                                    Anchor::new(
                                        InputDeclaration::Unbound(UnboundDeclaration {
                                            type_: Anchor::new(
                                                Type::Optional(Box::new(Anchor::new(
                                                    Type::User("Example2".to_owned()),
                                                    Span::from_components(21, 8, 331, 21, 16, 339)
                                                ))),
                                                Span::from_components(21, 8, 331, 21, 17, 340)
                                            ),
                                            name: Anchor::new(
                                                "ex".to_owned(),
                                                Span::from_components(21, 18, 341, 21, 20, 343)
                                            ),
                                        }),
                                        Span::from_components(21, 8, 331, 21, 20, 343)
                                    )
                                ]
                            }),
                            Span::from_components(18, 4, 280, 22, 5, 349)
                        ),
                        Anchor::new(
                            WorkflowElement::Declaration(BoundDeclaration {
                                type_: Anchor::new(
                                    Type::Float,
                                    Span::from_components(24, 4, 355, 24, 9, 360)
                                ),
                                name: Anchor::new(
                                    "f".to_owned(),
                                    Span::from_components(24, 10, 361, 24, 11, 362)
                                ),
                                expression: Anchor::new(
                                    Expression::Binary(Binary {
                                        operator: BinaryOperator::Add,
                                        left: Box::new(Anchor::new(
                                            Expression::Identifier("i".to_owned()),
                                            Span::from_components(24, 14, 365, 24, 15, 366)
                                        )),
                                        right: Box::new(Anchor::new(
                                            Expression::Float(Float::Decimal(1.0)),
                                            Span::from_components(24, 18, 369, 24, 21, 372)
                                        )),
                                    }),
                                    Span::from_components(24, 14, 365, 24, 21, 372)
                                )
                            }),
                            Span::from_components(24, 4, 355, 24, 21, 372)
                        ),
                        Anchor::new(
                            WorkflowElement::Declaration(BoundDeclaration {
                                type_: Anchor::new(
                                    Type::Array {
                                        item: Box::new(Anchor::new(
                                            Type::File,
                                            Span::from_components(25, 10, 383, 25, 14, 387)
                                        )),
                                        non_empty: false
                                    },
                                    Span::from_components(25, 4, 377, 25, 15, 388)
                                ),
                                name: Anchor::new(
                                    "file_array".to_owned(),
                                    Span::from_components(25, 16, 389, 25, 26, 399)
                                ),
                                expression: Anchor::new(
                                    Expression::Ternary(Ternary {
                                        condition: Box::new(Anchor::new(
                                            Expression::Apply(Apply {
                                                name: Anchor::new(
                                                    "defined".to_owned(),
                                                    Span::from_components(25, 32, 405, 25, 39, 412)
                                                ),
                                                arguments: vec![Anchor::new(
                                                    Expression::Identifier("ex".to_owned()),
                                                    Span::from_components(25, 40, 413, 25, 42, 415)
                                                )]
                                            }),
                                            Span::from_components(25, 32, 405, 25, 43, 416)
                                        )),
                                        true_branch: Box::new(Anchor::new(
                                            Expression::Apply(Apply {
                                                name: Anchor::new(
                                                    "select_all".to_owned(),
                                                    Span::from_components(25, 49, 422, 25, 59, 432)
                                                ),
                                                arguments: vec![Anchor::new(
                                                    Expression::Access(Access {
                                                        collection: Box::new(Anchor::new(
                                                            Expression::Apply(Apply {
                                                                name: Anchor::new(
                                                                    "select_first".to_owned(),
                                                                    Span::from_components(
                                                                        25, 60, 433, 25, 72, 445
                                                                    )
                                                                ),
                                                                arguments: vec![Anchor::new(
                                                                    Expression::Array(
                                                                        ArrayLiteral {
                                                                            elements: vec![Anchor::new(
                                                                                Expression::Identifier("ex".to_owned()),
                                                                                Span::from_components(
                                                                                    25, 74, 447, 25, 76, 449
                                                                                )
                                                                            )]
                                                                        }
                                                                    ),
                                                                    Span::from_components(
                                                                        25, 73, 446, 25, 77, 450
                                                                    )
                                                                )]
                                                            }),
                                                            Span::from_components(
                                                                25, 60, 433, 25, 78, 451
                                                            )
                                                        )),
                                                        accesses: vec![
                                                            Anchor::new(
                                                                AccessOperation::Field("a".to_owned()),
                                                                Span::from_components(
                                                                    25, 79, 452, 25, 80, 453
                                                                )
                                                            )
                                                        ]
                                                    }),
                                                    Span::from_components(25, 60, 433, 25, 80, 453)
                                                )]
                                            }),
                                            Span::from_components(25, 49, 422, 25, 81, 454)
                                        )),
                                        false_branch: Box::new(Anchor::new(
                                            Expression::Array(ArrayLiteral {
                                                elements: Vec::new()
                                            }),
                                            Span::from_components(25, 87, 460, 25, 89, 462)
                                        ))
                                    }),
                                    Span::from_components(25, 29, 402, 25, 89, 462)
                                )
                            }),
                            Span::from_components(25, 4, 377, 25, 89, 462)
                        ),
                        Anchor::new(
                            WorkflowElement::Call(Call {
                                target: Anchor::new(
                                    QualifiedIdentifier {
                                        parts: vec![
                                            Anchor::new(
                                                "local".to_owned(),
                                                Span::from_components(27, 9, 477, 27, 14, 482)
                                            ),
                                            Anchor::new(
                                                "foo".to_owned(),
                                                Span::from_components(27, 15, 483, 27, 18, 486)
                                            )
                                        ]
                                    },
                                    Span::from_components(27, 9, 477, 27, 18, 486)
                                ),
                                alias: None,
                                inputs: None,
                            }),
                            Span::from_components(27, 4, 472, 27, 18, 486)
                        ),
                        Anchor::new(
                            WorkflowElement::Call(Call {
                                target: Anchor::new(
                                    QualifiedIdentifier {
                                        parts: vec![
                                            Anchor::new(
                                                "local".to_owned(),
                                                Span::from_components(28, 9, 496, 28, 14, 501)
                                            ),
                                            Anchor::new(
                                                "foo".to_owned(),
                                                Span::from_components(28, 15, 502, 28, 18, 505)
                                            )
                                        ]
                                    },
                                    Span::from_components(28, 9, 496, 28, 18, 505)
                                ),
                                alias: Some(Anchor::new(
                                    "bar".to_owned(),
                                    Span::from_components(28, 22, 509, 28, 25, 512)
                                )),
                                inputs: Some(Vec::new())
                            }),
                            Span::from_components(28, 4, 491, 28, 28, 515)
                        ),
                        Anchor::new(
                            WorkflowElement::Call(Call {
                                target: Anchor::new(
                                    QualifiedIdentifier {
                                        parts: vec![
                                            Anchor::new(
                                                "local".to_owned(),
                                                Span::from_components(29, 9, 525, 29, 14, 530)
                                            ),
                                            Anchor::new(
                                                "baz".to_owned(),
                                                Span::from_components(29, 15, 531, 29, 18, 534)
                                            )
                                        ]
                                    },
                                    Span::from_components(29, 9, 525, 29, 18, 534)
                                ),
                                alias: None,
                                inputs: Some(Vec::new())
                            }),
                            Span::from_components(29, 4, 520, 31, 5, 557)
                        ),
                        Anchor::new(
                            WorkflowElement::Call(Call {
                                target: Anchor::new(
                                    QualifiedIdentifier {
                                        parts: vec![
                                            Anchor::new(
                                                "remote".to_owned(),
                                                Span::from_components(32, 9, 567, 32, 15, 573)
                                            ),
                                            Anchor::new(
                                                "waldo".to_owned(),
                                                Span::from_components(32, 16, 574, 32, 21, 579)
                                            )
                                        ]
                                    },
                                    Span::from_components(32, 9, 567, 32, 21, 579)
                                ),
                                alias: None,
                                inputs: Some(vec![
                                    Anchor::new(
                                        CallInput {
                                            name: Anchor::new(
                                                "x".to_owned(),
                                                Span::from_components(34, 12, 609, 34, 13, 610)
                                            ),
                                            expression: Some(Anchor::new(
                                                Expression::Int(Integer::Decimal(1)),
                                                Span::from_components(34, 16, 613, 34, 17, 614)
                                            ))
                                        },
                                        Span::from_components(34, 12, 609, 34, 17, 614)
                                    ),
                                    Anchor::new(
                                        CallInput {
                                            name: Anchor::new(
                                                "y".to_owned(),
                                                Span::from_components(35, 12, 628, 35, 13, 629)
                                            ),
                                            expression: Some(Anchor::new(
                                                Expression::Boolean(false),
                                                Span::from_components(35, 16, 632, 35, 21, 637)
                                            ))
                                        },
                                        Span::from_components(35, 12, 628, 35, 21, 637)
                                    )
                                ])
                            }),
                            Span::from_components(32, 4, 562, 36, 5, 643)
                        ),
                        Anchor::new(
                            WorkflowElement::Conditional(Conditional {
                                expression: Anchor::new(
                                    Expression::Binary(Binary {
                                        operator: BinaryOperator::Gt,
                                        left: Box::new(Anchor::new(
                                            Expression::Int(Integer::Decimal(1)),
                                            Span::from_components(38, 8, 653, 38, 9, 654)
                                        )),
                                        right: Box::new(Anchor::new(
                                            Expression::Int(Integer::Decimal(2)),
                                            Span::from_components(38, 12, 657, 38, 13, 658)
                                        )),
                                    }),
                                    Span::from_components(38, 8, 653, 38, 13, 658)
                                ),
                                body: vec![Anchor::new(
                                    WorkflowNestedElement::Scatter(Scatter {
                                        name: Anchor::new(
                                            "file".to_owned(),
                                            Span::from_components(39, 17, 679, 39, 21, 683),
                                        ),
                                        expression: Anchor::new(
                                            Expression::Identifier("file_array".to_owned()),
                                            Span::from_components(39, 25, 687, 39, 35, 697)
                                        ),
                                        body: vec![Anchor::new(
                                            WorkflowNestedElement::Call(Call {
                                                target: Anchor::new(
                                                    QualifiedIdentifier {
                                                        parts: vec![Anchor::new(
                                                            "task1".to_owned(),
                                                            Span::from_components(
                                                                40, 17, 718, 40, 22, 723
                                                            )
                                                        ),]
                                                    },
                                                    Span::from_components(40, 17, 718, 40, 22, 723)
                                                ),
                                                alias: None,
                                                inputs: Some(vec![
                                                    Anchor::new(
                                                        CallInput {
                                                            name: Anchor::new(
                                                                "file".to_owned(),
                                                                Span::from_components(
                                                                    42, 18, 767, 42, 22, 771
                                                                )
                                                            ),
                                                            expression: None,
                                                        },
                                                        Span::from_components(
                                                            42, 18, 767, 42, 22, 771
                                                        )
                                                    ),
                                                    Anchor::new(
                                                        CallInput {
                                                            name: Anchor::new(
                                                                "ex".to_owned(),
                                                                Span::from_components(
                                                                    43, 18, 791, 43, 20, 793
                                                                )
                                                            ),
                                                            expression: None,
                                                        },
                                                        Span::from_components(
                                                            43, 18, 791, 43, 20, 793
                                                        )
                                                    ),
                                                    Anchor::new(
                                                        CallInput {
                                                            name: Anchor::new(
                                                                "docker_image".to_owned(),
                                                                Span::from_components(
                                                                    44, 18, 813, 44, 30, 825
                                                                )
                                                            ),
                                                            expression: Some(Anchor::new(
                                                                Expression::String(StringLiteral {
                                                                    parts: vec![Anchor::new(
                                                                        StringPart::Content(
                                                                            "ubuntu".to_owned()
                                                                        ),
                                                                        Span::from_components(
                                                                            44, 34, 829, 44, 40, 835
                                                                        )
                                                                    )]
                                                                }),
                                                                Span::from_components(
                                                                    44, 33, 828, 44, 41, 836
                                                                )
                                                            ))
                                                        },
                                                        Span::from_components(
                                                            44, 18, 813, 44, 41, 836
                                                        )
                                                    )
                                                ])
                                            }),
                                            Span::from_components(40, 12, 713, 45, 13, 850)
                                        ),]
                                    }),
                                    Span::from_components(39, 8, 670, 46, 9, 860)
                                )]
                            }),
                            Span::from_components(38, 4, 649, 47, 5, 866)
                        ),
                        Anchor::new(
                            WorkflowElement::Output(Output {
                                declarations: vec![Anchor::new(
                                    BoundDeclaration {
                                        type_: Anchor::new(
                                            Type::Optional(Box::new(Anchor::new(
                                                Type::Array {
                                                    item: Box::new(Anchor::new(
                                                        Type::File,
                                                        Span::from_components(
                                                            50, 14, 895, 50, 18, 899
                                                        )
                                                    )),
                                                    non_empty: false
                                                },
                                                Span::from_components(50, 8, 889, 50, 19, 900)
                                            ))),
                                            Span::from_components(50, 8, 889, 50, 20, 901)
                                        ),
                                        name: Anchor::new(
                                            "f".to_owned(),
                                            Span::from_components(50, 21, 902, 50, 22, 903)
                                        ),
                                        expression: Anchor::new(
                                            Expression::Access(Access {
                                                collection: Box::new(Anchor::new(
                                                    Expression::Identifier("task1".to_owned()),
                                                    Span::from_components(50, 25, 906, 50, 30, 911)
                                                )),
                                                accesses: vec![Anchor::new(
                                                    AccessOperation::Field("name_file".to_owned()),
                                                    Span::from_components(50, 31, 912, 50, 40, 921)
                                                )],
                                            }),
                                            Span::from_components(50, 25, 906, 50, 40, 921)
                                        )
                                    },
                                    Span::from_components(50, 8, 889, 50, 40, 921)
                                )]
                            }),
                            Span::from_components(49, 4, 872, 51, 5, 927)
                        ),
                        Anchor::new(
                            WorkflowElement::Meta(Meta {
                                attributes: vec![
                                    Anchor::new(
                                        MetaAttribute {
                                            name: Anchor::new(
                                                "description".to_owned(),
                                                Span::from_components(54, 8, 948, 54, 19, 959)
                                            ),
                                            value: Anchor::new(
                                                MetaValue::String(MetaString {
                                                    parts: vec![Anchor::new(
                                                        MetaStringPart::Content(
                                                            "Test workflow".to_owned()
                                                        ),
                                                        Span::from_components(
                                                            54, 22, 962, 54, 35, 975
                                                        ),
                                                    )]
                                                }),
                                                Span::from_components(54, 21, 961, 54, 36, 976)
                                            ),
                                        },
                                        Span::from_components(54, 8, 948, 54, 36, 976)
                                    ),
                                    Anchor::new(
                                        MetaAttribute {
                                            name: Anchor::new(
                                                "test".to_owned(),
                                                Span::from_components(55, 8, 985, 55, 12, 989)
                                            ),
                                            value: Anchor::new(
                                                MetaValue::Boolean(true),
                                                Span::from_components(55, 14, 991, 55, 18, 995)
                                            ),
                                        },
                                        Span::from_components(55, 8, 985, 55, 18, 995)
                                    ),
                                    Anchor::new(
                                        MetaAttribute {
                                            name: Anchor::new(
                                                "size".to_owned(),
                                                Span::from_components(56, 8, 1004, 56, 12, 1008)
                                            ),
                                            value: Anchor::new(
                                                MetaValue::Int(Integer::Decimal(10)),
                                                Span::from_components(56, 14, 1010, 56, 16, 1012)
                                            ),
                                        },
                                        Span::from_components(56, 8, 1004, 56, 16, 1012)
                                    ),
                                    Anchor::new(
                                        MetaAttribute {
                                            name: Anchor::new(
                                                "numbers".to_owned(),
                                                Span::from_components(57, 8, 1021, 57, 15, 1028)
                                            ),
                                            value: Anchor::new(
                                                MetaValue::Array(MetaArray {
                                                    elements: vec![
                                                        Anchor::new(
                                                            MetaValue::Int(Integer::Decimal(1)),
                                                            Span::from_components(
                                                                57, 18, 1031, 57, 19, 1032
                                                            )
                                                        ),
                                                        Anchor::new(
                                                            MetaValue::Int(Integer::Decimal(2)),
                                                            Span::from_components(
                                                                57, 21, 1034, 57, 22, 1035
                                                            )
                                                        ),
                                                        Anchor::new(
                                                            MetaValue::Int(Integer::Decimal(3)),
                                                            Span::from_components(
                                                                57, 24, 1037, 57, 25, 1038
                                                            )
                                                        ),
                                                    ]
                                                }),
                                                Span::from_components(57, 17, 1030, 57, 26, 1039)
                                            ),
                                        },
                                        Span::from_components(57, 8, 1021, 57, 26, 1039)
                                    ),
                                    Anchor::new(
                                        MetaAttribute {
                                            name: Anchor::new(
                                                "keywords".to_owned(),
                                                Span::from_components(58, 8, 1048, 58, 16, 1056)
                                            ),
                                            value: Anchor::new(
                                                MetaValue::Object(MetaObject {
                                                    fields: vec![
                                                        Anchor::new(
                                                            MetaObjectField {
                                                                name: Anchor::new(
                                                                    "a".to_owned(),
                                                                    Span::from_components(
                                                                        59, 12, 1072, 59, 13, 1073
                                                                    )
                                                                ),
                                                                value: Anchor::new(
                                                                    MetaValue::Float(
                                                                        Float::Decimal(1.0)
                                                                    ),
                                                                    Span::from_components(
                                                                        59, 15, 1075, 59, 18, 1078
                                                                    )
                                                                ),
                                                            },
                                                            Span::from_components(
                                                                59, 12, 1072, 59, 18, 1078
                                                            )
                                                        ),
                                                        Anchor::new(
                                                            MetaObjectField {
                                                                name: Anchor::new(
                                                                    "b".to_owned(),
                                                                    Span::from_components(
                                                                        60, 12, 1092, 60, 13, 1093
                                                                    )
                                                                ),
                                                                value: Anchor::new(
                                                                    MetaValue::Int(
                                                                        Integer::Decimal(-1)
                                                                    ),
                                                                    Span::from_components(
                                                                        60, 15, 1095, 60, 17, 1097
                                                                    )
                                                                ),
                                                            },
                                                            Span::from_components(
                                                                60, 12, 1092, 60, 17, 1097
                                                            )
                                                        )
                                                    ]
                                                }),
                                                Span::from_components(58, 18, 1058, 61, 9, 1107)
                                            ),
                                        },
                                        Span::from_components(58, 8, 1048, 61, 9, 1107)
                                    ),
                                    Anchor::new(
                                        MetaAttribute {
                                            name: Anchor::new(
                                                "x".to_owned(),
                                                Span::from_components(62, 8, 1116, 62, 9, 1117)
                                            ),
                                            value: Anchor::new(
                                                MetaValue::Null,
                                                Span::from_components(62, 11, 1119, 62, 15, 1123)
                                            ),
                                        },
                                        Span::from_components(62, 8, 1116, 62, 15, 1123)
                                    )
                                ]
                            }),
                            Span::from_components(53, 4, 933, 63, 5, 1129)
                        ),
                    ]
                }),
                Span::from_components(17, 0, 255, 64, 1, 1131)
            )
        );
        assert_eq!(
            body.next().unwrap(),
            Anchor::new(
                DocumentElement::Task(Task {
                    name: Anchor::new(
                        "Task1".to_owned(),
                        Span::from_components(66, 5, 1138, 66, 10, 1143)
                    ),
                    body: vec![
                        Anchor::new(
                            TaskElement::Input(Input {
                                declarations: vec![
                                    Anchor::new(
                                        InputDeclaration::Unbound(UnboundDeclaration {
                                            type_: Anchor::new(
                                                Type::File,
                                                Span::from_components(68, 8, 1166, 68, 12, 1170)
                                            ),
                                            name: Anchor::new(
                                                "file".to_owned(),
                                                Span::from_components(68, 13, 1171, 68, 17, 1175)
                                            )
                                        }),
                                        Span::from_components(68, 8, 1166, 68, 17, 1175)
                                    ),
                                    Anchor::new(
                                        InputDeclaration::Unbound(UnboundDeclaration {
                                            type_: Anchor::new(
                                                Type::Optional(Box::new(Anchor::new(
                                                    Type::User("Example2".to_owned()),
                                                    Span::from_components(
                                                        69, 8, 1184, 69, 16, 1192
                                                    )
                                                ))),
                                                Span::from_components(69, 8, 1184, 69, 17, 1193)
                                            ),
                                            name: Anchor::new(
                                                "ex".to_owned(),
                                                Span::from_components(69, 18, 1194, 69, 20, 1196)
                                            ),
                                        }),
                                        Span::from_components(69, 8, 1184, 69, 20, 1196)
                                    ),
                                    Anchor::new(
                                        InputDeclaration::Unbound(UnboundDeclaration {
                                            type_: Anchor::new(
                                                Type::String,
                                                Span::from_components(70, 8, 1205, 70, 14, 1211)
                                            ),
                                            name: Anchor::new(
                                                "docker_image".to_owned(),
                                                Span::from_components(70, 15, 1212, 70, 27, 1224)
                                            )
                                        }),
                                        Span::from_components(70, 8, 1205, 70, 27, 1224)
                                    ),
                                ]
                            }),
                            Span::from_components(67, 4, 1150, 71, 5, 1230)
                        ),
                        Anchor::new(
                            TaskElement::Command(Command {
                                parts: vec![
                                    Anchor::new(
                                        StringPart::Content("\n    echo ".to_owned()),
                                        Span::from_components(73, 15, 1247, 74, 9, 1257)
                                    ),
                                    Anchor::new(
                                        StringPart::Placeholder(Expression::Identifier(
                                            "file".to_owned()
                                        )),
                                        Span::from_components(74, 9, 1257, 74, 16, 1264)
                                    ),
                                    Anchor::new(
                                        StringPart::Content(" \\\n      | cat\n    ".to_owned()),
                                        Span::from_components(74, 16, 1264, 76, 4, 1283)
                                    )
                                ]
                            }),
                            Span::from_components(73, 4, 1236, 76, 7, 1286)
                        ),
                        Anchor::new(
                            TaskElement::Output(Output {
                                declarations: vec![Anchor::new(
                                    BoundDeclaration {
                                        type_: Anchor::new(
                                            Type::File,
                                            Span::from_components(79, 8, 1309, 79, 12, 1313)
                                        ),
                                        name: Anchor::new(
                                            "name_file".to_owned(),
                                            Span::from_components(79, 13, 1314, 79, 22, 1323)
                                        ),
                                        expression: Anchor::new(
                                            Expression::Apply(Apply {
                                                name: Anchor::new(
                                                    "stdout".to_owned(),
                                                    Span::from_components(
                                                        79, 25, 1326, 79, 31, 1332
                                                    )
                                                ),
                                                arguments: vec![]
                                            }),
                                            Span::from_components(79, 25, 1326, 79, 33, 1334)
                                        )
                                    },
                                    Span::from_components(79, 8, 1309, 79, 33, 1334)
                                )]
                            }),
                            Span::from_components(78, 4, 1292, 80, 5, 1340)
                        ),
                        Anchor::new(
                            TaskElement::Runtime(Runtime {
                                attributes: vec![Anchor::new(
                                    RuntimeAttribute {
                                        name: Anchor::new(
                                            "container".to_owned(),
                                            Span::from_components(83, 8, 1368, 83, 17, 1377)
                                        ),
                                        expression: Anchor::new(
                                            Expression::Identifier("docker_image".to_owned()),
                                            Span::from_components(83, 19, 1379, 83, 31, 1391)
                                        )
                                    },
                                    Span::from_components(83, 8, 1368, 83, 31, 1391)
                                )]
                            }),
                            Span::from_components(82, 4, 1350, 84, 5, 1397)
                        ),
                        Anchor::new(
                            TaskElement::Meta(Meta {
                                attributes: vec![Anchor::new(
                                    MetaAttribute {
                                        name: Anchor::new(
                                            "description".to_owned(),
                                            Span::from_components(87, 8, 1418, 87, 19, 1429)
                                        ),
                                        value: Anchor::new(
                                            MetaValue::String(MetaString {
                                                parts: vec![Anchor::new(
                                                    MetaStringPart::Content(
                                                        "write name to file".to_owned()
                                                    ),
                                                    Span::from_components(
                                                        87, 22, 1432, 87, 40, 1450
                                                    ),
                                                )]
                                            }),
                                            Span::from_components(87, 21, 1431, 87, 41, 1451)
                                        ),
                                    },
                                    Span::from_components(87, 8, 1418, 87, 41, 1451)
                                )]
                            }),
                            Span::from_components(86, 4, 1403, 88, 5, 1457)
                        )
                    ]
                }),
                Span::from_components(66, 0, 1133, 89, 1, 1459)
            )
        );
    }
}
