use super::ident::Ident;
use super::path::{PathSegment, Root, StatePath};
use crate::sduix::stacx::common::{Path, Validate, ValidationIssue};
use std::fmt::{self, Write};

/// 2^63 as `f64`: integers with a module at or above it do not fit the `int.tryParse` of the
/// Stac runtime (a syntax error there).
const INT_LIMIT: f64 = 9_223_372_036_854_775_808.0;

/// Unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    /// `!x`
    Not,
    /// `-x`
    Neg,
}

/// Binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Rem,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `&&`
    And,
    /// `||`
    Or,
}

impl BinOp {
    fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::And => "&&",
            BinOp::Or => "||",
        }
    }

    /// Precedence level (see [`Expr`]): `||` 2, `&&` 3, `== !=` 4, comparison 5, `+ -` 6,
    /// `* / %` 7.
    fn level(self) -> u8 {
        match self {
            BinOp::Or => 2,
            BinOp::And => 3,
            BinOp::Eq | BinOp::Ne => 4,
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 5,
            BinOp::Add | BinOp::Sub => 6,
            BinOp::Mul | BinOp::Div | BinOp::Rem => 7,
        }
    }
}

const LEVEL_TERNARY: u8 = 1;
const LEVEL_UNARY: u8 = 8;
const LEVEL_PRIMARY: u8 = 9;

/// Typed AST of the expression language (`{"$expr": ".."}`), a closed set: an unknown
/// function or a wrong arity cannot be represented.
///
/// Precedence, from the loosest: `?:` (right associative), `||`, `&&`, `== !=`,
/// `< <= > >=`, `+ -`, `* / %` (all binary operators associate to the left), unary `! -`, then
/// postfix `.field`, `[expr]` and calls. [`Expr::to_source`] / [`Display`](fmt::Display) print
/// the expression with the minimum number of parentheses; [`Expr::Group`] always prints its
/// own (use [`Expr::grouped`] to force them).
///
/// ```
/// use derust::sduix::stacx::v1::state::{Expr, StatePath};
///
/// let (a, b, c) = (
///     Expr::from(StatePath::key("a")),
///     Expr::from(StatePath::key("b")),
///     Expr::from(StatePath::key("c")),
/// );
/// assert_eq!(((a.clone() + b.clone()) * c.clone()).to_source(), "(a + b) * c");
/// assert_eq!((a + b * c).to_source(), "a + b * c");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Number printed with a fractional part when it has one (`1.5`); integral values print as
    /// integers (`2`). Negative values print as a unary minus applied to the magnitude.
    Num(f64),
    /// Integer literal.
    Int(i64),
    /// Text, printed between single quotes with escapes.
    Text(String),
    /// `true` / `false`
    Bool(bool),
    /// `null`
    Null,
    /// A state path read.
    Ref(StatePath),
    /// `base.field`
    Field(Box<Expr>, Ident),
    /// `base[index]`
    Index(Box<Expr>, Box<Expr>),
    /// `!x` / `-x`
    Unary(UnOp, Box<Expr>),
    /// `left op right`
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// `condition ? when_true : when_false`
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    /// Explicit parentheses (the parser drops them; they are not part of the structure).
    Group(Box<Expr>),
    /// `length(x)` of a text or list.
    Length(Box<Expr>),
    /// `trim(x)` of a text.
    Trim(Box<Expr>),
    /// `contains(list, value)`
    Contains {
        /// The list searched.
        list: Box<Expr>,
        /// The value looked up.
        value: Box<Expr>,
    },
    /// `contains(list, object, 'field')`: some element has the same `field` as `object`.
    ContainsBy {
        /// The list searched.
        list: Box<Expr>,
        /// The object whose `field` is looked up.
        object: Box<Expr>,
        /// Name of the compared field (printed as a text literal).
        field: String,
    },
}

fn boxed(expr: impl Into<Expr>) -> Box<Expr> {
    Box::new(expr.into())
}

impl Expr {
    /// Number literal.
    pub fn num(value: f64) -> Self {
        Expr::Num(value)
    }

    /// Integer literal.
    pub fn int(value: i64) -> Self {
        Expr::Int(value)
    }

    /// Text literal.
    pub fn text(value: impl Into<String>) -> Self {
        Expr::Text(value.into())
    }

