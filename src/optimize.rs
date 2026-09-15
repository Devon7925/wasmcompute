//! Generated-function optimizations which preserve command order and results.
use crate::wasm::{Module, Node, Ty};
use std::collections::{BTreeMap, BTreeSet};
use wasmparser::Operator as O;

#[derive(Clone)]
enum ScoreExpr {
    Const(i32),
    Copy(String),
    Binary(String, String),
    Provider(serde_json::Value),
}
#[derive(Clone)]
struct ScoreEffect {
    dest: String,
    reads: Vec<String>,
    expr: ScoreExpr,
}
fn score_effect(line: &str) -> Option<ScoreEffect> {
    let t: Vec<_> = line.split_whitespace().collect();
    let key = |a: &str, b: &str| format!("{a} {b}");
    if t.starts_with(&["scoreboard", "players", "set"]) && t.len() == 6 && t[3].starts_with('#') {
        return Some(ScoreEffect {
            dest: key(t[3], t[4]),
            reads: vec![],
            expr: ScoreExpr::Const(t[5].parse().ok()?),
        });
    }
    if t.starts_with(&["scoreboard", "players", "operation"])
        && t.len() == 8
        && t[3].starts_with('#')
        && t[6].starts_with('#')
    {
        let dest = key(t[3], t[4]);
        let src = key(t[6], t[7]);
        if t[5] == "=" {
            return Some(ScoreEffect {
                dest,
                reads: vec![src.clone()],
                expr: ScoreExpr::Copy(src),
            });
        }
        if ["+=", "-=", "*=", "/=", "%=", "<", ">"].contains(&t[5]) {
            return Some(ScoreEffect {
                dest: dest.clone(),
                reads: vec![dest, src.clone()],
                expr: ScoreExpr::Binary(t[5].into(), src),
            });
        }
    }
    if let Some((head, tail)) = line.split_once(" run compute default integer ") {
        let h: Vec<_> = head.split_whitespace().collect();
        if h.len() != 6
            || h[..4] != ["execute", "store", "result", "score"]
            || !h[4].starts_with('#')
        {
            return None;
        }
        let p: serde_json::Value = serde_json::from_str(tail).ok()?;
        let mut reads = vec![];
        fn visit(p: &serde_json::Value, reads: &mut Vec<String>) -> Option<()> {
            if p.is_number() {
                return Some(());
            }
            let o = p.as_object()?;
            let typ = o.get("type")?.as_str()?;
            if typ == "score" {
                let target = o.get("target")?;
                if target.get("type")?.as_str()? != "fixed" {
                    return None;
                }
                reads.push(format!(
                    "{} {}",
                    target.get("name")?.as_str()?,
                    o.get("score")?.as_str()?
                ));
                return Some(());
            }
            if ![
                "add",
                "sub",
                "mul",
                "div",
                "mod",
                "floor_div",
                "floor_mod",
                "min",
                "max",
                "abs",
                "negate",
            ]
            .contains(&typ)
            {
                return None;
            }
            for (k, v) in o {
                if k == "type" {
                    continue;
                }
                if let Some(a) = v.as_array() {
                    for x in a {
                        visit(x, reads)?;
                    }
                } else {
                    visit(v, reads)?;
                }
            }
            Some(())
        }
        visit(&p, &mut reads)?;
        reads.sort();
        reads.dedup();
        return Some(ScoreEffect {
            dest: key(h[4], h[5]),
            reads,
            expr: ScoreExpr::Provider(p),
        });
    }
    None
}

