use std::collections::HashSet;

// TODO: Support all features: https://tc39.github.io/ecma262/#prod-FormalParameter
#[derive(Clone, Debug, PartialEq)]
pub struct FormalParameter {
    pub name: String,
    pub init: Option<Node>,
    pub is_rest_param: bool,
}

pub type FormalParameters = Vec<FormalParameter>;

impl FormalParameter {
    pub fn new(name: String, init: Option<Node>, is_rest_param: bool) -> FormalParameter {
        FormalParameter {
            name: name,
            init: init,
            is_rest_param: is_rest_param,
        }
    }
}

// TODO: Support all features: https://tc39.github.io/ecma262/#prod-PropertyDefinition
#[derive(Clone, Debug, PartialEq)]
pub enum PropertyDefinition {
    IdentifierReference(String), // Not used in phases after fv_finder. This is replaced with Property(_, _) in fv_finder.
    Property(String, Node),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionDeclNode {
    pub name: String,
    pub mangled_name: Option<String>,
    pub fv: HashSet<String>,
    pub use_this: bool,
    pub params: FormalParameters,
    pub body: Box<Node>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeBase {
    StatementList(Vec<Node>),
    FunctionDecl(FunctionDeclNode),
    FunctionExpr(Option<String>, FormalParameters, Box<Node>), // Name, params, body
    VarDecl(String, Option<Box<Node>>),
    Member(Box<Node>, String),
    Index(Box<Node>, Box<Node>),
    New(Box<Node>),
    Call(Box<Node>, Vec<Node>),
    If(Box<Node>, Box<Node>, Box<Node>), // Cond, Then, Else
    While(Box<Node>, Box<Node>),         // Cond, Body
    For(Box<Node>, Box<Node>, Box<Node>, Box<Node>), // Init, Cond, Step, Body
    Assign(Box<Node>, Box<Node>),
    UnaryOp(Box<Node>, UnaryOp),
    BinaryOp(Box<Node>, Box<Node>, BinOp),
    TernaryOp(Box<Node>, Box<Node>, Box<Node>),
    Return(Option<Box<Node>>),
    Break,
    Continue,
    Array(Vec<Node>),
    Object(Vec<PropertyDefinition>),
    Identifier(String),
    This,
    Arguments,
    String(String),
    Boolean(bool),
    Number(f64),
    Nope,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub base: NodeBase,
    pub pos: usize,
}

impl Node {
    pub fn new(base: NodeBase, pos: usize) -> Node {
        Node {
            base: base,
            pos: pos,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UnaryOp {
    Delete,
    Void,
    Typeof,
    Plus,
    Minus,
    BitwiseNot,
    Not,
    PrInc, // Prefix
    PrDec,
    PoInc, // Postfix
    PoDec,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Exp,
    And,
    Or,
    Xor,
    LAnd,
    LOr,
    Eq,
    Ne,
    SEq, // Strict Eq
    SNe, // Strict Ne
    Lt,
    Gt,
    Le,
    Ge,
    Shl,
    Shr,
    ZFShr,
    Comma,
    Assign,
}