    /// Boolean literal.
    pub fn bool(value: bool) -> Self {
        Expr::Bool(value)
    }

    /// `null`
    pub fn null() -> Self {
        Expr::Null
    }

    /// Reads a state path.
    pub fn path(path: StatePath) -> Self {
        Expr::Ref(path)
    }

    /// `condition ? when_true : when_false`
    pub fn cond(
        condition: impl Into<Expr>,
        when_true: impl Into<Expr>,
        when_false: impl Into<Expr>,
    ) -> Self {
        Expr::Ternary(boxed(condition), boxed(when_true), boxed(when_false))
    }

    /// `length(value)`
    pub fn length(value: impl Into<Expr>) -> Self {
        Expr::Length(boxed(value))
    }

    /// `trim(value)`
    pub fn trim(value: impl Into<Expr>) -> Self {
        Expr::Trim(boxed(value))
    }

    /// `contains(list, value)`
    pub fn contains(list: impl Into<Expr>, value: impl Into<Expr>) -> Self {
        Expr::Contains {
            list: boxed(list),
            value: boxed(value),
        }
    }

    /// `contains(list, object, 'field')`
    pub fn contains_by(
        list: impl Into<Expr>,
        object: impl Into<Expr>,
        field: impl Into<String>,
    ) -> Self {
        Expr::ContainsBy {
            list: boxed(list),
            object: boxed(object),
            field: field.into(),
        }
    }

    /// `self.name`
    pub fn field(self, name: impl Into<Ident>) -> Self {
        Expr::Field(Box::new(self), name.into())
    }

    /// `self[index]`
    pub fn at(self, index: impl Into<Expr>) -> Self {
        Expr::Index(Box::new(self), boxed(index))
    }

    /// `(self)`, always with parentheses.
    pub fn grouped(self) -> Self {
        Expr::Group(Box::new(self))
    }

    fn binary(self, op: BinOp, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(op, Box::new(self), boxed(rhs))
    }

    /// `self == rhs`
    pub fn eq_(self, rhs: impl Into<Expr>) -> Self {
        self.binary(BinOp::Eq, rhs)
    }

    /// `self != rhs`
    pub fn ne_(self, rhs: impl Into<Expr>) -> Self {
        self.binary(BinOp::Ne, rhs)
    }

    /// `self < rhs`
    pub fn lt(self, rhs: impl Into<Expr>) -> Self {
        self.binary(BinOp::Lt, rhs)
    }

    /// `self <= rhs`
    pub fn le(self, rhs: impl Into<Expr>) -> Self {
        self.binary(BinOp::Le, rhs)
    }

    /// `self > rhs`
    pub fn gt(self, rhs: impl Into<Expr>) -> Self {
        self.binary(BinOp::Gt, rhs)
    }

    /// `self >= rhs`
    pub fn ge(self, rhs: impl Into<Expr>) -> Self {
        self.binary(BinOp::Ge, rhs)
    }

    /// `self && rhs`
    pub fn and(self, rhs: impl Into<Expr>) -> Self {
        self.binary(BinOp::And, rhs)
    }

    /// `self || rhs`
    pub fn or(self, rhs: impl Into<Expr>) -> Self {
        self.binary(BinOp::Or, rhs)
    }

    /// Whether this node prints as a negative number literal (a unary minus in the grammar).
    fn is_negative_literal(&self) -> bool {
        match self {
            Expr::Int(n) => *n < 0,
            Expr::Num(x) => *x < 0.0,
            _ => false,
        }
    }

    /// Precedence level of the printed form of this node.
    fn level(&self) -> u8 {
        match self {
            Expr::Ternary(..) => LEVEL_TERNARY,
            Expr::Binary(op, ..) => op.level(),
            Expr::Unary(..) => LEVEL_UNARY,
            _ if self.is_negative_literal() => LEVEL_UNARY,
            _ => LEVEL_PRIMARY,
        }
    }

    /// Whether the printed form starts with a unary operator.
    fn starts_with_unary(&self) -> bool {
        matches!(self, Expr::Unary(..)) || self.is_negative_literal()
    }

