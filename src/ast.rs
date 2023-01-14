use anyhow::{bail, ensure, Error, Result};
use regex::Regex;
use std::{
    collections::HashSet, convert::TryInto, fmt::Display, mem::discriminant, ops::Deref,
    path::PathBuf, str::FromStr,
};

// operators
pub const POS: &str = "+";
pub const NEG: &str = "-";
pub const ADD: &str = "+";
pub const SUB: &str = "-";
pub const MUL: &str = "*";
pub const DIV: &str = "/";
pub const MOD: &str = "%";
pub const GT: &str = ">";
pub const LT: &str = "<";
pub const GTE: &str = ">=";
pub const LTE: &str = "<=";
pub const EQ: &str = "==";
pub const NEQ: &str = "!=";
pub const AND: &str = "&&";
pub const OR: &str = "||";
pub const NOT: &str = "!";

#[derive(Debug)]
pub struct Location {
    /// Line of the source file
    pub line: usize,
    /// Column of the source line
    pub column: usize,
    /// Absolute byte offset
    pub offset: usize,
}

/// Wrapper around a syntax element of type `T` that also encapsulates the source tree-sitter node.
/// This is useful for retaining source code position information in the AST. `Deref`s to `T`.
#[derive(Debug)]
pub struct Node<T> {
    /// The AST element.
    pub element: T,
    /// The starting position in the source document from which this node was derived.
    pub start: Location,
    /// The ending position in the source document from which this node was derived.
    pub end: Location,
}

impl<'a, T> Deref for Node<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.element
    }
}

#[derive(Debug)]
pub enum Integer {
    Decimal(i64),
    Octal(String),
    Hex(String),
}

impl Integer {
    pub fn negate(&mut self) -> Result<()> {
        match self {
            Self::Decimal(i) => *i *= -1,
            _ => bail!("Can only negate a decimal integer"),
        }
        Ok(())
    }
}

impl FromStr for Integer {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        if s.len() >= 2 {
            match &s[..2] {
                "0o" | "0O" => return Ok(Self::Octal(s.to_owned())),
                "0x" | "0X" => return Ok(Self::Hex(s.to_owned())),
                _ => (),
            }
        }
        Ok(Self::Decimal(s.parse::<i64>()?))
    }
}

impl TryInto<i64> for Integer {
    type Error = Error;

    fn try_into(self) -> Result<i64> {
        let i = match self {
            Integer::Decimal(i) => i,
            Integer::Octal(o) => i64::from_str_radix(&o, 8)?,
            Integer::Hex(h) => i64::from_str_radix(&h, 16)?,
        };
        Ok(i)
    }
}

#[derive(Debug)]
pub enum Float {
    Decimal(f64),
    Scientific(String),
}

impl Float {
    pub fn negate(&mut self) -> Result<()> {
        match self {
            Self::Decimal(f) => *f *= -1.0,
            Self::Scientific(s) if s.starts_with("-") => {
                s.remove(0);
            }
            Self::Scientific(s) => s.insert(0, '-'),
        }
        Ok(())
    }
}

impl FromStr for Float {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let f = if s.contains('e') || s.contains('E') {
            Self::Scientific(s.to_owned())
        } else {
            Self::Decimal(s.parse::<f64>()?)
        };
        Ok(f)
    }
}

impl TryInto<f64> for Float {
    type Error = Error;

    fn try_into(self) -> Result<f64> {
        match self {
            Float::Decimal(f) => Ok(f),
            Float::Scientific(s) => Ok(s.parse()?),
        }
    }
}

#[derive(Debug)]
pub enum StringPart {
    Literal(String),
    Escape(String),
    Placeholder(Expression),
}

#[derive(Debug)]
pub struct StringLiteral {
    pub parts: Vec<Node<StringPart>>,
}

#[derive(Debug)]
pub struct ArrayLiteral {
    pub elements: Vec<Node<Expression>>,
}

#[derive(Debug)]
pub struct MapEntry {
    pub key: Node<Expression>,
    pub value: Node<Expression>,
}

#[derive(Debug)]
pub struct MapLiteral {
    pub entries: Vec<Node<MapEntry>>,
}

#[derive(Debug)]
pub struct PairLiteral {
    pub left: InnerExpression,
    pub right: InnerExpression,
}

#[derive(Debug)]
pub struct ObjectField {
    pub name: Node<String>,
    pub expression: Node<Expression>,
}

