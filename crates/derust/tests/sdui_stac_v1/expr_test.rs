//! Expression language (`Expr`): printing, validation and the precedence oracle.
//!
//! The oracle is a reference parser written here, following the grammar of the Stac fork
//! (`packages/stac/lib/src/state/expression/{lexer,parser}.dart`). A deterministic generator
//! (LCG with a fixed seed, no dependency) builds thousands of random trees and checks
//! `parse(print(e)) == e` (ignoring `Group`), that printing again is identical and that
//! `Expr::sexp()` agrees with an independent dump of the parsed structure.
use derust::sduix::stacx::v1::prelude::*;

// ---------------------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------------------

fn v(name: &str) -> Expr {
    Expr::from(StatePath::key(name))
}

fn src(e: &Expr) -> String {
    e.to_source()
}

fn issues(e: &Expr) -> Vec<ValidationIssue> {
    e.validate()
}

// ---------------------------------------------------------------------------------------
// reference parser (test only)
// ---------------------------------------------------------------------------------------

/// Structure produced by the parser (mirrors the Dart AST: no `Group`, no state paths).
#[derive(Debug, Clone, PartialEq)]
enum N {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
    Ident(String),
    Field(Box<N>, String),
    Index(Box<N>, Box<N>),
    Call(String, Vec<N>),
    Unary(&'static str, Box<N>),
    Binary(&'static str, Box<N>, Box<N>),
    Ternary(Box<N>, Box<N>, Box<N>),
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Int(i64),
    Float(f64),
    Str(String),
    Ident(String),
    Sym(&'static str),
    End,
}

const SYMBOLS: [&str; 24] = [
    "&&", "||", "==", "!=", "<=", ">=", "(", ")", "[", "]", ",", ".", "?", ":", "+", "-", "*", "/",
    "%", "!", "<", ">", "=", "&",
];

fn tokenize(source: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if matches!(c, ' ' | '\t' | '\n' | '\r') {
            i += 1;
        } else if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let mut is_float = false;
            if i + 1 < chars.len() && chars[i] == '.' && chars[i + 1].is_ascii_digit() {
                is_float = true;
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let text: String = chars[start..i].iter().collect();
            if is_float {
                tokens.push(Tok::Float(
                    text.parse().map_err(|_| format!("number {text}"))?,
                ));
            } else {
                // `int.tryParse`: fails above 2^63 - 1.
                tokens.push(Tok::Int(
                    text.parse().map_err(|_| format!("integer {text}"))?,
                ));
            }
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            tokens.push(Tok::Ident(chars[start..i].iter().collect()));
        } else if c == '\'' || c == '"' {
            let quote = c;
            let mut text = String::new();
            i += 1;
            let mut closed = false;
            while i < chars.len() {
                if chars[i] == '\\' {
                    if i + 1 >= chars.len() {
                        break;
                    }
                    text.push(match chars[i + 1] {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        other => other,
                    });
                    i += 2;
                } else if chars[i] == quote {
                    closed = true;
                    i += 1;
                    break;
                } else {
                    text.push(chars[i]);
                    i += 1;
                }
            }
            if !closed {
                return Err("unterminated string".into());
            }
            tokens.push(Tok::Str(text));
        } else {
            let rest: String = chars[i..chars.len().min(i + 2)].iter().collect();
            let symbol = SYMBOLS
                .iter()
                .find(|s| rest.starts_with(**s))
                .ok_or_else(|| format!("unexpected character {c}"))?;
            // a lone `=` or `&` is not a token
            if matches!(*symbol, "=" | "&") {
                return Err(format!("unexpected character {c}"));
            }
            tokens.push(Tok::Sym(symbol));
            i += symbol.len();
        }
    }
    tokens.push(Tok::End);
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Tok>,
    pos: usize,
}

const LEVELS: [&[&str]; 6] = [
    &["||"],
    &["&&"],
    &["==", "!="],
    &["<", "<=", ">", ">="],
    &["+", "-"],
    &["*", "/", "%"],
];

impl Parser {
    fn current(&self) -> &Tok {
        &self.tokens[self.pos]
    }

    fn eat(&mut self, symbol: &str) -> bool {
        if matches!(self.current(), Tok::Sym(s) if *s == symbol) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, symbol: &str) -> Result<(), String> {
        if self.eat(symbol) {
            Ok(())
        } else {
            Err(format!("expected {symbol} at {:?}", self.current()))
        }
    }

    fn ternary(&mut self) -> Result<N, String> {
        let condition = self.binary(0)?;
        if !self.eat("?") {
            return Ok(condition);
        }
        let when_true = self.ternary()?;
        self.expect(":")?;
        let when_false = self.ternary()?;
        Ok(N::Ternary(
            Box::new(condition),
            Box::new(when_true),
            Box::new(when_false),
        ))
    }

    fn binary(&mut self, level: usize) -> Result<N, String> {
        if level >= LEVELS.len() {
            return self.unary();
        }
        let mut left = self.binary(level + 1)?;
        loop {
            let op = match self.current() {
                Tok::Sym(s) => LEVELS[level].iter().find(|o| *o == s).copied(),
                _ => None,
            };
            let Some(op) = op else { return Ok(left) };
            self.pos += 1;
            let right = self.binary(level + 1)?;
            left = N::Binary(op, Box::new(left), Box::new(right));
        }
    }

    fn unary(&mut self) -> Result<N, String> {
        if self.eat("!") {
            return Ok(N::Unary("not", Box::new(self.unary()?)));
        }
        if self.eat("-") {
            return Ok(N::Unary("neg", Box::new(self.unary()?)));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<N, String> {
        let mut node = self.primary()?;
        loop {
            if self.eat(".") {
                match self.current().clone() {
                    Tok::Ident(name) => {
                        self.pos += 1;
                        node = N::Field(Box::new(node), name);
                    }
                    other => return Err(format!("expected a field name, got {other:?}")),
                }
            } else if self.eat("[") {
                let index = self.ternary()?;
                self.expect("]")?;
                node = N::Index(Box::new(node), Box::new(index));
            } else {
                return Ok(node);
            }
        }
    }

    fn primary(&mut self) -> Result<N, String> {
        let token = self.current().clone();
        match token {
            Tok::Int(n) => {
                self.pos += 1;
                Ok(N::Int(n))
            }
            Tok::Float(x) => {
                self.pos += 1;
                Ok(N::Float(x))
            }
            Tok::Str(s) => {
                self.pos += 1;
                Ok(N::Str(s))
            }
            Tok::Sym("(") => {
                self.pos += 1;
                let inner = self.ternary()?;
                self.expect(")")?;
                Ok(inner)
            }
            Tok::Ident(name) => {
                self.pos += 1;
                if matches!(self.current(), Tok::Sym("(")) {
                    return self.call(name);
                }
                Ok(match name.as_str() {
                    "true" => N::Bool(true),
                    "false" => N::Bool(false),
                    "null" => N::Null,
                    _ => N::Ident(name),
                })
            }
            other => Err(format!("unexpected {other:?}")),
        }
    }

    fn call(&mut self, name: String) -> Result<N, String> {
        let (min, max) = match name.as_str() {
            "length" | "trim" => (1, 1),
            "contains" => (2, 3),
            _ => return Err(format!("unknown function {name}")),
        };
        self.expect("(")?;
        let mut args = Vec::new();
        if !matches!(self.current(), Tok::Sym(")")) {
            loop {
                args.push(self.ternary()?);
                if !self.eat(",") {
                    break;
                }
            }
        }
        self.expect(")")?;
        if args.len() < min || args.len() > max {
            return Err(format!("wrong number of arguments for {name}"));
        }
        Ok(N::Call(name, args))
    }
}

fn parse(source: &str) -> Result<N, String> {
    let mut parser = Parser {
        tokens: tokenize(source)?,
        pos: 0,
    };
    let node = parser.ternary()?;
    if *parser.current() != Tok::End {
        return Err(format!("unexpected {:?}", parser.current()));
    }
    Ok(node)
}

/// Structure of an `Expr` as the parser would see it (no `Group`, paths expanded, numbers
/// normalized: integral values are integers, negative literals are a unary minus).
fn structure(e: &Expr) -> N {
    fn b(n: N) -> Box<N> {
        Box::new(n)
    }
    fn path(p: &StatePath) -> N {
        let mut node = match p.root() {
            Root::Key(name) => N::Ident(name.as_str().into()),
            Root::Global(name) => N::Field(b(N::Ident("global".into())), name.as_str().into()),
            Root::Params => N::Ident("params".into()),
            Root::Result => N::Ident("result".into()),
            Root::Response => N::Ident("response".into()),
            Root::Event => N::Ident("event".into()),
        };
        for segment in p.segments() {
            node = match segment {
                PathSegment::Field(name) => N::Field(b(node), name.as_str().into()),
                PathSegment::Index(i) => N::Index(b(node), b(N::Int(*i as i64))),
            };
        }
        node
    }
    match e {
        Expr::Int(n) if *n < 0 => N::Unary("neg", b(N::Int(n.abs()))),
        Expr::Int(n) => N::Int(*n),
        Expr::Num(x) => {
            let magnitude = x.abs();
            let literal = if magnitude.fract() == 0.0 {
                N::Int(magnitude as i64)
            } else {
                N::Float(magnitude)
            };
            if *x < 0.0 {
                N::Unary("neg", b(literal))
            } else {
                literal
            }
        }
        Expr::Text(t) => N::Str(t.clone()),
        Expr::Bool(x) => N::Bool(*x),
        Expr::Null => N::Null,
        Expr::Ref(p) => path(p),
        Expr::Field(base, name) => N::Field(b(structure(base)), name.as_str().into()),
        Expr::Index(base, index) => N::Index(b(structure(base)), b(structure(index))),
        Expr::Unary(op, x) => N::Unary(
            match op {
                UnOp::Not => "not",
                UnOp::Neg => "neg",
            },
            b(structure(x)),
        ),
        Expr::Binary(op, l, r) => {
            let symbol = match op {
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
            };
            N::Binary(symbol, b(structure(l)), b(structure(r)))
        }
        Expr::Ternary(c, t, f) => N::Ternary(b(structure(c)), b(structure(t)), b(structure(f))),
        Expr::Group(inner) => structure(inner),
        Expr::Length(x) => N::Call("length".into(), vec![structure(x)]),
        Expr::Trim(x) => N::Call("trim".into(), vec![structure(x)]),
        Expr::Contains { list, value } => {
            N::Call("contains".into(), vec![structure(list), structure(value)])
        }
        Expr::ContainsBy {
            list,
            object,
            field,
        } => N::Call(
            "contains".into(),
            vec![structure(list), structure(object), N::Str(field.clone())],
        ),
    }
}

/// Independent dump of `N` in the documented `sexp` format.
fn n_sexp(n: &N) -> String {
    match n {
        N::Int(i) => i.to_string(),
        N::Float(x) => x.to_string(),
        N::Str(s) => {
            let mut out = String::from("\"");
            for c in s.chars() {
                match c {
                    '\\' => out.push_str("\\\\"),
                    '"' => out.push_str("\\\""),
                    '\n' => out.push_str("\\n"),
                    '\t' => out.push_str("\\t"),
                    '\r' => out.push_str("\\r"),
                    other => out.push(other),
                }
            }
            out.push('"');
            out
        }
        N::Bool(b) => b.to_string(),
        N::Null => "null".into(),
        N::Ident(name) => name.clone(),
        N::Field(base, name) => format!("(. {} {name})", n_sexp(base)),
        N::Index(base, index) => format!("(idx {} {})", n_sexp(base), n_sexp(index)),
        N::Call(name, args) => {
            let args: Vec<String> = args.iter().map(n_sexp).collect();
            format!("(call {name} {})", args.join(" "))
        }
        N::Unary(op, x) => format!("({op} {})", n_sexp(x)),
        N::Binary(op, l, r) => format!("({op} {} {})", n_sexp(l), n_sexp(r)),
        N::Ternary(c, t, f) => format!("(? {} {} {})", n_sexp(c), n_sexp(t), n_sexp(f)),
    }
}

// ---------------------------------------------------------------------------------------
// random trees
// ---------------------------------------------------------------------------------------

/// Deterministic 64-bit LCG (Knuth's MMIX constants).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

const NAMES: [&str; 10] = [
    "a", "b", "c", "items", "name", "user", "x1", "_t", "item", "length_",
];
const FIELDS: [&str; 6] = ["id", "x", "length", "contains", "a_b", "Z9"];
const BIN_OPS: [BinOp; 13] = [
    BinOp::Add,
    BinOp::Sub,
    BinOp::Mul,
    BinOp::Div,
    BinOp::Rem,
    BinOp::Lt,
    BinOp::Le,
    BinOp::Gt,
    BinOp::Ge,
    BinOp::Eq,
    BinOp::Ne,
    BinOp::And,
    BinOp::Or,
];

fn random_text(rng: &mut Lcg) -> String {
    const ALPHABET: [char; 14] = [
        'a', 'Z', '0', ' ', '\'', '"', '\\', '\n', '\t', '\r', 'é', '日', '$', '{',
    ];
    let len = rng.below(6);
    (0..len).map(|_| *rng.pick(&ALPHABET)).collect()
}

fn random_path(rng: &mut Lcg) -> StatePath {
    let mut path = match rng.below(8) {
        0 => StatePath::global(*rng.pick(&FIELDS)),
        1 => StatePath::params(),
        2 => StatePath::result(),
        3 => StatePath::response(),
        4 => StatePath::event(),
        _ => StatePath::key(*rng.pick(&NAMES)),
    };
    for _ in 0..rng.below(3) {
        path = if rng.below(2) == 0 {
            path.field(*rng.pick(&FIELDS))
        } else {
            path.index(rng.below(4))
        };
    }
    path
}

fn random_leaf(rng: &mut Lcg) -> Expr {
    match rng.below(9) {
        0 => Expr::Int(rng.below(2000) as i64 - 1000),
        1 => Expr::Int(*rng.pick(&[0, i64::MAX, i64::MIN + 1, -1, 1])),
        2 => Expr::Num(*rng.pick(&[0.5, -1.5, 0.1, 1e-7, 123456.789, 2.0, -3.0, 0.0, -0.0])),
        3 => Expr::Num((rng.below(4000) as f64 - 2000.0) / 8.0),
        4 => Expr::Text(random_text(rng)),
        5 => Expr::Bool(rng.below(2) == 0),
        6 => Expr::Null,
        _ => Expr::Ref(random_path(rng)),
    }
}

fn random_expr(rng: &mut Lcg, depth: u32, budget: &mut u32) -> Expr {
    if depth == 0 || *budget == 0 || rng.below(10) < 2 {
        return random_leaf(rng);
    }
    *budget -= 1;
    let mut sub = |rng: &mut Lcg| Box::new(random_expr(rng, depth - 1, budget));
    match rng.below(12) {
        0..=3 => {
            let op = *rng.pick(&BIN_OPS);
            Expr::Binary(op, sub(rng), sub(rng))
        }
        4 => Expr::Unary(
            if rng.below(2) == 0 {
                UnOp::Not
            } else {
                UnOp::Neg
            },
            sub(rng),
        ),
        5 => Expr::Ternary(sub(rng), sub(rng), sub(rng)),
        6 => Expr::Group(sub(rng)),
        7 => Expr::Field(sub(rng), Ident::new(*rng.pick(&FIELDS))),
        8 => Expr::Index(sub(rng), sub(rng)),
        9 => match rng.below(3) {
            0 => Expr::Length(sub(rng)),
            1 => Expr::Trim(sub(rng)),
            _ => Expr::Contains {
                list: sub(rng),
                value: sub(rng),
            },
        },
        10 => Expr::ContainsBy {
            list: sub(rng),
            object: sub(rng),
            field: random_text(rng),
        },
        _ => random_leaf(rng),
    }
}

#[test]
fn oracle_parse_of_print_is_the_original_tree() {
    let mut rng = Lcg(0x5EED_0001);
    let mut checked = 0;
    let mut deep = 0;
    for _ in 0..8000 {
        let depth = 1 + rng.below(8) as u32;
        let mut budget = 40;
        let e = random_expr(&mut rng, depth, &mut budget);
        let printed = e.to_source();
        let parsed = parse(&printed)
            .unwrap_or_else(|err| panic!("`{printed}` does not parse ({err}); tree: {e:?}"));
        assert_eq!(parsed, structure(&e), "source: `{printed}`\ntree: {e:?}");
        // the dump agrees with an independent dump of the parsed structure
        assert_eq!(e.sexp(), n_sexp(&parsed), "source: `{printed}`");
        // printing is stable: print(parse(print(e))) is the same text once parsed again
        let reprinted = print_n_roundtrip(&parsed);
        assert_eq!(
            parse(&reprinted).unwrap_or_else(|err| panic!("`{reprinted}`: {err}")),
            parsed
        );
        // second printing of the same tree is identical (deterministic)
        assert_eq!(e.to_source(), printed);
        assert_eq!(e.to_string(), printed);
        checked += 1;
        if depth >= 7 {
            deep += 1;
        }
    }
    assert!(checked >= 5000);
    assert!(deep > 100, "the generator must reach deep trees ({deep})");
}

/// Converts a parsed structure back to an `Expr` (without groups) and prints it, proving that
/// the printer is idempotent on the canonical form of a tree.
fn print_n_roundtrip(n: &N) -> String {
    fn to_expr(n: &N) -> Expr {
        match n {
            N::Int(i) => Expr::Int(*i),
            N::Float(x) => Expr::Num(*x),
            N::Str(s) => Expr::Text(s.clone()),
            N::Bool(b) => Expr::Bool(*b),
            N::Null => Expr::Null,
            N::Ident(name) => Expr::from(StatePath::key(name.as_str())),
            N::Field(base, name) => Expr::Field(Box::new(to_expr(base)), Ident::new(name.as_str())),
            N::Index(base, index) => Expr::Index(Box::new(to_expr(base)), Box::new(to_expr(index))),
            N::Call(name, args) => match (name.as_str(), args.as_slice()) {
                ("length", [x]) => Expr::length(to_expr(x)),
                ("trim", [x]) => Expr::trim(to_expr(x)),
                ("contains", [l, x]) => Expr::contains(to_expr(l), to_expr(x)),
                ("contains", [l, o, N::Str(f)]) => {
                    Expr::contains_by(to_expr(l), to_expr(o), f.as_str())
                }
                other => panic!("unexpected call {other:?}"),
            },
            N::Unary(op, x) => Expr::Unary(
                if *op == "not" { UnOp::Not } else { UnOp::Neg },
                Box::new(to_expr(x)),
            ),
            N::Binary(op, l, r) => {
                let op = BIN_OPS
                    .iter()
                    .copied()
                    .find(|candidate| {
                        Expr::Binary(*candidate, Box::new(Expr::Null), Box::new(Expr::Null))
                            .to_source()
                            == format!("null {op} null")
                    })
                    .unwrap_or_else(|| panic!("operator {op}"));
                Expr::Binary(op, Box::new(to_expr(l)), Box::new(to_expr(r)))
            }
            N::Ternary(c, t, f) => Expr::cond(to_expr(c), to_expr(t), to_expr(f)),
        }
    }
    to_expr(n).to_source()
}

#[test]
fn oracle_rejects_what_the_runtime_rejects() {
    // sanity of the reference parser itself
    assert!(parse("a +").is_err());
    assert!(parse("foo(1)").is_err());
    assert!(parse("length()").is_err());
    assert!(parse("contains(a)").is_err());
    assert!(parse("'abc").is_err());
    assert!(parse("9223372036854775808").is_err());
    assert!(parse("9223372036854775807").is_ok());
    assert!(parse("1.").is_err());
    assert!(parse("a = b").is_err());
    // associativity and precedence the oracle must implement
    assert_eq!(n_sexp(&parse("a - b - c").unwrap()), "(- (- a b) c)");
    assert_eq!(
        n_sexp(&parse("a ? b : c ? d : e").unwrap()),
        "(? a b (? c d e))"
    );
    assert_eq!(n_sexp(&parse("!a == b").unwrap()), "(== (not a) b)");
    assert_eq!(n_sexp(&parse("-a.b").unwrap()), "(neg (. a b))");
    assert_eq!(n_sexp(&parse("a || b && c").unwrap()), "(|| a (&& b c))");
    assert_eq!(n_sexp(&parse("a < b == c").unwrap()), "(== (< a b) c)");
    assert_eq!(n_sexp(&parse("a + b * c").unwrap()), "(+ a (* b c))");
}

// ---------------------------------------------------------------------------------------
// manual cases
// ---------------------------------------------------------------------------------------

#[test]
fn binary_precedence_and_associativity() {
    let (a, b, c) = (v("a"), v("b"), v("c"));
    assert_eq!(src(&((a.clone() + b.clone()) * c.clone())), "(a + b) * c");
    assert_eq!(src(&(a.clone() + b.clone() * c.clone())), "a + b * c");
    assert_eq!(src(&(a.clone() * b.clone() + c.clone())), "a * b + c");
    assert_eq!(src(&(a.clone() - (b.clone() - c.clone()))), "a - (b - c)");
    assert_eq!(src(&((a.clone() - b.clone()) - c.clone())), "a - b - c");
    assert_eq!(src(&(a.clone() / (b.clone() * c.clone()))), "a / (b * c)");
    assert_eq!(src(&((a.clone() / b.clone()) * c.clone())), "a / b * c");
    assert_eq!(src(&(a.clone() % (b.clone() % c.clone()))), "a % (b % c)");
    assert_eq!(
        src(&(a.clone().or(b.clone())).and(c.clone())),
        "(a || b) && c"
    );
    assert_eq!(src(&a.clone().or(b.clone().and(c.clone()))), "a || b && c");
    assert_eq!(src(&a.clone().eq_(b.clone()).eq_(c.clone())), "a == b == c");
    assert_eq!(
        src(&a.clone().eq_(b.clone().eq_(c.clone()))),
        "a == (b == c)"
    );
    assert_eq!(src(&a.clone().lt(b.clone()).eq_(c.clone())), "a < b == c");
    assert_eq!(src(&a.clone().eq_(b.clone().lt(c.clone()))), "a == b < c");
    assert_eq!(src(&(a.clone().lt(b.clone())).lt(c.clone())), "a < b < c");
    assert_eq!(src(&a.clone().lt(b.clone().lt(c.clone()))), "a < (b < c)");
    assert_eq!(src(&a.clone().ne_(b.clone())), "a != b");
    assert_eq!(src(&a.clone().le(b.clone())), "a <= b");
    assert_eq!(src(&a.clone().gt(b.clone())), "a > b");
    assert_eq!(src(&a.ge(b)), "a >= b");
}

#[test]
fn unary_operators() {
    let (a, b) = (v("a"), v("b"));
    assert_eq!(src(&-(a.clone() + b.clone())), "-(a + b)");
    assert_eq!(src(&-(-v("x"))), "- -x");
    assert_eq!(src(&!(!v("x"))), "! !x");
    assert_eq!(src(&-(!v("x"))), "- !x");
    assert_eq!(src(&!(a.clone().and(b.clone()))), "!(a && b)");
    assert_eq!(src(&(!a.clone()).eq_(b.clone())), "!a == b");
    assert_eq!(src(&!(a.clone().eq_(b.clone()))), "!(a == b)");
    assert_eq!(src(&-v("a").field("b")), "-a.b");
    assert_eq!(src(&(-v("a")).field("b")), "(-a).b");
    assert_eq!(src(&(a.clone() * -b.clone())), "a * -b");
    assert_eq!(src(&(a - -b)), "a - -b");
    // an operand that is a ternary needs parentheses
    assert_eq!(src(&!Expr::cond(v("a"), v("b"), v("c"))), "!(a ? b : c)");
}

#[test]
fn ternary_nesting_in_the_three_places() {
    let (a, b, c, d, e) = (v("a"), v("b"), v("c"), v("d"), v("e"));
    // else branch: right associative, no parentheses
    assert_eq!(
        src(&Expr::cond(
            a.clone(),
            b.clone(),
            Expr::cond(c.clone(), d.clone(), e.clone())
        )),
        "a ? b : c ? d : e"
    );
    // condition: parentheses
    assert_eq!(
        src(&Expr::cond(
            Expr::cond(a.clone(), b.clone(), c.clone()),
            d.clone(),
            e.clone()
        )),
        "(a ? b : c) ? d : e"
    );
    // then branch: no parentheses
    assert_eq!(
        src(&Expr::cond(
            a.clone(),
            Expr::cond(b.clone(), c.clone(), d.clone()),
            e.clone()
        )),
        "a ? b ? c : d : e"
    );
    // as operand of a binary operator
    assert_eq!(
        src(&(Expr::cond(a.clone(), b.clone(), c.clone()) + d.clone())),
        "(a ? b : c) + d"
    );
    assert_eq!(
        src(&(d.clone() + Expr::cond(a.clone(), b.clone(), c.clone()))),
        "d + (a ? b : c)"
    );
    // a binary condition needs no parentheses
    assert_eq!(src(&Expr::cond(a.or(b), c, d)), "a || b ? c : d");
}

#[test]
fn postfix_bases_and_indexes() {
    let (a, b, n) = (v("a"), v("b"), v("n"));
    assert_eq!(src(&(a.clone() + b).field("x")), "(a + b).x");
    assert_eq!(src(&Expr::int(1).field("x")), "(1).x");
    assert_eq!(src(&Expr::num(1.5).field("x")), "(1.5).x");
    assert_eq!(src(&Expr::int(-1).field("x")), "(-1).x");
    assert_eq!(src(&Expr::int(1).at(0)), "(1)[0]");
    assert_eq!(src(&Expr::text("ab").field("length")), "'ab'.length");
    assert_eq!(src(&v("items").at(n.clone() + 1)), "items[n + 1]");
    assert_eq!(
        src(&v("items").at(Expr::cond(n.clone(), 1, 2))),
        "items[n ? 1 : 2]"
    );
    assert_eq!(src(&v("items").at(v("items").at(0))), "items[items[0]]");
    assert_eq!(src(&a.clone().field("x").at(0).field("y")), "a.x[0].y");
    assert_eq!(src(&(!a.clone()).field("x")), "(!a).x");
    assert_eq!(src(&Expr::length(a).field("x")), "length(a).x");
    // dynamic index: prints fine, Validate rejects what can be fractional
    let divided = v("items").at(n.clone() / 2);
    assert_eq!(src(&divided), "items[n / 2]");
    let found = issues(&divided);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].path, "$.index");
    assert_eq!(issues(&v("items").at((n.clone() / 2).grouped())).len(), 1);
    assert_eq!(issues(&v("items").at(Expr::num(1.5))).len(), 1);
    assert_eq!(issues(&v("items").at(Expr::num(-1.5).grouped())).len(), 1);
    assert!(issues(&v("items").at(Expr::num(2.0))).is_empty());
    assert!(issues(&v("items").at(n.clone() % 2)).is_empty());
    assert!(issues(&v("items").at(n * 2)).is_empty());
}

#[test]
fn groups_always_keep_their_parentheses() {
    let (a, b) = (v("a"), v("b"));
    assert_eq!(src(&a.clone().grouped()), "(a)");
    assert_eq!(src(&(a.clone() + b.clone()).grouped()), "(a + b)");
    assert_eq!(
        src(&(a.clone().grouped() + b.clone().grouped())),
        "(a) + (b)"
    );
    assert_eq!(src(&(a + b).grouped().grouped()), "((a + b))");
    // a group is invisible to the structure
    assert_eq!(v("a").grouped().sexp(), "a");
    assert_eq!((v("a") + v("b")).grouped().sexp(), "(+ a b)");
}

#[test]
fn functions() {
    assert_eq!(
        src(&Expr::length(Expr::trim(v("name"))).lt(3)),
        "length(trim(name)) < 3"
    );
    assert_eq!(
        src(&Expr::contains(v("tags"), "none")),
        "contains(tags, 'none')"
    );
    assert_eq!(
        src(&Expr::contains_by(v("list"), v("obj"), "id")),
        "contains(list, obj, 'id')"
    );
    assert_eq!(
        src(&Expr::contains(
            Expr::cond(v("a"), v("b"), v("c")),
            v("x").or(v("y"))
        )),
        "contains(a ? b : c, x || y)"
    );
    assert_eq!(
        src(&Expr::contains_by(v("l"), v("o"), "it's")),
        "contains(l, o, 'it\\'s')"
    );
    assert_eq!(
        Expr::contains_by(v("list"), v("obj"), "id").sexp(),
        "(call contains list obj \"id\")"
    );
    assert_eq!(Expr::length(v("a")).sexp(), "(call length a)");
    assert_eq!(Expr::trim(v("a")).sexp(), "(call trim a)");
}

#[test]
fn numbers() {
    assert_eq!(src(&Expr::num(-1.5)), "-1.5");
    assert_eq!(src(&Expr::num(2.0)), "2");
    assert_eq!(src(&Expr::num(0.1)), "0.1");
    assert_eq!(src(&Expr::num(0.0)), "0");
    assert_eq!(src(&Expr::num(-0.0)), "0");
    assert_eq!(src(&Expr::num(-3.0)), "-3");
    assert_eq!(src(&Expr::num(1e-7)), "0.0000001");
    assert_eq!(src(&Expr::int(0)), "0");
    assert_eq!(src(&Expr::int(-42)), "-42");
    assert_eq!(src(&Expr::int(i64::MAX)), "9223372036854775807");
    // a negative literal is a unary minus: precedence and spacing follow
    assert_eq!(src(&(v("a") - Expr::int(-5))), "a - -5");
    assert_eq!(src(&(-Expr::int(-5))), "- -5");
    assert_eq!(src(&(-Expr::num(-1.5))), "- -1.5");
    assert_eq!(src(&!Expr::int(-5)), "! -5");
    assert_eq!(Expr::num(-1.5).sexp(), "(neg 1.5)");
    assert_eq!(Expr::int(-2).sexp(), "(neg 2)");
    assert_eq!(Expr::num(2.0).sexp(), "2");
    assert_eq!(Expr::num(0.1).sexp(), "0.1");

    for ok in [
        Expr::num(0.1),
        Expr::num(-1.5),
        Expr::num(2.0),
        Expr::int(i64::MAX),
        Expr::int(i64::MIN + 1),
        Expr::num(9.0e18),
    ] {
        assert!(issues(&ok).is_empty(), "{ok:?}");
    }

    // 1e21 prints its digits but is an integer above 2^63: the runtime would fail to parse it
    assert_eq!(src(&Expr::num(1e21)), "1000000000000000000000");
    for bad in [
        Expr::num(1e21),
        Expr::num(-1e21),
        Expr::num(9.3e18),
        Expr::int(i64::MIN),
        Expr::num(f64::NAN),
        Expr::num(f64::INFINITY),
        Expr::num(f64::NEG_INFINITY),
    ] {
        assert_eq!(issues(&bad).len(), 1, "{bad:?}");
        // printing never panics
        let _ = bad.to_source();
        let _ = bad.sexp();
    }
    assert!(!i64::MIN.to_string().is_empty());
    // the error is located
    let nested = v("a") + (v("b") * Expr::num(f64::NAN));
    let found = nested.validate();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, "$.right.right");
}

#[test]
fn texts() {
    assert_eq!(src(&Expr::text("a'b\\c")), "'a\\'b\\\\c'");
    assert_eq!(src(&Expr::text("")), "''");
    assert_eq!(src(&Expr::text("say \"hi\"")), "'say \"hi\"'");
    assert_eq!(src(&Expr::text("l1\nl2\tx\ry")), "'l1\\nl2\\tx\\ry'");
    assert_eq!(src(&Expr::text("é日本🙂")), "'é日本🙂'");
    assert_eq!(src(&Expr::text("{{x}} $state")), "'{{x}} $state'");
    assert_eq!(src(&Expr::text("a\\")), "'a\\\\'");
    // what is printed is read back unchanged
    for text in ["a'b\\c", "", "\"", "\n\t\r", "é日本🙂", "\\", "\\'", "'\\"] {
        let printed = src(&Expr::text(text));
        assert_eq!(parse(&printed).unwrap(), N::Str(text.into()), "{printed}");
    }
    assert_eq!(Expr::text("a\"b\\c\n").sexp(), "\"a\\\"b\\\\c\\n\"");
    assert!(issues(&Expr::text("x'\\")).is_empty());
}

#[test]
fn literals_and_conversions() {
    assert_eq!(src(&Expr::bool(true)), "true");
    assert_eq!(src(&Expr::from(false)), "false");
    assert_eq!(src(&Expr::null()), "null");
    assert_eq!(src(&Expr::from(3)), "3");
    assert_eq!(src(&Expr::from(3u32)), "3");
    assert_eq!(src(&Expr::from(3i64)), "3");
    assert_eq!(src(&Expr::from(0.5)), "0.5");
    assert_eq!(src(&Expr::from("x")), "'x'");
    assert_eq!(src(&Expr::from(String::from("x"))), "'x'");
    assert_eq!(src(&Expr::path(StatePath::global("token"))), "global.token");
    assert_eq!(format!("{}", v("a") + 1), "a + 1");
}

#[test]
fn state_paths_and_identifiers() {
    assert_eq!(src(&Expr::from(StatePath::global("token"))), "global.token");
    assert_eq!(
        src(&Expr::from(StatePath::key("user").field("name"))),
        "user.name"
    );
    assert_eq!(
        src(&Expr::from(StatePath::key("items").index(0))),
        "items[0]"
    );
    assert_eq!(
        src(&Expr::from(StatePath::params().field("name"))),
        "params.name"
    );
    assert_eq!(
        src(&Expr::from(
            StatePath::response().field("body").field("items")
        )),
        "response.body.items"
    );
    assert_eq!(
        Expr::from(StatePath::global("token")).sexp(),
        "(. global token)"
    );
    assert_eq!(
        Expr::from(StatePath::key("user").field("address").field("city")).sexp(),
        "(. (. user address) city)"
    );
    assert_eq!(
        Expr::from(StatePath::global("a").index(0).field("b")).sexp(),
        "(. (idx (. global a) 0) b)"
    );

    for bad in ["user-name", "1abc", "é", "true", "false", "null", ""] {
        let found = issues(&Expr::from(StatePath::key(bad)));
        assert_eq!(found.len(), 1, "{bad}");
        assert_eq!(found[0].path, "$.root", "{bad}");
        // the message never echoes the offending text (keywords are named by the rule)
        if !matches!(bad, "" | "true" | "false" | "null") {
            assert!(!found[0].message.contains(bad), "{bad}");
        }
    }
    let found = issues(&(v("ok") + Expr::from(StatePath::global("bad-key"))));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, "$.right.root");
    let found = issues(&v("a").field("true"));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, "$.field");
    assert!(issues(&v("a").field("b").at(0)).is_empty());
}