/// Local value numbering and dead score writes. All unknown commands (including
/// calls, branches, macros and host effects) are barriers. Only private Wasm
/// code is rewritten; public functions and user-authored resources are excluded.
pub fn simplify_score_code(files: &mut BTreeMap<String, String>, ns: &str) {
    let prefix = format!("data/{ns}/function/f");
    let paths: Vec<_> = files
        .keys()
        .filter(|p| {
            p.strip_prefix(&prefix)
                .and_then(|s| s.split_once('/'))
                .is_some_and(|(n, _)| n.parse::<usize>().is_ok())
        })
        .cloned()
        .collect();
    let path_set: BTreeSet<_> = paths.iter().cloned().collect();
    for path in &paths {
        let mut values: BTreeMap<String, String> = BTreeMap::new();
        let mut serial = 0usize;
        let mut result = String::new();
        fn val(values: &mut BTreeMap<String, String>, serial: &mut usize, r: &str) -> String {
            values
                .entry(r.into())
                .or_insert_with(|| {
                    *serial += 1;
                    format!("v{serial}")
                })
                .clone()
        }
        for line in files[path].lines() {
            let Some(e) = score_effect(line) else {
                values.clear();
                result.push_str(line);
                result.push('\n');
                continue;
            };
            let mut identity = match &e.expr {
                ScoreExpr::Const(n) => format!("c{n}"),
                ScoreExpr::Copy(r) => val(&mut values, &mut serial, r),
                ScoreExpr::Binary(op, r) => {
                    let mut a = val(&mut values, &mut serial, &e.dest);
                    let mut b = val(&mut values, &mut serial, r);
                    if ["+=", "*=", "<", ">"].contains(&op.as_str()) && a > b {
                        std::mem::swap(&mut a, &mut b);
                    }
                    format!("({op} {a} {b})")
                }
                ScoreExpr::Provider(p) => {
                    let mut key = p.to_string();
                    for r in &e.reads {
                        key.push_str(&val(&mut values, &mut serial, r));
                        key.push(';');
                    }
                    key
                }
            };
            // Bound value-number expressions in heavily unrolled code.
            if identity.len() > 32768 {
                serial += 1;
                identity = format!("v{serial}");
            }
            let same = values
                .iter()
                .find(|(_, v)| **v == identity)
                .map(|(r, _)| r.clone());
            if let Some(source) = same {
                if source != e.dest {
                    result.push_str(&format!(
                        "scoreboard players operation {} = {source}\n",
                        e.dest
                    ));
                }
            } else {
                result.push_str(line);
                result.push('\n');
            }
            values.insert(e.dest, identity);
        }
        files.insert(path.clone(), result);
    }
    // A temporary which is never referenced outside its own pure definitions
    // cannot affect guest results. Repeat to remove dead dependency chains.
    for _ in 0..8 {
        let mut referenced = BTreeSet::new();
        for (path, text) in files.iter() {
            for line in text.lines() {
                let scanned = if path_set.contains(path) {
                    if let Some(e) = score_effect(line) {
                        e.reads
                            .into_iter()
                            .filter(|r| r != &e.dest)
                            .collect::<Vec<_>>()
                            .join(" ")
                    } else {
                        line.to_string()
                    }
                } else {
                    line.to_string()
                };
                for word in
                    scanned.split(|c: char| !(c.is_ascii_alphanumeric() || c == '#' || c == '_'))
                {
                    if word.starts_with("#f") {
                        referenced.insert(word.to_string());
                    }
                }
            }
        }
        let mut changed = false;
        for path in &paths {
            let mut overwritten = BTreeSet::new();
            let mut kept = vec![];
            for line in files[path].lines().rev() {
                if let Some(e) = score_effect(line) {
                    let holder = e.dest.split_whitespace().next().unwrap();
                    let temp = holder
                        .strip_prefix("#f")
                        .and_then(|s| s.split_once('t'))
                        .is_some_and(|(f, t)| {
                            f.parse::<usize>().is_ok() && t.parse::<usize>().is_ok()
                        });
                    if overwritten.contains(&e.dest) || (temp && !referenced.contains(holder)) {
                        changed = true;
                        continue;
                    }
                    overwritten.insert(e.dest);
                    for read in e.reads {
                        overwritten.remove(&read);
                    }
                } else {
                    overwritten.clear();
                }
                kept.push(line);
            }
            kept.reverse();
            files.insert(path.clone(), kept.join("\n") + "\n");
        }
        if !changed {
            break;
        }
    }
}

/// Recognize the complete four-byte-unrolled forward byte-copy loop, including
/// every local, branch, offset and signature. Function names are never trusted.
/// The overlapping dst > src case retains its byte-propagation semantics.
pub fn recognize_forward_copy(m: &mut Module<'_>) {
    let pattern = Module::parse(include_bytes!("patterns/forward-copy.wasm"))
        .expect("embedded forward-copy pattern is valid Wasm");
    let expected = &pattern.funcs[0];
    let body = format!("{:?}", expected.body);
    for f in &mut m.funcs {
        if f.import.is_some()
            || m.types[f.tid] != pattern.types[expected.tid]
            || f.locals != expected.locals
            || format!("{:?}", f.body) != body
        {
            continue;
        }
        let original = std::mem::take(&mut f.body);
        let get = |local_index| Node::Op(O::LocalGet { local_index });
        f.body = vec![
            get(0),
            get(1),
            Node::Op(O::I32LeU),
            get(0),
            get(1),
            Node::Op(O::I32Sub),
            get(2),
            Node::Op(O::I32GeU),
            Node::Op(O::I32Or),
            Node::Structured {
                kind: "if",
                params: vec![],
                results: vec![Ty::I32],
                body: vec![
                    get(0),
                    get(1),
                    get(2),
                    Node::Op(O::MemoryCopy {
                        dst_mem: 0,
                        src_mem: 0,
                    }),
                    get(0),
                ],
                other: original,
            },
        ];
        if m.accurate {
            let branch = f.body.pop().unwrap();
            for pointer in [0, 1] {
                f.body.extend([
                    get(pointer),
                    Node::Op(O::I64ExtendI32U),
                    get(2),
                    Node::Op(O::I64ExtendI32U),
                    Node::Op(O::I64Add),
                    Node::Op(O::MemorySize {
                        mem: 0,
                        mem_byte: 0,
                    }),
                    Node::Op(O::I64ExtendI32U),
                    Node::Op(O::I64Const { value: 16 }),
                    Node::Op(O::I64Shl),
                    Node::Op(O::I64LeU),
                    Node::Op(O::I32And),
                ]);
            }
            f.body.push(branch);
        }
    }
}