    /// The expression as the source accepted by the Stac runtime (the `$expr` value), with the
    /// minimum number of parentheses. Binary operators are printed with spaces (`a + b`).
    ///
    /// Printing never panics. Non representable values (`NaN`, infinities, integers with a
    /// module of 2^63 or more) are printed on a best effort basis and reported by
    /// [`Validate`].
    pub fn to_source(&self) -> String {
        let mut out = String::new();
        // Writing to a `String` cannot fail.
        let _ = self.write_source(&mut out);
        out
    }

    fn write_child<W: Write>(&self, out: &mut W, parenthesize: bool) -> fmt::Result {
        if parenthesize {
            out.write_char('(')?;
            self.write_source(out)?;
            out.write_char(')')
        } else {
            self.write_source(out)
        }
    }

    fn write_source<W: Write>(&self, out: &mut W) -> fmt::Result {
        match self {
            Expr::Num(x) => {
                if *x < 0.0 {
                    out.write_char('-')?;
                }
                write_number(out, *x)
            }
            Expr::Int(n) => {
                if *n < 0 {
                    out.write_char('-')?;
                }
                write!(out, "{}", n.unsigned_abs())
            }
            Expr::Text(text) => write_text(out, text),
            Expr::Bool(true) => out.write_str("true"),
            Expr::Bool(false) => out.write_str("false"),
            Expr::Null => out.write_str("null"),
            Expr::Ref(path) => write!(out, "{path}"),
            Expr::Field(base, name) => {
                write_postfix_base(out, base)?;
                write!(out, ".{name}")
            }
            Expr::Index(base, index) => {
                write_postfix_base(out, base)?;
                out.write_char('[')?;
                index.write_source(out)?;
                out.write_char(']')
            }
            Expr::Unary(op, operand) => {
                out.write_char(match op {
                    UnOp::Not => '!',
                    UnOp::Neg => '-',
                })?;
                if operand.starts_with_unary() {
                    out.write_char(' ')?;
                }
                operand.write_child(out, operand.level() < LEVEL_UNARY)
            }
            Expr::Binary(op, left, right) => {
                let level = op.level();
                left.write_child(out, left.level() < level)?;
                write!(out, " {} ", op.symbol())?;
                right.write_child(out, right.level() <= level)
            }
            Expr::Ternary(condition, when_true, when_false) => {
                condition.write_child(out, condition.level() <= LEVEL_TERNARY)?;
                out.write_str(" ? ")?;
                when_true.write_source(out)?;
                out.write_str(" : ")?;
                when_false.write_source(out)
            }
            Expr::Group(inner) => inner.write_child(out, true),
            Expr::Length(value) => write_call1(out, "length", value),
            Expr::Trim(value) => write_call1(out, "trim", value),
            Expr::Contains { list, value } => {
                out.write_str("contains(")?;
                list.write_source(out)?;
                out.write_str(", ")?;
                value.write_source(out)?;
                out.write_char(')')
            }
            Expr::ContainsBy {
                list,
                object,
                field,
            } => {
                out.write_str("contains(")?;
                list.write_source(out)?;
                out.write_str(", ")?;
                object.write_source(out)?;
                out.write_str(", ")?;
                write_text(out, field)?;
                out.write_char(')')
            }
        }
    }

    /// Canonical S-expression dump of the *parsed structure* of the expression (parentheses
    /// are not part of it, so [`Expr::Group`] is transparent). The Dart compatibility harness
    /// dumps the AST of the real parser the same way and compares both strings.
    ///
    /// Format (single spaces, no trailing space):
    ///
    /// * numbers as in the source without sign: `5`, `1.5` (integral values have no `.0`);
    ///   a negative number is `(neg 5)`;
    /// * text between double quotes, escaping `\` as `\\`, `"` as `\"`, newline as `\n`, tab as
    ///   `\t` and carriage return as `\r`: `"a\"b"`;
    /// * `true`, `false`, `null`;
    /// * a state path is expanded into its parse: `a`, `(. global token)`, `(idx items 0)`,
    ///   `(. (. user address) city)`; `params`/`result`/`response`/`event` are plain names;
    /// * `(. base name)` field, `(idx base index)` index;
    /// * `(neg x)` and `(not x)`;
    /// * `(op left right)` with `+ - * / % < <= > >= == != && ||`;
    /// * `(? condition when_true when_false)`;
    /// * `(call length x)`, `(call trim x)`, `(call contains list value)` and
    ///   `(call contains list object "field")`.
    ///
    /// ```
    /// use derust::sduix::stacx::v1::state::{Expr, StatePath};
    ///
    /// let e = Expr::from(StatePath::key("a")) + Expr::from(StatePath::key("b")) * Expr::int(2);
    /// assert_eq!(e.sexp(), "(+ a (* b 2))");
    /// ```
    pub fn sexp(&self) -> String {
        let mut out = String::new();
        // Writing to a `String` cannot fail.
        let _ = self.write_sexp(&mut out);
        out
    }

