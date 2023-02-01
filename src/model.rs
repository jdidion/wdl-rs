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
/// coordinates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    /// Line of the source file
    pub line: usize,
    /// Column of the source line
    pub column: usize,
    /// Absolute byte offset
    pub offset: usize,
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
            Span { start, end }
        } else {
            panic!("Span start position must be less than end position")
        }
    }

    pub fn len(&self) -> usize {
        self.end.offset - self.start.offset - 1
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

#[derive(Debug)]
pub struct SourceFragment(pub String);

impl Display for SourceFragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

/// Wrapper around a model element of type `T` that also encapsulates source code span information.
/// `Deref`s to `T`.
#[derive(Debug)]
pub struct Anchor<T> {
    /// The model element.
    pub element: T,
    /// The span of the source document from which the element was derived.    
    pub span: Span,
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
#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
pub enum StringPart {
    Literal(String),
    Escape(String),
    Placeholder(Expression),
}

impl Display for StringPart {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            StringPart::Literal(s) => write!(f, "{}", s),
            StringPart::Escape(s) => write!(f, "{}", s),
            StringPart::Placeholder(_) => write!(f, "~{{..}}"),
        }
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct ArrayLiteral {
    pub elements: Vec<Anchor<Expression>>,
}

#[derive(Debug)]
pub struct MapEntry {
    pub key: Anchor<Expression>,
    pub value: Anchor<Expression>,
}

#[derive(Debug)]
pub struct MapLiteral {
    pub entries: Vec<Anchor<MapEntry>>,
}

#[derive(Debug)]
pub struct PairLiteral {
    pub left: InnerExpression,
    pub right: InnerExpression,
}

#[derive(Debug)]
pub struct ObjectField {
    pub name: Anchor<String>,
    pub expression: Anchor<Expression>,
}

#[derive(Debug)]
pub struct ObjectLiteral {
    pub type_name: Anchor<String>,
    pub fields: Vec<Anchor<ObjectField>>,
}

const POS: &str = "+";
const NEG: &str = "-";
const NOT: &str = "!";

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Unary {
    pub operator: UnaryOperator,
    pub expression: InnerExpression,
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

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Binary {
    pub operator: BinaryOperator,
    pub operands: Vec<Anchor<Expression>>,
}

#[derive(Debug)]
pub struct Apply {
    pub name: Anchor<String>,
    pub arguments: Vec<Anchor<Expression>>,
}

#[derive(Debug)]
pub enum AccessOperation {
    Index(Expression),
    Field(String),
}

#[derive(Debug)]
pub struct Access {
    pub collection: InnerExpression,
    pub accesses: Vec<Anchor<AccessOperation>>,
}

#[derive(Debug)]
pub struct Ternary {
    pub condition: InnerExpression,
    pub true_branch: InnerExpression,
    pub false_branch: InnerExpression,
}

pub type InnerExpression = Box<Anchor<Expression>>;

#[derive(Debug)]
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

pub type InnerType = Box<Anchor<Type>>;

#[derive(Debug)]
pub enum Type {
    Boolean,
    Int,
    Float,
    String,
    File,
    Array(InnerType),
    NonEmpty(InnerType),
    Map { key: InnerType, value: InnerType },
    Pair { left: InnerType, right: InnerType },
    Object,
    User(String),
    Optional(InnerType),
}

#[derive(Debug)]
pub struct UnboundDeclaration {
    pub wdl_type: Anchor<Type>,
    pub name: Anchor<String>,
}

#[derive(Debug)]
pub struct BoundDeclaration {
    pub wdl_type: Anchor<Type>,
    pub name: Anchor<String>,
    pub expression: Anchor<Expression>,
}

/// Enumeration of sources from which a WDL document may be loaded.
#[derive(Clone, Debug)]
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

#[derive(Debug)]
pub struct Version {
    pub identifier: Anchor<VersionIdentifier>,
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Alias {
    pub from: Anchor<String>,
    pub to: Anchor<String>,
}

#[derive(Debug)]
pub struct Import {
    pub uri: Anchor<String>,
    pub namespace: Namespace,
    pub aliases: Vec<Anchor<Alias>>,
}

#[derive(Debug)]
pub struct Struct {
    pub name: Anchor<String>,
    pub fields: Vec<Anchor<UnboundDeclaration>>,
}

#[derive(Debug)]
pub enum InputDeclaration {
    Bound(BoundDeclaration),
    Unbound(UnboundDeclaration),
}

#[derive(Debug)]
pub struct Input {
    pub declarations: Vec<Anchor<InputDeclaration>>,
}

#[derive(Debug)]
pub struct Output {
    pub declarations: Vec<Anchor<BoundDeclaration>>,
}

#[derive(Debug)]
pub enum MetaStringPart {
    Content(String),
    Escape(String),
}

#[derive(Debug)]
pub struct MetaString {
    pub parts: Vec<Anchor<MetaStringPart>>,
}

#[derive(Debug)]
pub struct MetaArray {
    pub elements: Vec<Anchor<MetaValue>>,
}

#[derive(Debug)]
pub struct MetaObjectField {
    pub name: Anchor<String>,
    pub value: Anchor<MetaValue>,
}

#[derive(Debug)]
pub struct MetaObject {
    pub fields: Vec<Anchor<MetaObjectField>>,
}

#[derive(Debug)]
pub enum MetaValue {
    Null,
    Boolean(bool),
    Int(Integer),
    Float(Float),
    String(MetaString),
    Array(MetaArray),
    Object(MetaObject),
}

#[derive(Debug)]
pub struct MetaAttribute {
    pub name: Anchor<String>,
    pub value: Anchor<MetaValue>,
}

#[derive(Debug)]
pub struct Meta {
    pub attributes: Vec<Anchor<MetaAttribute>>,
}

#[derive(Debug)]
pub struct Command {
    pub parts: Vec<Anchor<StringPart>>,
}

#[derive(Debug)]
pub struct RuntimeAttribute {
    pub name: Anchor<String>,
    pub expression: Anchor<Expression>,
}

#[derive(Debug)]
pub struct Runtime {
    pub attributes: Vec<Anchor<RuntimeAttribute>>,
}

#[derive(Debug)]
pub enum TaskElement {
    Input(Input),
    Output(Output),
    Declaration(BoundDeclaration),
    Command(Command),
    Runtime(Runtime),
    Meta(Meta),
    ParameterMeta(Meta),
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

#[derive(Debug)]
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

#[derive(Debug)]
pub struct QualifiedName {
    pub parts: Vec<Anchor<String>>,
}

#[derive(Debug)]
pub struct CallInput {
    pub name: Anchor<String>,
    pub expression: Option<Anchor<Expression>>,
}

#[derive(Debug)]
pub struct Call {
    pub target: Anchor<QualifiedName>,
    pub alias: Option<Anchor<String>>,
    pub inputs: Vec<Anchor<CallInput>>,
}

#[derive(Debug)]
pub struct Scatter {
    pub name: Anchor<String>,
    pub expression: Anchor<Expression>,
    pub body: Vec<Anchor<WorkflowBodyElement>>,
}

#[derive(Debug)]
pub struct Conditional {
    pub expression: Anchor<Expression>,
    pub body: Vec<Anchor<WorkflowBodyElement>>,
}

#[derive(Debug)]
pub enum WorkflowBodyElement {
    Call(Call),
    Scatter(Scatter),
    Conditional(Conditional),
}

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
pub enum DocumentElement {
    Import(Import),
    Struct(Struct),
    Task(Task),
    Workflow(Workflow),
}

#[derive(Debug)]
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