/// Inline only small, straight-line memory wrappers without private locals or
/// nested calls. Fresh caller locals hold arguments, preserving Wasm evaluation
/// order and allowing the ordinary data-flow pass to specialize constant bases.
/// The per-caller instruction budget bounds code growth, including nested blocks.
pub fn inline_memory_wrappers(m: &mut Module<'_>) {
    let candidates: BTreeMap<usize, (Vec<Ty>, Vec<Node<'_>>)> = m
        .funcs
        .iter()
        .enumerate()
        .filter_map(|(fid, f)| {
            if m.helper_start.is_some_and(|first| fid >= first) {
                return None;
            }
            let params = &m.signature(fid).0;
            if f.import.is_some() || !f.locals.is_empty() || f.body.len() > 16 || params.len() > 4 {
                return None;
            }
            let mut memory = false;
            for node in &f.body {
                let Node::Op(op) = node else {
                    return None;
                };
                if matches!(
                    op,
                    O::Call { .. }
                        | O::CallIndirect { .. }
                        | O::Return
                        | O::Br { .. }
                        | O::BrIf { .. }
                        | O::BrTable { .. }
                        | O::Unreachable
                ) {
                    return None;
                }
                memory |= crate::backend::memory_op(op).is_some();
            }
            memory.then(|| (fid, (params.clone(), f.body.clone())))
        })
        .collect();
    fn visit<'a>(
        nodes: &mut Vec<Node<'a>>,
        locals: &mut Vec<Ty>,
        parameters: usize,
        candidates: &BTreeMap<usize, (Vec<Ty>, Vec<Node<'a>>)>,
        budget: &mut usize,
    ) {
        let mut out = Vec::new();
        for mut node in std::mem::take(nodes) {
            match &mut node {
                Node::Structured { body, other, .. } => {
                    visit(body, locals, parameters, candidates, budget);
                    visit(other, locals, parameters, candidates, budget);
                }
                Node::Op(O::Call { function_index }) => {
                    if let Some((params, body)) = candidates.get(&(*function_index as usize)) {
                        let cost = params.len() + body.len();
                        if cost <= *budget {
                            *budget -= cost;
                            let base = (parameters + locals.len()) as u32;
                            locals.extend(params);
                            for i in (0..params.len()).rev() {
                                out.push(Node::Op(O::LocalSet {
                                    local_index: base + i as u32,
                                }));
                            }
                            for op in body {
                                let Node::Op(op) = op else { unreachable!() };
                                let mut op = op.clone();
                                match &mut op {
                                    O::LocalGet { local_index }
                                    | O::LocalSet { local_index }
                                    | O::LocalTee { local_index } => *local_index += base,
                                    _ => {}
                                }
                                out.push(Node::Op(op));
                            }
                            continue;
                        }
                    }
                }
                _ => {}
            }
            out.push(node);
        }
        *nodes = out;
    }
    for f in &mut m.funcs {
        visit(
            &mut f.body,
            &mut f.locals,
            m.types[f.tid].0.len(),
            &candidates,
            &mut 128,
        );
    }
}

fn target<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    line.split_once(marker)?.1.split_whitespace().next()
}

fn rewrite(text: &str, marker: &str, names: &BTreeMap<String, String>) -> String {
    text.lines()
        .map(|line| {
            if let Some((head, tail)) = line.split_once(marker) {
                if let Some(name) = tail.split_whitespace().next() {
                    if let Some(replacement) = names.get(name) {
                        return format!("{head}{marker}{replacement}{}\n", &tail[name.len()..]);
                    }
                }
            }
            format!("{line}\n")
        })
        .collect()
}