    fn write_sexp<W: Write>(&self, out: &mut W) -> fmt::Result {
        match self {
            Expr::Num(x) => {
                if *x < 0.0 {
                    out.write_str("(neg ")?;
                    write_number(out, *x)?;
                    out.write_char(')')
                } else {
                    write_number(out, *x)
                }
            }
            Expr::Int(n) => {
                if *n < 0 {
                    write!(out, "(neg {})", n.unsigned_abs())
                } else {
                    write!(out, "{n}")
                }
            }
            Expr::Text(text) => write_sexp_text(out, text),
            Expr::Bool(true) => out.write_str("true"),
            Expr::Bool(false) => out.write_str("false"),
            Expr::Null => out.write_str("null"),
            Expr::Ref(path) => write_sexp_path(out, path),
            Expr::Field(base, name) => {
                out.write_str("(. ")?;
                base.write_sexp(out)?;
                write!(out, " {name})")
            }
            Expr::Index(base, index) => {
                out.write_str("(idx ")?;
                base.write_sexp(out)?;
                out.write_char(' ')?;
                index.write_sexp(out)?;
                out.write_char(')')
            }
            Expr::Unary(op, operand) => {
                out.write_str(match op {
                    UnOp::Not => "(not ",
                    UnOp::Neg => "(neg ",
                })?;
                operand.write_sexp(out)?;
                out.write_char(')')
            }
            Expr::Binary(op, left, right) => {
                write!(out, "({} ", op.symbol())?;
                left.write_sexp(out)?;
                out.write_char(' ')?;
                right.write_sexp(out)?;
                out.write_char(')')
            }
            Expr::Ternary(condition, when_true, when_false) => {
                out.write_str("(? ")?;
                condition.write_sexp(out)?;
                out.write_char(' ')?;
                when_true.write_sexp(out)?;
                out.write_char(' ')?;
                when_false.write_sexp(out)?;
                out.write_char(')')
            }
            Expr::Group(inner) => inner.write_sexp(out),
            Expr::Length(value) => write_sexp_call1(out, "length", value),
            Expr::Trim(value) => write_sexp_call1(out, "trim", value),
            Expr::Contains { list, value } => {
                out.write_str("(call contains ")?;
                list.write_sexp(out)?;
                out.write_char(' ')?;
                value.write_sexp(out)?;
                out.write_char(')')
            }
            Expr::ContainsBy {
                list,
                object,
                field,
            } => {
                out.write_str("(call contains ")?;
                list.write_sexp(out)?;
                out.write_char(' ')?;
                object.write_sexp(out)?;
                out.write_char(' ')?;
                write_sexp_text(out, field)?;
                out.write_char(')')
            }
        }
    }
}

/// Magnitude of a number as the lexer accepts it: digits, with a fractional part only when
/// there is one (Rust never prints an exponent for `f64`). `-0.0` prints `0`.
fn write_number<W: Write>(out: &mut W, value: f64) -> fmt::Result {
    write!(out, "{}", value.abs())
}

/// Text between single quotes; escapes `\`, `'`, newline, tab and carriage return.
fn write_text<W: Write>(out: &mut W, text: &str) -> fmt::Result {
    out.write_char('\'')?;
    for c in text.chars() {
        match c {
            '\\' => out.write_str("\\\\")?,
            '\'' => out.write_str("\\'")?,
            '\n' => out.write_str("\\n")?,
            '\t' => out.write_str("\\t")?,
            '\r' => out.write_str("\\r")?,
            other => out.write_char(other)?,
        }
    }
    out.write_char('\'')
}

fn write_sexp_text<W: Write>(out: &mut W, text: &str) -> fmt::Result {
    out.write_char('"')?;
    for c in text.chars() {
        match c {
            '\\' => out.write_str("\\\\")?,
            '"' => out.write_str("\\\"")?,
            '\n' => out.write_str("\\n")?,
            '\t' => out.write_str("\\t")?,
            '\r' => out.write_str("\\r")?,
            other => out.write_char(other)?,
        }
    }
    out.write_char('"')
}