#[derive(Debug)]
pub struct ObjectLiteral {
    pub type_name: Node<String>,
    pub fields: Vec<Node<ObjectField>>,
}

#[derive(Debug)]
pub enum UnaryOperator {
    Pos,
    Neg,
    Not,
}

impl FromStr for UnaryOperator {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let oper = match s {
            POS => Self::Pos,
            NEG => Self::Neg,
            NOT => Self::Not,
            _ => bail!("Invalid unary operator {:?}", s),
        };
        Ok(oper)
    }
}

impl Display for UnaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
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
            _ => bail!("Invalid binary operator {}", s),
        };
        Ok(oper)
    }
}

impl Display for BinaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    pub operands: Vec<Node<Expression>>,
}

#[derive(Debug)]
pub struct Apply {
    pub name: Node<String>,
    pub arguments: Vec<Node<Expression>>,
}

#[derive(Debug)]
pub enum AccessOperation {
    Index(Expression),
    Field(String),
}

#[derive(Debug)]
pub struct Access {
    pub collection: InnerExpression,
    pub accesses: Vec<Node<AccessOperation>>,
}

#[derive(Debug)]
pub struct Ternary {
    pub condition: InnerExpression,
    pub true_branch: InnerExpression,
    pub false_branch: InnerExpression,
}

pub type InnerExpression = Box<Node<Expression>>;

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

pub type InnerType = Box<Node<Type>>;

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
    pub wdl_type: Node<Type>,
    pub name: Node<String>,
}

#[derive(Debug)]
pub struct BoundDeclaration {
    pub wdl_type: Node<Type>,
    pub name: Node<String>,
    pub expression: Node<Expression>,
}

/// Enumeration of sources from which a WDL document may be loaded.
#[derive(Debug)]
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

#[derive(Clone, Debug, PartialEq)]
pub enum VersionIdentifier {
    V1_0,
    V1_1,
}

impl FromStr for VersionIdentifier {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "1.0" => Ok(VersionIdentifier::V1_0),
            "1.1" => Ok(VersionIdentifier::V1_1),
            _ => bail!("Invalid version identifier {}", s),
        }
    }
}

#[derive(Debug)]
pub struct Version {
    pub identifier: Node<VersionIdentifier>,
}

#[derive(Debug)]
pub enum Namespace {
    Explicit(Node<String>),
    Implicit(String),
}

impl Namespace {
    pub fn try_from_uri<S: AsRef<str>>(uri: S) -> Result<Self> {
        let s = uri.as_ref();
        let ns = match Regex::new(r".*/(.+)\.wdl").unwrap().captures(s) {
            Some(cap) => cap[1].to_owned(),
            None if s.ends_with(".wdl") => s[0..s.len() - 4].to_owned(),
            None => bail!(""),
        };
        Ok(Self::Implicit(ns))
    }
}

#[derive(Debug)]
pub struct Alias {
    pub from: Node<String>,
    pub to: Node<String>,
}

#[derive(Debug)]
pub struct Import {
    pub uri: Node<String>,
    pub namespace: Namespace,
    pub aliases: Vec<Node<Alias>>,
}

#[derive(Debug)]
pub struct Struct {
    pub name: Node<String>,
    pub fields: Vec<Node<UnboundDeclaration>>,
}

#[derive(Debug)]
pub enum InputDeclaration {
    Bound(BoundDeclaration),
    Unbound(UnboundDeclaration),
}

#[derive(Debug)]
pub struct Input {
    pub declarations: Vec<Node<InputDeclaration>>,
}

#[derive(Debug)]
pub struct Output {
    pub declarations: Vec<Node<BoundDeclaration>>,
}

#[derive(Debug)]
pub struct Command {
    pub parts: Vec<Node<StringPart>>,
}

#[derive(Debug)]
pub struct RuntimeAttribute {
    pub name: Node<String>,
    pub expression: Node<Expression>,
}

#[derive(Debug)]
pub struct Runtime {
    pub attributes: Vec<Node<RuntimeAttribute>>,
}

#[derive(Debug)]
pub enum MetaStringPart {
    Content(String),
    Escape(String),
}

#[derive(Debug)]
pub struct MetaString {
    pub parts: Vec<Node<MetaStringPart>>,
}

#[derive(Debug)]
pub struct MetaArray {
    pub elements: Vec<Node<MetaValue>>,
}

