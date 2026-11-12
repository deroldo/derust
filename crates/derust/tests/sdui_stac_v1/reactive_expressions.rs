//! Expression fixtures: one file per category in `fixtures_reactive/expressions`, a list of
//! `{source, sexp}` produced from typed `Expr` values. The Dart harness
//! (`tools/stac-compat/v1-reactive`) parses each `source` with the parser of the fork and compares
//! the dump of its AST with `sexp`.
//!
//! The files are checked in. To regenerate them after an intended change run the tests with
//! `UPDATE_REACTIVE_FIXTURES=1`.
use derust::sduix::stacx::v1::state::{Expr, StatePath};
use serde_json::{json, Value};

fn v(name: &str) -> Expr {
    Expr::from(StatePath::key(name))
}

fn p(path: StatePath) -> Expr {
    Expr::from(path)
}

fn int(n: i64) -> Expr {
    Expr::int(n)
}

/// Every category and its expressions. The category is the fixture file name.
pub fn categories() -> Vec<(&'static str, Vec<Expr>)> {
    vec![
        (
            "literals",
            vec![
                int(0),
                int(42),
                Expr::num(1.5),
                Expr::num(0.25),
                Expr::num(3.0),
                -int(5),
                -Expr::num(2.5),
                Expr::text("hello"),
                Expr::text(""),
                Expr::text("it's"),
                Expr::text("say \"hi\""),
                Expr::text("a\\b"),
                Expr::text("line\nbreak\ttab\rcr"),
                Expr::bool(true),
                Expr::bool(false),
                Expr::null(),
            ],
        ),
        (
            "unary",
            vec![
                !v("a"),
                -v("a"),
                -(-v("a")),
                !(!v("a")),
                -(v("a") + v("b")),
                !(v("a").and(v("b"))),
                -v("a") * v("b"),
                (!v("a")).and(v("b")),
            ],
        ),
        (
            "arithmetic",
            vec![
                v("a") + v("b"),
                v("a") - v("b"),
                v("a") * v("b"),
                v("a") / v("b"),
                v("a") % v("b"),
                v("a") + v("b") * v("c"),
                (v("a") + v("b")) * v("c"),
                v("a") - (v("b") - v("c")),
                (v("a") - v("b")) - v("c"),
                v("a") / (v("b") * v("c")),
                v("a") * v("b") % v("c"),
                v("price") * v("qty"),
                Expr::text("Total: ") + v("price") * v("qty"),
            ],
        ),
        (
            "comparison",
            vec![
                v("a").lt(v("b")),
                v("a").le(v("b")),
                v("a").gt(v("b")),
                v("a").ge(v("b")),
                (v("a") + 1).lt(v("b") * 2),
                v("a").lt(v("b")).lt(v("c")),
                v("a").lt(v("b").lt(v("c"))),
            ],
        ),
        (
            "equality",
            vec![
                v("a").eq_(v("b")),
                v("a").ne_(v("b")),
                v("a").lt(v("b")).eq_(v("c").gt(v("d"))),
                v("a").eq_(v("b")).ne_(v("c")),
                v("name").eq_(Expr::text("Ana")),
                v("x").ne_(Expr::null()),
                v("flag").eq_(Expr::bool(true)),
            ],
        ),
        (
            "logical",
            vec![
                v("a").and(v("b")),
                v("a").or(v("b")),
                v("a").or(v("b")).and(v("c")),
                v("a").and(v("b")).or(v("c")),
                v("a").or(v("b").and(v("c"))),
                v("a").and(v("b").or(v("c"))),
                v("a").eq_(v("b")).and(v("c").ne_(v("d"))),
                v("accepted").and(Expr::length(Expr::trim(v("name"))).ge(3)),
            ],
        ),
        (
            "ternary",
            vec![
                Expr::cond(v("a"), v("b"), v("c")),
                Expr::cond(v("a"), Expr::cond(v("b"), v("c"), v("d")), v("e")),
                Expr::cond(v("a"), v("b"), Expr::cond(v("c"), v("d"), v("e"))),
                Expr::cond(Expr::cond(v("a"), v("b"), v("c")), v("d"), v("e")),
                Expr::cond(v("a").or(v("b")), v("c") + 1, v("d") * 2),
                Expr::cond(
                    Expr::length(Expr::trim(v("name"))).ge(3),
                    Expr::text("ok"),
                    Expr::text("too short"),
                ),
            ],
        ),
        (
            "grouping",
            vec![
                (v("a") + v("b")).grouped(),
                (v("a") + v("b")).grouped() * v("c"),
                v("a") * (v("b") + v("c")).grouped(),
                v("a").grouped(),
                (v("a") + v("b")).grouped().grouped(),
                Expr::cond(v("a"), v("b"), v("c")).grouped() + 1,
                (Expr::from(StatePath::key("i")) + 1).grouped() + Expr::text(". "),
            ],
        ),
        (
            "field_access",
            vec![
                p(StatePath::key("user").field("name")),
                p(StatePath::key("user").field("address").field("city")),
                v("user").field("name").field("first"),
                Expr::contains(v("a"), v("b")).field("length"),
                (v("a") + v("b")).field("x"),
                int(1).field("x"),
                Expr::text("t").field("length"),
                p(StatePath::global("user").field("name")),
            ],
        ),
        (
            "index_access",
            vec![
                p(StatePath::key("items").index(0)),
                p(StatePath::key("items").index(3).field("name")),
                p(StatePath::key("grid").index(1).index(2)),
                v("items").at(v("i")),
                v("items").at(v("i") + 1),
                v("items").at(int(0)).field("id"),
                v("items").at(v("i")).field("name"),
                v("items").at(Expr::cond(v("a"), int(0), int(1))),
                p(StatePath::global("list").index(0)),
            ],
        ),
        (
            "roots",
            vec![
                v("total"),
                p(StatePath::global("token")),
                p(StatePath::params()),
                p(StatePath::params().field("name")),
                p(StatePath::result()),
                p(StatePath::result().field("qty")),
                p(StatePath::response().field("body").field("items")),
                p(StatePath::response().field("body").field("items").index(0)),
                p(StatePath::event().field("value")),
                p(StatePath::key("item").field("id")),
                p(StatePath::key("index")),
            ],
        ),
        (
            "functions",
            vec![
                Expr::length(v("tags")),
                Expr::length(Expr::trim(v("name"))),
                Expr::trim(v("name")),
                Expr::trim(Expr::text("  x ")),
                Expr::length(v("tags")) + Expr::text(" tags"),
                Expr::length(v("tags")).gt(0),
                Expr::contains(v("tags"), Expr::text("red")),
                Expr::contains(v("selected"), p(StatePath::key("item").field("id"))),
                Expr::contains_by(v("selected"), v("item"), "id"),
                Expr::contains_by(v("selected"), p(StatePath::key("item")), "it's"),
                !Expr::contains(v("tags"), v("x")),
                Expr::contains(v("tags"), v("x")).and(Expr::length(v("tags")).lt(3)),
            ],
        ),
    ]
}

