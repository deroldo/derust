//! Contextual checks over the serialized JSON of a screen (what the Stac runtime sees).
//!
//! The typed model (`Validate`) cannot know where a node ends up, so everything that depends on
//! context is checked here: declared keys, reserved names, the variables available at each
//! position (`response`, `event`, `result`, `item`/`index`), `bind` and secure keys.
//!
//! Rules marked `contrato:` come from the reactive-state contract of the Stac fork
//! (`docs/concepts/state_reference.mdx`). Only what the contract calls an error is an error;
//! reading an undeclared key is a warning (the runtime only logs it). The root of a response is
//! an opaque scope (the screen may be a fragment of a larger one) and so are navigation targets
//! and overlays; `params` is never reported.
//!
//! Messages carry the JSON path and key/property names, never values (a `secure` value must not
//! leak into logs).

use crate::sduix::stacx::common::{Path, ValidationIssue, ValidationReport};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::rc::Rc;

/// Deepest JSON nesting inspected; deeper trees are reported as an error instead of recursing.
pub const MAX_CHECK_DEPTH: usize = 256;

// contrato: nomes reservados do escopo de tela.
const RESERVED: [&str; 5] = ["global", "params", "result", "response", "event"];

/// Action properties that hold plain data: bindings are resolved at any depth and nothing below
/// them is a widget or an action.
const DATA_KEYS: [&str; 9] = [
    "values",
    "value",
    "result",
    "params",
    "body",
    "queryParameters",
    "headers",
    "initial",
    "arguments",
];

/// Widgets that accept `bind`, with the value property `bind` overrides.
// contrato: bind
const BIND_TABLE: [(&str, &str); 7] = [
    ("checkBox", "value"),
    ("switch", "value"),
    ("slider", "value"),
    ("radioGroup", "groupValue"),
    ("textField", "initialValue"),
    ("textFormField", "initialValue"),
    ("dropdownMenu", "initialSelection"),
];

const DEFAULT_ITEM: &str = "item";
const DEFAULT_INDEX: &str = "index";

/// Checks the context dependent rules of a screen (a widget or an action) given as JSON.
///
/// Never panics: values of unexpected types become issues (or are ignored when the typed layer
/// already owns the rule).
pub fn check_screen(json: &Value) -> ValidationReport {
    let mut ctx = Ctx {
        secure: collect_secure(json),
        report: ValidationReport::new(),
        depth_reported: false,
    };
    let scope = Scope {
        keys: None,
        response: false,
        event: false,
        result: false,
        levels: Rc::new(Vec::new()),
    };
    walk(json, &scope, &Path::root("$"), &mut ctx, 0);
    ctx.report
}

struct Ctx {
    /// Keys declared `secure` in any `globalState` of the response.
    secure: HashSet<String>,
    report: ValidationReport,
    depth_reported: bool,
}

impl Ctx {
    fn error(&mut self, path: &Path<'_>, message: impl Into<String>) {
        self.report.errors.push(ValidationIssue::new(path, message));
    }

    fn warning(&mut self, path: &Path<'_>, message: impl Into<String>) {
        self.report
            .warnings
            .push(ValidationIssue::new(path, message));
    }

    /// True (after reporting once) when `depth` is beyond the limit.
    fn too_deep(&mut self, path: &Path<'_>, depth: usize) -> bool {
        if depth <= MAX_CHECK_DEPTH {
            return false;
        }
        if !self.depth_reported {
            self.depth_reported = true;
            self.error(
                path,
                format!("screen is nested deeper than {MAX_CHECK_DEPTH} levels"),
            );
        }
        true
    }
}

#[derive(Clone)]
struct Level {
    item: String,
    index: String,
}

#[derive(Clone)]
struct Scope {
    /// Declared screen keys; `None` is the opaque scope (anything goes).
    keys: Option<Rc<HashSet<String>>>,
    response: bool,
    event: bool,
    result: bool,
    /// Active list templates, outermost first.
    levels: Rc<Vec<Level>>,
}

impl Scope {
    fn opaque(&self) -> Scope {
        Scope {
            keys: None,
            ..self.clone()
        }
    }

    fn is_local(&self, name: &str) -> bool {
        self.levels
            .iter()
            .any(|level| level.item == name || level.index == name)
    }
}

fn collect_secure(json: &Value) -> HashSet<String> {
    let mut found = HashSet::new();
    let mut stack = vec![(json, 0usize)];
    while let Some((value, depth)) = stack.pop() {
        if depth > MAX_CHECK_DEPTH {
            continue;
        }
        match value {
            Value::Object(map) => {
                if let Some(Value::Object(global)) = map.get("globalState") {
                    for (key, entry) in global {
                        if let Value::Object(entry) = entry {
                            if is_declaration(entry)
                                && entry.get("secure") == Some(&Value::Bool(true))
                            {
                                found.insert(key.clone());
                            }
                        }
                    }
                }
                stack.extend(map.values().map(|v| (v, depth + 1)));
            }
            Value::Array(items) => stack.extend(items.iter().map(|v| (v, depth + 1))),
            _ => {}
        }
    }
    found
}