#[derive(Debug)]
pub struct MetaObjectField {
    pub name: Node<String>,
    pub value: Node<MetaValue>,
}
#[derive(Debug)]
pub struct MetaObject {
    pub fields: Vec<Node<MetaObjectField>>,
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
    pub name: Node<String>,
    pub value: Node<MetaValue>,
}

#[derive(Debug)]
pub struct Meta {
    pub attributes: Vec<Node<MetaAttribute>>,
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

#[derive(Debug)]
pub struct Task {
    pub name: Node<String>,
    pub body: Vec<Node<TaskElement>>,
}

impl Task {
    pub fn validate(&self) -> Result<()> {
        let mut seen = HashSet::with_capacity(6);
        let mut seen_command = false;
        for element in self.body.iter().map(|e| e.deref()) {
            let must_be_unique = match *element {
                TaskElement::Input(_)
                | TaskElement::Output(_)
                | TaskElement::Runtime(_)
                | TaskElement::Meta(_)
                | TaskElement::ParameterMeta(_) => true,
                TaskElement::Command(_) => {
                    seen_command = true;
                    true
                }
                _ => false,
            };
            if must_be_unique {
                let d = discriminant(element);
                ensure!(
                    !seen.contains(&d),
                    "Task contains more than one of the same element type {:?}",
                    element
                );
                seen.insert(d);
            }
        }
        ensure!(seen_command, "Task is missing required Command element");
        Ok(())
    }
}

#[derive(Debug)]
pub struct CallInput {
    pub name: Node<String>,
    pub expression: Option<Node<Expression>>,
}

#[derive(Debug)]
pub struct QualifiedName {
    pub parts: Vec<Node<String>>,
}

#[derive(Debug)]
pub struct Call {
    pub target: Node<QualifiedName>,
    pub alias: Option<Node<String>>,
    pub inputs: Vec<Node<CallInput>>,
}

#[derive(Debug)]
pub struct Scatter {
    pub name: Node<String>,
    pub expression: Node<Expression>,
    pub body: Vec<Node<WorkflowBodyElement>>,
}

#[derive(Debug)]
pub struct Conditional {
    pub expression: Node<Expression>,
    pub body: Vec<Node<WorkflowBodyElement>>,
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

#[derive(Debug)]
pub struct Workflow {
    pub name: Node<String>,
    pub body: Vec<Node<WorkflowElement>>,
}

impl Workflow {
    pub fn validate(&self) -> Result<()> {
        let mut seen = HashSet::with_capacity(4);
        for element in self.body.iter().map(|e| e.deref()) {
            match element.deref() {
                WorkflowElement::Input(_)
                | WorkflowElement::Output(_)
                | WorkflowElement::Meta(_)
                | WorkflowElement::ParameterMeta(_) => {
                    let d = discriminant(element);
                    ensure!(
                        !seen.contains(&d),
                        "Workflow contains more than one of the same element type {:?}",
                        element
                    );
                    seen.insert(d);
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
    pub version: Node<Version>,
    pub body: Vec<Node<DocumentElement>>,
}

impl Document {
    pub fn validate(&self) -> Result<()> {
        let mut element_count = 0;
        let mut workflow_count = 0;
        for element in self.element_iter() {
            match element {
                DocumentElement::Task(task) => {
                    task.validate()?;
                    element_count += 1;
                }
                DocumentElement::Workflow(workflow) => {
                    workflow.validate()?;
                    element_count += 1;
                    workflow_count += 1;
                }
                DocumentElement::Struct(_) => {
                    element_count += 1;
                }
                _ => (),
            }
        }
        ensure!(
            element_count > 0,
            "Document is missing at least one element of kind Struct, Task, or Workflow"
        );
        ensure!(
            workflow_count <= 1,
            "Document has more than one Workflow element"
        );
        Ok(())
    }

    pub fn element_iter(&self) -> impl Iterator<Item = &DocumentElement> {
        self.body.iter().map(|e| (*e).deref())
    }

    /// Returns this Document's Workflow if it contains one, or its Task if it contains exactly
    /// one, otherwise None.
    pub fn get_primary_element(&self) -> Option<&DocumentElement> {
        self.element_iter()
            .find(|e| match (*e).deref() {
                &DocumentElement::Workflow(_) => true,
                _ => false,
            })
            .or_else(|| {
                let tasks: Vec<_> = self
                    .element_iter()
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