/// Base of `.field` / `[..]`: parenthesized below the postfix level or when it is a number
/// literal (`(1).x`).
fn write_postfix_base<W: Write>(out: &mut W, base: &Expr) -> fmt::Result {
    let numeric = matches!(base, Expr::Int(_) | Expr::Num(_));
    base.write_child(out, base.level() < LEVEL_PRIMARY || numeric)
}

fn write_call1<W: Write>(out: &mut W, name: &str, arg: &Expr) -> fmt::Result {
    out.write_str(name)?;
    out.write_char('(')?;
    arg.write_source(out)?;
    out.write_char(')')
}

fn write_sexp_call1<W: Write>(out: &mut W, name: &str, arg: &Expr) -> fmt::Result {
    write!(out, "(call {name} ")?;
    arg.write_sexp(out)?;
    out.write_char(')')
}

fn write_sexp_path<W: Write>(out: &mut W, path: &StatePath) -> fmt::Result {
    // The last segment is the outermost node: open it first.
    let mut open = String::new();
    let mut close = String::new();
    for segment in path.segments().iter().rev() {
        match segment {
            PathSegment::Field(_) => open.push_str("(. "),
            PathSegment::Index(_) => open.push_str("(idx "),
        }
    }
    out.write_str(&open)?;
    match path.root() {
        Root::Key(name) => write!(out, "{name}")?,
        Root::Global(name) => write!(out, "(. global {name})")?,
        Root::Params => out.write_str("params")?,
        Root::Result => out.write_str("result")?,
        Root::Response => out.write_str("response")?,
        Root::Event => out.write_str("event")?,
    }
    for segment in path.segments() {
        close.clear();
        match segment {
            PathSegment::Field(name) => write!(close, " {name})")?,
            PathSegment::Index(index) => write!(close, " {index})")?,
        }
        out.write_str(&close)?;
    }
    Ok(())
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_source(f)
    }
}

/// The expression with parentheses ignored (what the parser would produce).
fn without_groups(expr: &Expr) -> &Expr {
    let mut current = expr;
    while let Expr::Group(inner) = current {
        current = inner;
    }
    current
}

/// Deepest expression tree accepted. Printing, validating and dropping an expression are
/// recursive, so a tree coming from outside could overflow the stack.
pub const MAX_EXPR_DEPTH: usize = 128;

impl Expr {
    /// Whether the tree is deeper than `limit` levels (explicit stack, no recursion).
    fn deeper_than(&self, limit: usize) -> bool {
        let mut stack: Vec<(&Expr, usize)> = vec![(self, 1)];
        while let Some((expr, depth)) = stack.pop() {
            if depth > limit {
                return true;
            }
            let next = depth + 1;
            match expr {
                Expr::Num(_)
                | Expr::Int(_)
                | Expr::Text(_)
                | Expr::Bool(_)
                | Expr::Null
                | Expr::Ref(_) => {}
                Expr::Field(a, _)
                | Expr::Unary(_, a)
                | Expr::Group(a)
                | Expr::Length(a)
                | Expr::Trim(a) => stack.push((a, next)),
                Expr::Index(a, b)
                | Expr::Binary(_, a, b)
                | Expr::Contains { list: a, value: b }
                | Expr::ContainsBy {
                    list: a, object: b, ..
                } => {
                    stack.push((a, next));
                    stack.push((b, next));
                }
                Expr::Ternary(a, b, c) => {
                    stack.push((a, next));
                    stack.push((b, next));
                    stack.push((c, next));
                }
            }
        }
        false
    }
}

impl Validate for Expr {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        if self.deeper_than(MAX_EXPR_DEPTH) {
            issues.push(ValidationIssue::new(
                path,
                format!("expression is nested deeper than {MAX_EXPR_DEPTH} levels"),
            ));
            return;
        }
        self.validate_inner(path, issues);
    }
}