/// `{initial, persist, secure}` object (a declared global entry).
fn is_declaration(map: &Map<String, Value>) -> bool {
    !map.is_empty()
        && map
            .keys()
            .all(|k| matches!(k.as_str(), "initial" | "persist" | "secure"))
}

fn as_binding(map: &Map<String, Value>) -> Option<(&str, &str)> {
    if map.len() != 1 {
        return None;
    }
    let (key, value) = map.iter().next()?;
    match (key.as_str(), value) {
        ("$state", Value::String(source)) => Some(("$state", source)),
        ("$expr", Value::String(source)) => Some(("$expr", source)),
        _ => None,
    }
}

fn walk(value: &Value, scope: &Scope, path: &Path<'_>, ctx: &mut Ctx, depth: usize) {
    if ctx.too_deep(path, depth) {
        return;
    }
    match value {
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                walk(item, scope, &path.index(index), ctx, depth + 1);
            }
        }
        Value::Object(map) => {
            if let Some((kind, source)) = as_binding(map) {
                check_binding(kind, source, scope, path, ctx);
            } else if let Some(Value::String(action_type)) = map.get("actionType") {
                check_action(map, action_type, scope, path, ctx, depth);
            } else if let Some(Value::String(widget_type)) = map.get("type") {
                check_widget(map, widget_type, scope, path, ctx, depth);
            } else {
                for (key, child) in map {
                    walk(child, scope, &path.key(key), ctx, depth + 1);
                }
            }
        }
        _ => {}
    }
}