#[test]
fn per_item_list_state_patterns() {
    let item = || StatePath::key("item");
    assert_eq!(src(&Expr::from(item().field("id"))), "item.id");
    assert_eq!(
        src(&Expr::from(StatePath::key("todo").field("name"))),
        "todo.name"
    );
    assert_eq!(src(&Expr::from(StatePath::key("i"))), "i");
    assert_eq!(
        src(&Expr::contains(v("selected"), item().field("id"))),
        "contains(selected, item.id)"
    );
    assert_eq!(
        src(&Expr::contains_by(v("selected"), item(), "id")),
        "contains(selected, item, 'id')"
    );
    let limited = Expr::length(v("selected"))
        .lt(3)
        .or(Expr::contains(v("selected"), item().field("id")));
    assert_eq!(
        src(&limited),
        "length(selected) < 3 || contains(selected, item.id)"
    );
    assert!(issues(&limited).is_empty());
    assert_eq!(
        limited.sexp(),
        "(|| (< (call length selected) 3) (call contains selected (. item id)))"
    );
    // `||` binds looser than `<`: the other grouping needs parentheses
    let other = Expr::length(v("selected"))
        .lt(Expr::int(3).or(Expr::contains(v("selected"), item().field("id"))));
    assert_eq!(
        src(&other),
        "length(selected) < (3 || contains(selected, item.id))"
    );
    for text in [src(&limited), src(&other)] {
        assert!(parse(&text).is_ok());
    }
}