/// Give each Wasm function its own address-macro caches. Clone only the small
/// memory helper graph; shared arithmetic and four-way byte-lane macros remain
/// shared. No runtime selector, extra command, or changed register is introduced.
pub fn localize_memory(files: &mut BTreeMap<String, String>, ns: &str) {
    let base = format!("data/{ns}/function/");
    let marker = format!("function {ns}:");
    let helpers: BTreeSet<String> = [
        "word_read",
        "word_write",
        "load_word",
        "store_word",
        "load8",
        "store8",
        "load16",
        "store16",
        "load32",
        "store32",
        "load64",
        "store64",
        "load_f32",
        "store_f32",
        "load_f32_cached",
        "load_f32_uncached",
        "store_f32_cached",
        "store_f32_unaligned",
        "cache_f32",
        "cache_f32_macro",
        "flush_float",
    ]
    .iter()
    .map(|s| format!("rt/{s}"))
    .collect();
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in files.keys() {
        if let Some(rest) = path.strip_prefix(&base) {
            if let Some((fid, _)) = rest.split_once('/') {
                if fid.starts_with('f') && fid[1..].parse::<usize>().is_ok() {
                    groups.entry(fid.into()).or_default().push(path.clone());
                }
            }
        }
    }
    for (fid, paths) in groups {
        let mut todo = Vec::new();
        for path in &paths {
            todo.extend(
                files[path]
                    .lines()
                    .filter_map(|l| target(l, &marker))
                    .filter(|n| helpers.contains(*n))
                    .map(str::to_owned),
            );
        }
        let mut needed = BTreeSet::new();
        while let Some(name) = todo.pop() {
            if !needed.insert(name.clone()) {
                continue;
            }
            if let Some(body) = files.get(&format!("{base}{name}.mcfunction")) {
                todo.extend(
                    body.lines()
                        .filter_map(|l| target(l, &marker))
                        .filter(|n| helpers.contains(*n))
                        .map(str::to_owned),
                );
            }
        }
        let names: BTreeMap<String, String> = needed
            .iter()
            .map(|n| {
                (
                    n.clone(),
                    format!("local/{fid}/{}", n.strip_prefix("rt/").unwrap()),
                )
            })
            .collect();
        for name in needed {
            if let Some(body) = files.get(&format!("{base}{name}.mcfunction")) {
                let body = rewrite(body, &marker, &names);
                files.insert(format!("{base}{}.mcfunction", names[&name]), body);
            }
        }
        for path in paths {
            let body = rewrite(&files[&path], &marker, &names);
            files.insert(path, body);
        }
    }
}

/// Inline small, non-returning helpers only at ordinary, unconditional call
/// sites whose return value is ignored. Never flatten macros, conditional calls,
/// `return run`, or calls which capture a command result/success callback.
pub fn inline_command_helpers(files: &mut BTreeMap<String, String>, ns: &str) {
    let base = format!("data/{ns}/function/");
    let marker = format!("function {ns}:");
    for _ in 0..3 {
        let helpers: BTreeMap<String, String> = files
            .iter()
            .filter_map(|(path, text)| {
                let name = path.strip_prefix(&base)?.strip_suffix(".mcfunction")?;
                if !(name.starts_with("rt/") || name.starts_with("local/")) {
                    return None;
                }
                let lines: Vec<_> = text
                    .lines()
                    .filter(|l| !l.is_empty() && !l.starts_with('#'))
                    .collect();
                if lines.is_empty()
                    || lines.len() > 4
                    || lines
                        .iter()
                        .any(|l| l.starts_with('$') || l.split_whitespace().any(|w| w == "return"))
                {
                    return None;
                }
                Some((name.to_owned(), text.clone()))
            })
            .collect();
        let mut changes = 0;
        for (path, text) in files.iter_mut() {
            if !path.starts_with(&base) || !path.ends_with(".mcfunction") {
                continue;
            }
            let mut result = String::new();
            for line in text.lines() {
                if let Some(name) = line.strip_prefix(&marker) {
                    if let Some(body) = helpers.get(name) {
                        // A strict per-file growth cap keeps expansion bounded.
                        if result.len() + body.len() <= 64 * 1024 {
                            result.push_str(body);
                            changes += 1;
                            continue;
                        }
                    }
                }
                result.push_str(line);
                result.push('\n');
            }
            *text = result;
        }
        if changes == 0 {
            break;
        }
    }
}