/// Plain data: only bindings matter.
fn walk_data(value: &Value, scope: &Scope, path: &Path<'_>, ctx: &mut Ctx, depth: usize) {
    if ctx.too_deep(path, depth) {
        return;
    }
    match value {
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                walk_data(item, scope, &path.index(index), ctx, depth + 1);
            }
        }
        Value::Object(map) => {
            if let Some((kind, source)) = as_binding(map) {
                check_binding(kind, source, scope, path, ctx);
            } else {
                for (key, child) in map {
                    walk_data(child, scope, &path.key(key), ctx, depth + 1);
                }
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------------- reads

fn check_binding(kind: &str, source: &str, scope: &Scope, path: &Path<'_>, ctx: &mut Ctx) {
    if kind == "$state" {
        if let Some(name) = path_root(source) {
            check_read(name, scope, path, ctx);
        }
    } else {
        for name in expr_names(source) {
            check_read(&name, scope, path, ctx);
        }
    }
}

/// Root name of a `$state` path (`user.name` -> `user`, `items[0]` -> `items`).
fn path_root(source: &str) -> Option<&str> {
    let source = source.trim();
    let end = source.find(['.', '[']).unwrap_or(source.len());
    let root = &source[..end];
    is_ident(root).then_some(root)
}

fn is_ident(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Names read by an expression: identifiers that are not inside a text literal, not a field
/// (`.name`), not a function (`name(`) and not `true`/`false`/`null`. Not a parser.
fn expr_names(source: &str) -> Vec<String> {
    let chars: Vec<char> = source.chars().collect();
    let mut names = Vec::new();
    let mut i = 0;
    let mut after_dot = false;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '\'' || c == '"' {
            i += 1;
            while i < chars.len() && chars[i] != c {
                // `\x` -> x: the escaped character never closes the literal.
                i += if chars[i] == '\\' { 2 } else { 1 };
            }
            i += 1;
            after_dot = false;
        } else if c.is_ascii_digit() {
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            after_dot = false;
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let mut next = i;
            while next < chars.len() && chars[next].is_whitespace() {
                next += 1;
            }
            let is_call = chars.get(next) == Some(&'(');
            if !after_dot && !is_call && !matches!(word.as_str(), "true" | "false" | "null") {
                names.push(word);
            }
            after_dot = false;
        } else {
            after_dot = c == '.';
            i += 1;
        }
    }
    names
}

fn check_read(name: &str, scope: &Scope, path: &Path<'_>, ctx: &mut Ctx) {
    let Some(keys) = &scope.keys else {
        return; // opaque scope: nothing is accused
    };
    match name {
        "global" | "params" => {}
        // contrato: `response` só em onSuccess/onError/onFinally
        "response" if !scope.response => ctx.error(
            path,
            "`response` is only available inside onSuccess, onError and onFinally",
        ),
        // contrato: `event` só em onChanged/onSelected
        "event" if !scope.event => ctx.error(
            path,
            "`event` is only available inside onChanged and onSelected",
        ),
        // contrato: `result` só em onResult
        "result" if !scope.result => ctx.error(path, "`result` is only available inside onResult"),
        "response" | "event" | "result" => {}
        _ if scope.is_local(name) || keys.contains(name) => {}
        // contrato: `item`/`index` só dentro de itemTemplate
        DEFAULT_ITEM | DEFAULT_INDEX => ctx.error(
            path,
            format!("`{name}` is only available inside an itemTemplate"),
        ),
        // contrato: leitura de chave não declarada só é avisada pelo runtime (D1)
        _ => ctx.warning(path, format!("reads undeclared state key `{name}`")),
    }
}

// ---------------------------------------------------------------------------------- writes

fn check_write(key: &str, scope: &Scope, path: &Path<'_>, ctx: &mut Ctx) {
    let key = key.trim();
    if key.starts_with("global.") {
        return; // global keys may be declared on another screen
    }
    let end = key.find(['.', '[']).unwrap_or(key.len());
    let root = &key[..end];
    if root.is_empty() {
        return; // empty keys are a typed-layer error
    }
    // contrato: nomes reservados
    if RESERVED.contains(&root) {
        ctx.error(path, format!("cannot write to reserved name `{root}`"));
    // contrato: `item`/`index` (e `as`/`indexAs`) são variáveis locais somente leitura
    } else if scope.is_local(root) {
        ctx.error(
            path,
            format!("cannot write to template variable `{root}` (read only)"),
        );
    // contrato: escrita em chave não declarada é erro
    } else if let Some(keys) = &scope.keys {
        if !keys.contains(root) {
            ctx.error(path, format!("writes to undeclared state key `{root}`"));
        }
    }
}

// ---------------------------------------------------------------------------------- actions

fn check_action(
    map: &Map<String, Value>,
    action_type: &str,
    scope: &Scope,
    path: &Path<'_>,
    ctx: &mut Ctx,
    depth: usize,
) {
    match action_type {
        "setState" => {
            if let Some(Value::Object(values)) = map.get("values") {
                let values_path = path.key("values");
                for key in values.keys() {
                    check_write(key, scope, &values_path.key(key), ctx);
                }
            }
        }
        "incrementState" | "decrementState" | "toggleState" | "appendState" | "removeFromState"
        | "toggleItem" => {
            if let Some(Value::String(key)) = map.get("key") {
                check_write(key, scope, &path.key("key"), ctx);
            }
        }
        // `resetState` ignores undeclared keys at runtime.
        "networkRequest" => {
            if let Some(Value::Array(keys)) = map.get("bodyFromState") {
                for (index, key) in keys.iter().enumerate() {
                    // contrato: bodyFromState não envia chaves secure
                    if let Value::String(key) = key {
                        if let Some(name) = key.trim().strip_prefix("global.") {
                            if ctx.secure.contains(name) {
                                ctx.error(
                                    &path.key("bodyFromState").index(index),
                                    format!("bodyFromState references secure global key `{name}`"),
                                );
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    for (key, value) in map {
        let child_path = path.key(key);
        let key = key.as_str();
        if key == "actionType" {
            continue;
        }
        if DATA_KEYS.contains(&key) {
            walk_data(value, scope, &child_path, ctx, depth + 1);
            continue;
        }
        let opaque_target = match action_type {
            "navigate" => key == "widgetJson",
            "showDialog" | "showModalBottomSheet" => key == "widget",
            "showSnackBar" => key == "content" || key == "action",
            _ => false,
        };
        if opaque_target {
            walk(value, &scope.opaque(), &child_path, ctx, depth + 1);
            continue;
        }
        let mut inner = scope.clone();
        match key {
            "onSuccess" | "onError" | "onFinally" => inner.response = true,
            "onResult" => inner.result = true,
            _ => {}
        }
        walk(value, &inner, &child_path, ctx, depth + 1);
    }
}

// ---------------------------------------------------------------------------------- widgets

fn check_widget(
    map: &Map<String, Value>,
    widget_type: &str,
    outer: &Scope,
    path: &Path<'_>,
    ctx: &mut Ctx,
    depth: usize,
) {
    let opens_scope =
        map.contains_key("state") || map.contains_key("globalState") || map.contains_key("onInit");
    let mut scope = outer.clone();
    if opens_scope {
        let mut keys = HashSet::new();
        if let Some(Value::Object(state)) = map.get("state") {
            keys.extend(state.keys().cloned());
        }
        scope.keys = Some(Rc::new(keys));
    }

    check_state_block(map, path, ctx);
    check_bind(map, widget_type, path, ctx);

    for (key, value) in map {
        let child_path = path.key(key);
        match key.as_str() {
            "type" | "bind" => {}
            "state" | "globalState" => {
                if let Value::Object(entries) = value {
                    for (entry_key, entry) in entries {
                        let entry_path = child_path.key(entry_key);
                        walk_data(entry, &scope, &entry_path, ctx, depth + 2);
                    }
                }
            }
            "itemTemplate" => check_template(map, value, &scope, &child_path, ctx, depth + 1),
            "onChanged" | "onSelected" => {
                let mut inner = scope.clone();
                inner.event = true;
                walk(value, &inner, &child_path, ctx, depth + 1);
            }
            _ => walk(value, &scope, &child_path, ctx, depth + 1),
        }
    }
}

fn check_state_block(map: &Map<String, Value>, path: &Path<'_>, ctx: &mut Ctx) {
    // contrato: state / globalState são objetos
    match map.get("state") {
        None => {}
        Some(Value::Object(state)) => {
            let state_path = path.key("state");
            for (key, entry) in state {
                // contrato: nomes reservados não podem ser chaves de estado
                if RESERVED.contains(&key.as_str()) {
                    ctx.error(
                        &state_path.key(key),
                        format!("`{key}` is a reserved name and cannot be a state key"),
                    );
                }
                // contrato: persist/secure só existem em globalState
                if let Value::Object(entry) = entry {
                    if is_declaration(entry)
                        && (entry.contains_key("persist") || entry.contains_key("secure"))
                    {
                        ctx.error(
                            &state_path.key(key),
                            format!(
                                "state key `{key}` uses persist/secure, which only exist in globalState"
                            ),
                        );
                    }
                }
            }
        }
        Some(_) => ctx.error(&path.key("state"), "`state` must be an object"),
    }
    match map.get("globalState") {
        None => {}
        Some(Value::Object(global)) => {
            let global_path = path.key("globalState");
            for (key, entry) in global {
                if let Value::Object(entry) = entry {
                    // contrato: secure exige persist
                    if is_declaration(entry)
                        && entry.get("secure") == Some(&Value::Bool(true))
                        && entry.get("persist") == Some(&Value::Bool(false))
                    {
                        ctx.error(
                            &global_path.key(key),
                            format!("global key `{key}` is secure and cannot have persist false"),
                        );
                    }
                }
            }
        }
        Some(_) => ctx.error(&path.key("globalState"), "`globalState` must be an object"),
    }
}

fn check_bind(map: &Map<String, Value>, widget_type: &str, path: &Path<'_>, ctx: &mut Ctx) {
    let Some(bind) = map.get("bind") else {
        return;
    };
    let bind_path = path.key("bind");
    // contrato: bind
    if widget_type == "radio" {
        ctx.error(
            &bind_path,
            "`radio` does not support bind: bind the enclosing radioGroup",
        );
        return;
    }
    let Some((_, value_property)) = BIND_TABLE.iter().find(|(name, _)| *name == widget_type) else {
        ctx.error(
            &bind_path,
            format!("widget `{widget_type}` does not support bind"),
        );
        return;
    };
    match bind {
        Value::String(key) if !key.trim().is_empty() => {}
        _ => ctx.error(&bind_path, "`bind` must be a non-empty text"),
    }
    if map.contains_key(*value_property) {
        ctx.error(
            &bind_path,
            format!("`bind` overrides `{value_property}`: remove the property"),
        );
    }
}

fn name_or(map: &Map<String, Value>, key: &str, default: &str) -> String {
    match map.get(key) {
        Some(Value::String(name)) if !name.is_empty() => name.clone(),
        _ => default.to_string(),
    }
}

fn check_template(
    list: &Map<String, Value>,
    template: &Value,
    scope: &Scope,
    path: &Path<'_>,
    ctx: &mut Ctx,
    depth: usize,
) {
    let item = name_or(list, "as", DEFAULT_ITEM);
    let index = name_or(list, "indexAs", DEFAULT_INDEX);

    for name in [&item, &index] {
        if scope.is_local(name) {
            ctx.warning(
                path,
                format!("template variable `{name}` shadows a variable of an outer template"),
            );
        }
    }
    if let Value::Object(root) = template {
        if root.contains_key("state") {
            ctx.warning(
                path,
                "the template root declares its own `state`, hiding the keys of the screen (including aggregates)",
            );
        }
    }

    let mut inner = scope.clone();
    let mut levels = (*scope.levels).clone();
    levels.push(Level { item, index });
    inner.levels = Rc::new(levels);
    walk(template, &inner, path, ctx, depth);
}