#[test]
fn sexp_of_the_basics() {
    assert_eq!((v("a") + v("b") * v("c")).sexp(), "(+ a (* b c))");
    assert_eq!(((v("a") + v("b")) * v("c")).sexp(), "(* (+ a b) c)");
    assert_eq!(
        Expr::cond(v("a"), v("b"), Expr::cond(v("c"), v("d"), v("e"))).sexp(),
        "(? a b (? c d e))"
    );
    assert_eq!((-(v("a") + v("b"))).sexp(), "(neg (+ a b))");
    assert_eq!((!v("a")).sexp(), "(not a)");
    assert_eq!(
        v("items").at(Expr::int(0)).field("id").sexp(),
        "(. (idx items 0) id)"
    );
    assert_eq!(Expr::null().sexp(), "null");
    assert_eq!(Expr::bool(false).sexp(), "false");
    assert_eq!(Expr::text("x").sexp(), "\"x\"");
    assert_eq!(v("a").lt(v("b")).sexp(), "(< a b)");
    assert_eq!(v("a").ne_(v("b")).sexp(), "(!= a b)");
    assert_eq!(v("a").and(v("b")).or(v("c")).sexp(), "(|| (&& a b) c)");
    assert_eq!((v("a") % v("b")).sexp(), "(% a b)");
}

#[test]
fn bindings_serialize_as_a_single_pair() {
    let state = Binding::from(StatePath::global("token"));
    assert_eq!(
        serde_json::to_value(&state).unwrap(),
        serde_json::json!({"$state": "global.token"})
    );
    let expr = Binding::from(Expr::length(v("tags")).gt(0));
    assert_eq!(
        serde_json::to_value(&expr).unwrap(),
        serde_json::json!({"$expr": "length(tags) > 0"})
    );
    let quoted = Binding::expr(Expr::text("it's").eq_(v("a")));
    assert_eq!(
        serde_json::to_value(&quoted).unwrap(),
        serde_json::json!({"$expr": "'it\\'s' == a"})
    );
    assert!(Binding::state(StatePath::key("a")).validate().is_empty());
}