impl Expr {
    fn validate_inner(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        match self {
            Expr::Num(x) => {
                if !x.is_finite() {
                    issues.push(ValidationIssue::new(
                        path,
                        "number is not finite and cannot be written in an expression",
                    ));
                } else if x.fract() == 0.0 && x.abs() >= INT_LIMIT {
                    issues.push(ValidationIssue::new(
                        path,
                        "integer is too large: the Stac runtime only accepts integers below 2^63",
                    ));
                }
            }
            Expr::Int(n) => {
                if *n == i64::MIN {
                    issues.push(ValidationIssue::new(
                        path,
                        "integer is too large: the Stac runtime only accepts integers below 2^63",
                    ));
                }
            }
            Expr::Text(_) | Expr::Bool(_) | Expr::Null => {}
            Expr::Ref(state_path) => state_path.validate_at(path, issues),
            Expr::Field(base, name) => {
                base.validate_inner(&path.key("base"), issues);
                name.validate_at(&path.key("field"), issues);
            }
            Expr::Index(base, index) => {
                base.validate_inner(&path.key("base"), issues);
                let index_path = path.key("index");
                index.validate_inner(&index_path, issues);
                match without_groups(index) {
                    Expr::Binary(BinOp::Div, ..) => issues.push(ValidationIssue::new(
                        &index_path,
                        "index must be an integer: a division can produce a fraction",
                    )),
                    Expr::Num(x) if x.is_finite() && x.fract() != 0.0 => {
                        issues.push(ValidationIssue::new(
                            &index_path,
                            "index must be an integer: the number has a fractional part",
                        ))
                    }
                    _ => {}
                }
            }
            Expr::Unary(_, operand) => operand.validate_inner(&path.key("operand"), issues),
            Expr::Binary(_, left, right) => {
                left.validate_inner(&path.key("left"), issues);
                right.validate_inner(&path.key("right"), issues);
            }
            Expr::Ternary(condition, when_true, when_false) => {
                condition.validate_inner(&path.key("condition"), issues);
                when_true.validate_inner(&path.key("when_true"), issues);
                when_false.validate_inner(&path.key("when_false"), issues);
            }
            Expr::Group(inner) | Expr::Length(inner) | Expr::Trim(inner) => {
                inner.validate_inner(&path.key("value"), issues)
            }
            Expr::Contains { list, value } => {
                list.validate_inner(&path.key("list"), issues);
                value.validate_inner(&path.key("value"), issues);
            }
            Expr::ContainsBy { list, object, .. } => {
                list.validate_inner(&path.key("list"), issues);
                object.validate_inner(&path.key("object"), issues);
            }
        }
    }
}

impl From<StatePath> for Expr {
    fn from(value: StatePath) -> Self {
        Expr::Ref(value)
    }
}

impl From<f64> for Expr {
    fn from(value: f64) -> Self {
        Expr::Num(value)
    }
}

impl From<i64> for Expr {
    fn from(value: i64) -> Self {
        Expr::Int(value)
    }
}

impl From<i32> for Expr {
    fn from(value: i32) -> Self {
        Expr::Int(i64::from(value))
    }
}

impl From<u32> for Expr {
    fn from(value: u32) -> Self {
        Expr::Int(i64::from(value))
    }
}

impl From<&str> for Expr {
    fn from(value: &str) -> Self {
        Expr::Text(value.to_owned())
    }
}

impl From<String> for Expr {
    fn from(value: String) -> Self {
        Expr::Text(value)
    }
}

impl From<bool> for Expr {
    fn from(value: bool) -> Self {
        Expr::Bool(value)
    }
}

macro_rules! binary_operator {
    ($($trait:ident, $method:ident => $op:ident);* $(;)?) => {
        $(impl<R: Into<Expr>> std::ops::$trait<R> for Expr {
            type Output = Expr;

            fn $method(self, rhs: R) -> Expr {
                Expr::Binary(BinOp::$op, Box::new(self), boxed(rhs))
            }
        })*
    };
}

binary_operator! {
    Add, add => Add;
    Sub, sub => Sub;
    Mul, mul => Mul;
    Div, div => Div;
    Rem, rem => Rem;
}

impl std::ops::Neg for Expr {
    type Output = Expr;

    fn neg(self) -> Expr {
        Expr::Unary(UnOp::Neg, Box::new(self))
    }
}

impl std::ops::Not for Expr {
    type Output = Expr;

    fn not(self) -> Expr {
        Expr::Unary(UnOp::Not, Box::new(self))
    }
}