/// Directory of the expression fixtures.
pub fn expressions_dir() -> String {
    format!("{}/expressions", crate::REACTIVE_FIXTURES_DIR)
}

fn rendered(exprs: &[Expr]) -> Value {
    Value::Array(
        exprs
            .iter()
            .map(|e| json!({"source": e.to_source(), "sexp": e.sexp()}))
            .collect(),
    )
}

#[test]
fn expression_fixtures_match_the_typed_expressions() {
    let update = std::env::var_os("UPDATE_REACTIVE_FIXTURES").is_some();
    let mut diverged = Vec::new();
    for (name, exprs) in categories() {
        let actual = rendered(&exprs);
        let path = format!("{}/{name}.json", expressions_dir());
        if update {
            std::fs::create_dir_all(expressions_dir()).unwrap();
            let mut text = serde_json::to_string_pretty(&actual).unwrap();
            text.push('\n');
            std::fs::write(&path, text).unwrap();
            continue;
        }
        let expected: Value = std::fs::read_to_string(&path)
            .map_err(|e| format!("{path}: {e}"))
            .and_then(|text| serde_json::from_str(&text).map_err(|e| format!("{path}: {e}")))
            .unwrap_or_else(|e| panic!("{e} (regenerate with UPDATE_REACTIVE_FIXTURES=1)"));
        if actual != expected {
            diverged.push(name);
        }
    }
    assert!(
        diverged.is_empty(),
        "expression fixtures out of date: {diverged:?} (regenerate with UPDATE_REACTIVE_FIXTURES=1 \
         and review the diff)"
    );
}

#[test]
fn no_orphan_expression_files() {
    let known: Vec<String> = categories()
        .iter()
        .map(|(name, _)| format!("{name}.json"))
        .collect();
    let mut orphans = Vec::new();
    if let Ok(entries) = std::fs::read_dir(expressions_dir()) {
        for entry in entries {
            let file = entry.unwrap().file_name().to_string_lossy().to_string();
            if !known.contains(&file) {
                orphans.push(file);
            }
        }
    }
    assert!(orphans.is_empty(), "files without a category: {orphans:?}");
}

#[test]
fn every_source_is_unique_per_category_and_non_empty() {
    for (name, exprs) in categories() {
        let mut seen = std::collections::HashSet::new();
        for e in &exprs {
            let source = e.to_source();
            assert!(!source.is_empty(), "{name}: empty source");
            assert!(seen.insert(source.clone()), "{name}: repeated `{source}`");
        }
    }
}
