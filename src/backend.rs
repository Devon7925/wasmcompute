use crate::wasm::{numeric_name, Module, Node, Num, Ty};
use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value as Json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use wasmparser::Operator as O;

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Const(Ty, Num),
    Reg(Ty, String),
    Expr(Ty, String, Vec<Val>),
    Read {
        word: u64,
        shift: u32,
        width: u32,
        signed: bool,
    },
}
impl Val {
    pub fn ty(&self) -> Ty {
        match self {
            Self::Const(t, _) | Self::Reg(t, _) | Self::Expr(t, ..) => *t,
            Self::Read { .. } => Ty::I32,
        }
    }
    pub fn reg(t: Ty, s: impl Into<String>) -> Self {
        Self::Reg(t, s.into())
    }
    pub fn int(n: i64) -> Self {
        Self::Const(Ty::I32, Num::I(n as i32 as i64))
    }
    pub fn name(&self) -> &str {
        match self {
            Self::Reg(_, s) => s,
            _ => panic!("expected materialized register"),
        }
    }
    fn depends(&self, name: &str) -> bool {
        match self {
            Self::Reg(_, n) => n == name,
            Self::Expr(_, _, args) => args.iter().any(|a| a.depends(name)),
            _ => false,
        }
    }
    fn reads_memory(&self) -> bool {
        match self {
            Self::Read { .. } => true,
            Self::Expr(_, _, a) => a.iter().any(Self::reads_memory),
            _ => false,
        }
    }
    fn depends_prefix(&self, prefix: &str) -> bool {
        match self {
            Self::Reg(_, n) => n.starts_with(prefix),
            Self::Expr(_, _, args) => args.iter().any(|a| a.depends_prefix(prefix)),
            _ => false,
        }
    }
}
#[derive(Clone)]
struct Label {
    target: String,
    height: usize,
    types: Vec<Ty>,
}
pub type Bindings = BTreeMap<String, Json>;
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CopyRegion {
    pub src: u32,
    pub dst: u32,
    pub len: u32,
}
pub struct Pack {
    pub files: BTreeMap<String, String>,
    pub warnings: BTreeSet<String>,
}
impl Pack {
    pub fn command_count(&self) -> usize {
        self.files
            .iter()
            .filter(|(p, _)| p.ends_with(".mcfunction"))
            .map(|(_, s)| {
                s.lines()
                    .filter(|l| !l.is_empty() && !l.starts_with('#'))
                    .count()
            })
            .sum()
    }
    pub fn write(&self, root: &Path) -> Result<()> {
        if root.exists() && root.read_dir()?.next().is_some() {
            ensure!(
                root.join("wasmcompute.json").is_file(),
                "refusing to overwrite a non-wasmcompute directory"
            );
        }
        for (p, s) in &self.files {
            let dest = root.join(p);
            std::fs::create_dir_all(dest.parent().unwrap())?;
            std::fs::write(dest, s)?;
        }
        Ok(())
    }
}
pub struct Backend<'m, 'a> {
    pub m: &'m Module<'a>,
    pub ns: String,
    pub obj: String,
    pub storage: String,
    pub mem: String,
    pub files: BTreeMap<String, String>,
    pub constants: BTreeMap<String, i32>,
    pub warnings: BTreeSet<String>,
    pub lines: Vec<String>,
    pub required: BTreeSet<String>,
    bindings: Bindings,
    opt: bool,
    fid: usize,
    temp: usize,
    serial: usize,
    locals: Vec<Ty>,
    regs: BTreeMap<usize, BTreeMap<String, Ty>>,
    reachable: BTreeSet<usize>,
    recursive: BTreeSet<usize>,
    ranges: BTreeMap<String, (i64, i64)>,
    fallthrough_ranges: BTreeMap<String, Vec<(String, (i64, i64))>>,
    memory_values: Vec<(Val, Ty, u32, bool, Val)>,
    local_constants: BTreeMap<u32, Val>,
    buffer_words: bool,
    pub score_memory: bool,
    pub copy_profile: Vec<CopyRegion>,
}
pub fn compile(m: &Module<'_>, ns: &str, opt: bool, bindings: Bindings) -> Result<Pack> {
    compile_with_options(m, ns, opt, bindings, false)
}
pub fn compile_with_options(
    m: &Module<'_>,
    ns: &str,
    opt: bool,
    bindings: Bindings,
    buffer_words: bool,
) -> Result<Pack> {
    compile_with_memory(m, ns, opt, bindings, buffer_words, true)
}
pub fn compile_with_memory(
    m: &Module<'_>,
    ns: &str,
    opt: bool,
    bindings: Bindings,
    buffer_words: bool,
    score_memory: bool,
) -> Result<Pack> {
    compile_with_profile(m, ns, opt, bindings, buffer_words, score_memory, vec![])
}
pub fn compile_with_profile(
    m: &Module<'_>,
    ns: &str,
    opt: bool,
    bindings: Bindings,
    buffer_words: bool,
    score_memory: bool,
    mut copy_profile: Vec<CopyRegion>,
) -> Result<Pack> {
    let mut bindings = bindings;
    for f in &m.funcs {
        if let Some((module, name)) = &f.import {
            if module == "minecraft" {
                if let Some(encoded) = name.strip_prefix("wc1_") {
                    ensure!(
                        encoded.len() % 2 == 0 && encoded.bytes().all(|b| b.is_ascii_hexdigit()),
                        "invalid SDK import encoding"
                    );
                    let bytes = (0..encoded.len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&encoded[i..i + 2], 16))
                        .collect::<std::result::Result<Vec<_>, _>>()?;
                    let binding: Json =
                        serde_json::from_slice(&bytes).context("invalid SDK command metadata")?;
                    bindings.insert(format!("{module}.{name}"), binding);
                }
            }
        }
    }
    copy_profile.sort();
    copy_profile.dedup();
    ensure!(
        copy_profile.len() <= 256
            && copy_profile.iter().map(|r| r.len as u64).sum::<u64>() <= 262144,
        "copy profile exceeds code-size budget"
    );
    for r in &copy_profile {
        ensure!(
            r.src > r.dst
                && r.src % 4 == 0
                && r.dst % 4 == 0
                && r.len > 0
                && r.len <= 4096
                && r.src as u64 + r.len as u64 <= i32::MAX as u64,
            "copy profile requires aligned forward ranges, 1..4096 bytes, in signed address range"
        );
    }
    let mut optimized;
    let m = if opt {
        optimized = m.clone();
        crate::optimize::recognize_forward_copy(&mut optimized);
        crate::optimize::inline_memory_wrappers(&mut optimized);
        &optimized
    } else {
        m
    };
    ensure!(
        !ns.is_empty()
            && ns.len() <= 12
            && ns
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
        "namespace must contain 1-12 lowercase letters, digits or underscores"
    );
    let mut reachable = BTreeSet::new();
    let mut todo = m.exports.values().copied().collect::<Vec<_>>();
    todo.extend(m.start);
    while let Some(f) = todo.pop() {
        if reachable.insert(f) {
            todo.extend(m.callees(f));
        }
    }
    let mut recursive = BTreeSet::new();
    for &f in &reachable {
        let mut seen = BTreeSet::new();
        let mut todo = m.callees(f).into_iter().collect::<Vec<_>>();
        while let Some(g) = todo.pop() {
            if g == f {
                recursive.insert(f);
                break;
            }
            if seen.insert(g) {
                todo.extend(m.callees(g));
            }
        }
    }
    let mut b = Backend {
        m,
        ns: ns.into(),
        obj: ns.into(),
        storage: format!("{ns}:vm"),
        mem: format!("{ns}:mem"),
        files: Default::default(),
        constants: Default::default(),
        warnings: Default::default(),
        lines: vec![],
        required: Default::default(),
        bindings,
        opt,
        fid: 0,
        temp: 0,
        serial: 0,
        locals: vec![],
        regs: Default::default(),
        reachable,
        recursive,
        ranges: Default::default(),
        fallthrough_ranges: Default::default(),
        memory_values: vec![],
        local_constants: Default::default(),
        buffer_words,
        score_memory,
        copy_profile,
    };
    if !m.accurate {
        let mut todo: Vec<_> = m.funcs.iter().flat_map(|f| f.body.iter()).collect();
        let mut approximated = m
            .globals
            .iter()
            .any(|(t, n)| t.float() && !(n.f() as f32).is_finite());
        while let Some(node) = todo.pop() {
            match node {
                Node::Structured { body, other, .. } => {
                    todo.extend(body);
                    todo.extend(other);
                }
                Node::Op(O::F32Const { value }) => {
                    approximated |= !f32::from_bits(value.bits()).is_finite()
                }
                Node::Op(O::F64Const { value }) => {
                    approximated |= !(f64::from_bits(value.bits()) as f32).is_finite()
                }
                _ => {}
            }
        }
        if approximated {
            b.warnings.insert("Fast mode saturates infinite/out-of-f32-range constants and maps NaN constants to zero; use Accurate for IEEE semantics".into());
        }
    }
    b.build()?;
    Ok(Pack {
        files: b.files,
        warnings: b.warnings,
    })
}
impl<'m, 'a> Backend<'m, 'a> {
    /// Large initial images live in compact int arrays. Scoreboard words form
    /// a lazy cache and writable overlay, including explicit zero writes.
    pub fn lazy_initial_memory(&self) -> bool {
        self.score_memory
            && self
                .m
                .data
                .iter()
                .map(|(_, data)| data.len())
                .sum::<usize>()
                >= 131072
    }
    pub fn word_provider(&self, word: u64) -> Json {
        if self.score_memory {
            json!({"type":"score","target":{"type":"fixed","name":format!("#m{word}")},"score":format!("{}_mem",self.ns)})
        } else {
            json!({"type":"storage","storage":self.mem,"path":format!("w.a{word}")})
        }
    }
    /// Store a canonical word; the caller supplies `$` for macro lines.
    pub fn write_word(&self, word: &str, register: &str) -> String {
        if self.score_memory {
            format!(
                "scoreboard players operation #m{word} {}_mem = {}",
                self.ns,
                self.score(register)
            )
        } else {
            format!(
                "execute store result storage {} w.a{word} int 1 run scoreboard players get {}",
                self.mem,
                self.score(register)
            )
        }
    }
    pub fn word_buffer(&self) -> bool {
        self.buffer_words && self.opt && !self.float_cache()
    }
    pub fn float_cache(&self) -> bool {
        // Only memory stores can create dirty float words. Floating command
        // arguments, locals and globals do not require integer memory accesses
        // to probe or flush the cache. Keep this module-wide when stores exist:
        // memory aliases and cross-function calls can observe those words.
        fn stores_float(nodes: &[Node<'_>]) -> bool {
            nodes.iter().any(|node| match node {
                Node::Op(O::F32Store { .. } | O::F64Store { .. }) => true,
                Node::Structured { body, other, .. } => stores_float(body) || stores_float(other),
                _ => false,
            })
        }
        self.opt && self.m.funcs.iter().any(|f| stores_float(&f.body))
    }
    pub fn emit(&mut self, s: impl Into<String>) {
        self.lines.push(s.into());
    }
    pub fn function(&mut self, name: &str, lines: Vec<String>) {
        self.files.insert(
            format!("data/{}/function/{name}.mcfunction", self.ns),
            lines.join("\n") + "\n",
        );
    }
    pub fn score(&self, n: &str) -> String {
        format!("#{n} {}", self.obj)
    }
    pub fn set(&mut self, n: &str, v: i64) {
        self.emit(format!(
            "scoreboard players set {} {}",
            self.score(n),
            v as i32
        ));
    }
    pub fn cp(&mut self, d: &str, s: &str) {
        if d != s {
            self.emit(format!(
                "scoreboard players operation {} = {}",
                self.score(d),
                self.score(s)
            ));
        }
    }
    pub fn operation(&mut self, d: &str, op: &str, s: &str) {
        self.emit(format!(
            "scoreboard players operation {} {op} {}",
            self.score(d),
            self.score(s)
        ));
    }
    pub fn c(&mut self, n: i64) -> String {
        let n = n as i32;
        let k = format!("c{}", n.to_string().replace('-', 'n'.to_string().as_str()));
        self.constants.insert(k.clone(), n);
        k
    }
    fn reg(&mut self, t: Ty, n: impl Into<String>) -> Val {
        let n = n.into();
        self.regs.entry(self.fid).or_default().insert(n.clone(), t);
        Val::reg(t, n)
    }
    fn tmp(&mut self, t: Ty) -> Val {
        self.temp += 1;
        self.reg(t, format!("f{}t{}", self.fid, self.temp))
    }
    fn fresh(&mut self, tag: &str) -> String {
        self.serial += 1;
        format!("f{}/{tag}{}", self.fid, self.serial)
    }
    pub fn rt(&mut self, n: &str) {
        self.required.insert(n.into());
        self.emit(format!("function {}:rt/{n}", self.ns));
    }
    pub fn provider(&self, v: &Val) -> Result<Json> {
        Ok(match v {
            Val::Read {
                word,
                shift,
                width,
                signed,
            } => {
                let mut p = self.word_provider(*word);
                if *width < 32 {
                    p = json!({"type":"floor_mod","left":{"type":"floor_div","left":p,"right":1u32<<shift},"right":1u32<<width});
                    if *signed {
                        p = json!({"type":"sub","left":{"type":"floor_mod","left":{"type":"add","inputs":[p,1u32<<(width-1)]},"right":1u32<<width},"right":1u32<<(width-1)});
                    }
                }
                p
            }
            Val::Const(t, n) => {
                if t.float() {
                    json!(fast_float_constant(n.f()))
                } else {
                    json!(n.i() as i32)
                }
            }
            Val::Reg(t, n) => {
                if t.float() {
                    json!({"type":"storage","storage":self.storage,"path":format!("r.{n}")})
                } else {
                    json!({"type":"score","target":{"type":"fixed","name":format!("#{n}")},"score":self.obj})
                }
            }
            Val::Expr(_, op, args) => {
                let ps = args
                    .iter()
                    .map(|a| self.provider(a))
                    .collect::<Result<Vec<_>>>()?;
                let op = match op.as_str() {
                    "neg" => "negate",
                    "trunc" => "truncate",
                    "nearest" => "round",
                    s => s,
                };
                match op {
                    "add" | "mul" | "min" | "max" => json!({"type":op,"inputs":ps}),
                    "sub" | "div" | "mod" => json!({"type":op,"left":ps[0],"right":ps[1]}),
                    "from_int" | "from_float" | "abs" | "negate" | "ceil" | "floor"
                    | "truncate" | "round" | "sqrt" | "sin" | "cos" => {
                        json!({"type":op,"input":ps[0]})
                    }
                    _ => bail!("unsupported provider {op}"),
                }
            }
        })
    }
    fn scalar(&mut self, v: &Val) -> Result<String> {
        match v {
            Val::Const(_, n) => Ok(self.c(n.i())),
            Val::Reg(_, n) => Ok(n.clone()),
            _ => Ok(self.materialize(v, None)?.name().into()),
        }
    }
    pub fn materialize(&mut self, v: &Val, d: Option<Val>) -> Result<Val> {
        if d.is_none() && matches!(v, Val::Reg(..)) {
            return Ok(v.clone());
        }
        let dest = d.unwrap_or_else(|| self.tmp(v.ty()));
        if &dest == v {
            return Ok(dest);
        }
        let name = dest.name().to_string();
        let range = self.interval(v);
        if self.opt && v.ty() == Ty::I32 && matches!(v, Val::Expr(..) | Val::Read { .. }) {
            if let Some(p) = self.integer_provider(v) {
                if let Some(n) = p.as_i64() {
                    self.set(&name, n);
                } else {
                    self.emit(format!(
                        "execute store result score {} run compute default integer {p}",
                        self.score(&name)
                    ));
                }
                self.ranges.insert(name, range);
                return Ok(dest);
            }
        }
        if v.ty().float() {
            match v {
                Val::Reg(_, s) => self.emit(format!(
                    "data modify storage {} r.{name} set from storage {} r.{s}",
                    self.storage, self.storage
                )),
                Val::Const(_, n) => {
                    self.emit(format!(
                        "data modify storage {} r.{name} set value {}f",
                        self.storage,
                        fast_float_constant(n.f()) as f32
                    ));
                }
                _ => self.emit(format!(
                    "data modify storage {} r.{name} set compute default float {}",
                    self.storage,
                    self.provider(v)?
                )),
            }
        } else {
            match v {
                Val::Read { .. } => unreachable!("memory providers are optimized i32 values"),
                Val::Const(t, n) => {
                    self.set(&name, n.i());
                    if *t == Ty::I64 {
                        self.set(&(name.clone() + "h"), n.i() >> 32);
                    }
                }
                Val::Reg(t, s) => {
                    self.cp(&name, s);
                    if *t == Ty::I64 {
                        self.cp(&(name.clone() + "h"), &(s.clone() + "h"));
                    }
                }
                Val::Expr(t, op, args) => {
                    if *t == Ty::I64 {
                        if self.opt && args.len() == 2 {
                            if let Val::Const(_, Num::I(k)) = args[1] {
                                let shift = if ["shl", "shr_s", "shr_u"].contains(&op.as_str()) {
                                    Some((op.as_str(), k as u32 & 63))
                                } else if ["div_s", "div_u"].contains(&op.as_str())
                                    && k > 0
                                    && (k as u64).is_power_of_two()
                                {
                                    Some((
                                        if op == "div_s" { "shr_s" } else { "shr_u" },
                                        (k as u64).trailing_zeros(),
                                    ))
                                } else {
                                    None
                                };
                                if let Some((shift_op, count)) = shift {
                                    self.materialize(&args[0], Some(Val::reg(Ty::I64, "a")))?;
                                    if op == "div_s" && count > 0 {
                                        // Add 2^count-1 only to negative dividends so
                                        // arithmetic shift rounds toward zero.
                                        let bias = (1u64 << count) - 1;
                                        self.set("b", bias as i64);
                                        self.set("bh", (bias >> 32) as i64);
                                        self.emit(format!("execute if score {} matches ..-1 run function {}:rt/div_pow2_bias",self.score("ah"),self.ns));
                                    }
                                    self.rt(&format!("i64_{shift_op}/{count}"));
                                    self.cp(&name, "r");
                                    self.cp(&(name.clone() + "h"), "rh");
                                    return Ok(dest);
                                }
                            }
                        }
                        let args = self.freeze(args.to_vec())?;
                        for (i, a) in args.iter().enumerate() {
                            self.materialize(
                                a,
                                Some(Val::reg(*t, if i == 0 { "a" } else { "b" })),
                            )?;
                        }
                        self.rt(&format!("i64_{op}"));
                        self.cp(&name, "r");
                        self.cp(&(name.clone() + "h"), "rh");
                    } else {
                        self.int32(op, args, &name)?;
                    }
                }
            }
        }
        if v.ty() == Ty::I32 {
            self.ranges.insert(name, range);
        }
        Ok(dest)
    }
    fn interval(&self, v: &Val) -> (i64, i64) {
        self.checked_interval(v)
            .unwrap_or((i32::MIN as i64, i32::MAX as i64))
    }
    fn checked_interval(&self, v: &Val) -> Option<(i64, i64)> {
        let full = (i32::MIN as i64, i32::MAX as i64);
        let range = match v {
            Val::Const(Ty::I32, Num::I(n)) => (*n, *n),
            Val::Reg(Ty::I32, n) => {
                if n.starts_with(&format!("f{}", self.fid)) {
                    *self.ranges.get(n).unwrap_or(&full)
                } else {
                    full
                }
            }
            Val::Read { width, signed, .. } => {
                if *width == 32 {
                    full
                } else if *signed {
                    (-(1i64 << (width - 1)), (1i64 << (width - 1)) - 1)
                } else {
                    (0, (1i64 << width) - 1)
                }
            }
            Val::Expr(Ty::I32, op, args) if args.len() == 2 => {
                let (lo, hi) = self.interval(&args[0]);
                let (bl, bh) = self.interval(&args[1]);
                match op.as_str() {
                    "add" => (lo + bl, hi + bh),
                    "sub" => (lo - bh, hi - bl),
                    "mul" => {
                        let a = [lo * bl, lo * bh, hi * bl, hi * bh];
                        (*a.iter().min()?, *a.iter().max()?)
                    }
                    "and" if bl == bh && bl >= 0 => (0, bl),
                    "and" | "or" | "xor" => full,
                    "shr_u" if bl == bh && (1..=31).contains(&(bl & 31)) => {
                        (0, ((1u64 << (32 - (bl & 31))) - 1) as i64)
                    }
                    "shr_s" if bl == bh => (lo >> (bl & 31), hi >> (bl & 31)),
                    "shl" if bl == bh && (bl & 31) < 31 => {
                        (lo * (1i64 << (bl & 31)), hi * (1i64 << (bl & 31)))
                    }
                    "div_s" if bl > 0 || bh < 0 => {
                        let a = [lo / bl, lo / bh, hi / bl, hi / bh];
                        (*a.iter().min()?, *a.iter().max()?)
                    }
                    "rem_s" if bl > 0 || bh < 0 => {
                        let m = bl.abs().max(bh.abs()) - 1;
                        (if lo < 0 { -m } else { 0 }, if hi > 0 { m } else { 0 })
                    }
                    _ => return None,
                }
            }
            _ => return None,
        };
        (range.0 >= full.0 && range.1 <= full.1).then_some(range)
    }
    // Fuse only trees whose individual intermediate values fit native integer
    // providers. Wrapping hashes/PRNGs remain on scoreboards.
    fn integer_provider(&self, v: &Val) -> Option<Json> {
        self.checked_interval(v)?;
        match v {
            Val::Const(Ty::I32, _) | Val::Reg(Ty::I32, _) | Val::Read { .. } => {
                self.provider(v).ok()
            }
            Val::Expr(Ty::I32, op, args) if args.len() == 2 => {
                let left = self.integer_provider(&args[0])?;
                let right = self.integer_provider(&args[1])?;
                let constant = if let Val::Const(_, Num::I(k)) = args[1] {
                    Some(k)
                } else {
                    None
                };
                Some(match op.as_str() {
                    "add" | "mul" => json!({"type":op,"inputs":[left,right]}),
                    "sub" => json!({"type":"sub","left":left,"right":right}),
                    "div_s" | "rem_s" => {
                        let a = self.interval(&args[0]);
                        let b = self.interval(&args[1]);
                        if a.0 == i32::MIN as i64 && b.0 <= -1 && b.1 >= -1 {
                            return None;
                        }
                        json!({"type":if op=="div_s" {"div"} else {"mod"},"left":left,"right":right})
                    }
                    "and" | "or" | "xor" => {
                        // Avoid exponentially duplicating nested bitwise trees.
                        if left.to_string().len() + right.to_string().len() > 512 {
                            return None;
                        }
                        crate::bitwise::provider(op, left, right)
                    }
                    "shr_s" | "shr_u" => {
                        let k = constant? & 31;
                        if k >= 31 {
                            return None;
                        }
                        let p = json!({"type":"floor_div","left":left,"right":1u32<<k});
                        if op == "shr_u" && self.interval(&args[0]).0 < 0 {
                            if k <= 1 {
                                return None;
                            }
                            json!({"type":"floor_mod","left":p,"right":1u32<<(32-k)})
                        } else {
                            p
                        }
                    }
                    "shl" => {
                        let k = constant? & 31;
                        if k >= 31 {
                            return None;
                        }
                        json!({"type":"mul","inputs":[left,1u32<<k]})
                    }
                    _ => return None,
                })
            }
            _ => None,
        }
    }
    fn flush_reads(&mut self, stack: &mut [Val]) -> Result<()> {
        for v in stack {
            if v.reads_memory() {
                let d = self.tmp(v.ty());
                *v = self.materialize(v, Some(d))?;
            }
        }
        Ok(())
    }
    fn int32(&mut self, op: &str, args: &[Val], d: &str) -> Result<()> {
        if self.opt && args.len() == 2 {
            if let Val::Const(_, Num::I(k)) = args[1] {
                let a = self.scalar(&args[0])?;
                let k = k as i32;
                if ["shl", "shr_s", "shr_u"].contains(&op) {
                    self.cp("a", &a);
                    self.rt(&format!("{op}/{}", (k as u32) & 31));
                    self.cp(d, "r");
                    return Ok(());
                }
                if op == "and" && k >= 0 && ((k as u32) + 1).is_power_of_two() {
                    self.cp(d, &a);
                    if k == 0 {
                        self.set(d, 0);
                    } else if k == i32::MAX {
                        let c = self.c(i32::MIN as i64);
                        self.emit(format!("execute if score {} matches ..-1 run scoreboard players operation {} -= {}", self.score(d), self.score(d), self.score(&c)));
                    } else {
                        let c = self.c(k as i64 + 1);
                        self.operation(d, "%=", &c);
                    }
                    return Ok(());
                }
                if (op == "and" && k == -1)
                    || (["or", "xor", "add", "sub"].contains(&op) && k == 0)
                    || (op == "mul" && k == 1)
                {
                    self.cp(d, &a);
                    return Ok(());
                }
                if ["div_u", "rem_u"].contains(&op) && k > 0 {
                    let divisor = self.c(k as i64);
                    self.cp("const_div_a", &a);
                    self.cp("const_div_q", &a);
                    self.cp("const_div_r", &a);
                    self.operation("const_div_q", "/=", &divisor);
                    self.operation("const_div_r", "%=", &divisor);
                    let q = self.c((1u64 << 32).div_euclid(k as u64) as i64);
                    let r = (1u64 << 32).rem_euclid(k as u64) as i64;
                    self.emit(format!("execute if score {} matches ..-1 run scoreboard players operation {} += {}", self.score("const_div_a"), self.score("const_div_q"), self.score(&q)));
                    // Avoid intermediate overflow when k is near i32::MAX.
                    if r != 0 {
                        self.cp("const_div_r0", "const_div_r");
                        self.emit(format!("execute if score {} matches ..-1 if score {} matches {}.. run scoreboard players add {} 1",self.score("const_div_a"),self.score("const_div_r"),k as i64-r,self.score("const_div_q")));
                        let adjust = self.c(r - k as i64);
                        self.emit(format!("execute if score {} matches ..-1 if score {} matches {}.. run scoreboard players operation {} += {}",self.score("const_div_a"),self.score("const_div_r"),k as i64-r,self.score("const_div_r"),self.score(&adjust)));
                        self.emit(format!("execute if score {} matches ..-1 unless score {} matches {}.. run scoreboard players add {} {r}",self.score("const_div_a"),self.score("const_div_r0"),k as i64-r,self.score("const_div_r")));
                    }
                    self.cp(
                        d,
                        if op == "div_u" {
                            "const_div_q"
                        } else {
                            "const_div_r"
                        },
                    );
                    return Ok(());
                }
            }
        }
        // Evaluate operands before moving either into shared runtime registers.
        let a = self.scalar(&args[0])?;
        let mut b = if args.len() > 1 {
            self.scalar(&args[1])?
        } else {
            String::new()
        };
        match op {
            "add" | "sub" | "mul" => {
                if d == b && d != a {
                    let t = self.tmp(Ty::I32);
                    self.cp(t.name(), &b);
                    b = t.name().into();
                }
                self.cp(d, &a);
                self.operation(
                    d,
                    match op {
                        "add" => "+=",
                        "sub" => "-=",
                        _ => "*=",
                    },
                    &b,
                );
            }
            "div_s" | "rem_s" => {
                let p = json!({"type":if op=="div_s"{"div"}else{"mod"},"left":self.provider(&Val::reg(Ty::I32,a))?,"right":self.provider(&Val::reg(Ty::I32,b))?});
                self.emit(format!(
                    "execute store result score {} run compute default integer {p}",
                    self.score(d)
                ));
            }
            _ => {
                self.cp("a", &a);
                if !b.is_empty() {
                    self.cp("b", &b);
                }
                self.rt(&format!("i32_{op}"));
                self.cp(d, "r");
            }
        }
        Ok(())
    }
    fn freeze(&mut self, vals: Vec<Val>) -> Result<Vec<Val>> {
        vals.iter()
            .map(|v| {
                if self.opt
                    && (matches!(v, Val::Const(..))
                        || matches!(v,Val::Reg(_,n) if n.starts_with(&format!("f{}",self.fid))))
                {
                    return Ok(v.clone());
                }
                let d = self.tmp(v.ty());
                self.materialize(v, Some(d))
            })
            .collect()
    }
    fn join(&mut self, vals: &[Val], height: usize) -> Result<()> {
        let vs = if self.opt {
            let targets = (0..vals.len())
                .map(|i| format!("f{}s{}", self.fid, height + i))
                .collect::<Vec<_>>();
            vals.iter()
                .enumerate()
                .map(|(i, v)| {
                    if targets
                        .iter()
                        .enumerate()
                        .any(|(j, n)| j != i && v.depends(n))
                    {
                        let d = self.tmp(v.ty());
                        self.materialize(v, Some(d))
                    } else {
                        Ok(v.clone())
                    }
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            self.freeze(vals.to_vec())?
        };
        for (i, v) in vs.iter().enumerate() {
            let d = self.reg(v.ty(), format!("f{}s{}", self.fid, height + i));
            self.materialize(v, Some(d))?;
        }
        Ok(())
    }
    fn jump(&mut self, stack: &[Val], l: &Label) -> Result<()> {
        ensure!(stack.len() >= l.types.len(), "stack underflow at branch");
        self.join(&stack[stack.len() - l.types.len()..], l.height)?;
        self.emit(format!("return run function {}:{}", self.ns, l.target));
        Ok(())
    }
    fn block(
        &mut self,
        name: &str,
        nodes: &[Node<'a>],
        stack: Vec<Val>,
        labels: &[Label],
        end: &Label,
    ) -> Result<()> {
        let old = std::mem::take(&mut self.lines);
        // Emitting a sibling/continuation must not inherit facts from the last
        // branch emitted. Facts passed into this block are valid on every entry.
        let constants = self.local_constants.clone();
        let memory = self.memory_values.clone();
        let ranges = self.ranges.clone();
        self.sequence(nodes, stack, labels, end)?;
        self.local_constants = constants;
        self.memory_values = memory;
        self.ranges = ranges;
        let lines = std::mem::replace(&mut self.lines, old);
        self.function(name, lines);
        Ok(())
    }
    /// Conservative region effects. Loop entries and branch joins may retain
    /// only facts unaffected by *any* path, including nested branches/backedges.
    fn invalidate_region(&mut self, nodes: &[Node<'a>]) {
        for node in nodes {
            match node {
                Node::Structured { body, other, .. } => {
                    self.invalidate_region(body);
                    self.invalidate_region(other);
                }
                Node::Op(op) => match op {
                    O::LocalSet { local_index } | O::LocalTee { local_index } => {
                        self.local_constants.remove(local_index);
                        let name = format!("f{}l{local_index}", self.fid);
                        self.memory_values
                            .retain(|(p, _, _, _, v)| !p.depends(&name) && !v.depends(&name));
                    }
                    O::GlobalSet { global_index } => {
                        let name = format!("g{global_index}");
                        self.memory_values
                            .retain(|(p, _, _, _, v)| !p.depends(&name) && !v.depends(&name));
                    }
                    O::Call { .. }
                    | O::CallIndirect { .. }
                    | O::MemoryCopy { .. }
                    | O::MemoryFill { .. } => self.memory_values.clear(),
                    _ => {
                        if memory_op(op).is_some_and(|(_, _, store, _, _, _)| store) {
                            self.memory_values.clear();
                        }
                    }
                },
            }
        }
    }
    fn invalidate_stack_facts(&mut self) {
        let prefix = format!("f{}s", self.fid);
        self.memory_values
            .retain(|(p, _, _, _, v)| !p.depends_prefix(&prefix) && !v.depends_prefix(&prefix));
    }
    fn sequence(
        &mut self,
        nodes: &[Node<'a>],
        mut stack: Vec<Val>,
        labels: &[Label],
        end: &Label,
    ) -> Result<()> {
        self.ranges.clear();
        self.fallthrough_ranges.clear();
        for (index, node) in nodes.iter().enumerate() {
            if let Node::Structured {
                kind,
                params,
                results,
                body,
                other,
            } = node
            {
                let condition = if *kind == "if" {
                    Some(stack.pop().context("if stack underflow")?)
                } else {
                    None
                };
                let height = stack
                    .len()
                    .checked_sub(params.len())
                    .context("block stack underflow")?;
                let entry = self.fresh("b");
                let after = self.fresh("join");
                let condition = condition.map(|c| self.scalar(&c)).transpose()?;
                self.join(&stack, 0)?;
                self.invalidate_stack_facts();
                let entry_stack = stack
                    .iter()
                    .enumerate()
                    .map(|(i, v)| self.reg(v.ty(), format!("f{}s{i}", self.fid)))
                    .collect::<Vec<_>>();
                let mut after_stack = entry_stack[..height].to_vec();
                for (i, t) in results.iter().enumerate() {
                    after_stack.push(self.reg(*t, format!("f{}s{}", self.fid, height + i)));
                }
                let continuation = Label {
                    target: after.clone(),
                    height,
                    types: results.clone(),
                };
                let label = if *kind == "loop" {
                    Label {
                        target: entry.clone(),
                        height,
                        types: params.clone(),
                    }
                } else {
                    continuation.clone()
                };
                let mut nested = labels.to_vec();
                nested.push(label);
                if let Some(c) = condition {
                    let other_name = self.fresh("else");
                    self.emit(format!(
                        "execute unless score {} matches 0 run return run function {}:{entry}",
                        self.score(&c),
                        self.ns
                    ));
                    self.emit(format!("return run function {}:{other_name}", self.ns));
                    self.block(
                        &other_name,
                        other,
                        entry_stack.clone(),
                        &nested,
                        &continuation,
                    )?;
                } else {
                    self.emit(format!("return run function {}:{entry}", self.ns));
                }
                if *kind == "loop" {
                    self.invalidate_region(body);
                }
                self.block(&entry, body, entry_stack, &nested, &continuation)?;
                self.invalidate_region(body);
                self.invalidate_region(other);
                self.block(&after, &nodes[index + 1..], after_stack, labels, end)?;
                return Ok(());
            }
            let Node::Op(op) = node else { unreachable!() };
            match op {
                O::Nop => {}
                O::Unreachable => {
                    self.set("trap", 1);
                    self.emit("return fail");
                    return Ok(());
                }
                O::Br { relative_depth } => {
                    self.jump(&stack, &labels[labels.len() - 1 - *relative_depth as usize])?;
                    return Ok(());
                }
                O::BrIf { relative_depth } => {
                    let v = stack.pop().context("br_if stack")?;
                    let c = self.scalar(&v)?;
                    let name = self.fresh("taken");
                    let old = std::mem::take(&mut self.lines);
                    self.jump(&stack, &labels[labels.len() - 1 - *relative_depth as usize])?;
                    self.invalidate_stack_facts();
                    let lines = std::mem::replace(&mut self.lines, old);
                    self.function(&name, lines);
                    self.emit(format!(
                        "execute unless score {} matches 0 run return run function {}:{name}",
                        self.score(&c),
                        self.ns
                    ));
                    if let Some(facts) = self.fallthrough_ranges.get(&c) {
                        for (reg, range) in facts {
                            self.ranges.insert(reg.clone(), *range);
                        }
                    }
                }
                O::BrTable { targets } => {
                    let v = stack.pop().context("br_table stack")?;
                    let c = self.scalar(&v)?;
                    for (i, depth) in targets.targets().enumerate() {
                        let name = self.fresh("case");
                        let old = std::mem::take(&mut self.lines);
                        self.jump(&stack, &labels[labels.len() - 1 - depth? as usize])?;
                        let lines = std::mem::replace(&mut self.lines, old);
                        self.function(&name, lines);
                        self.emit(format!(
                            "execute if score {} matches {i} run return run function {}:{name}",
                            self.score(&c),
                            self.ns
                        ));
                    }
                    self.jump(
                        &stack,
                        &labels[labels.len() - 1 - targets.default() as usize],
                    )?;
                    return Ok(());
                }
                O::Return => {
                    self.jump(&stack, &labels[0])?;
                    return Ok(());
                }
                O::Drop => {
                    stack.pop().context("drop stack")?;
                }
                O::I32Const { value } => stack.push(Val::int(*value as i64)),
                O::I64Const { value } => stack.push(Val::Const(Ty::I64, Num::I(*value))),
                O::F32Const { value } => stack.push(Val::Const(
                    Ty::F32,
                    Num::F(f32::from_bits(value.bits()) as f64),
                )),
                O::F64Const { value } => {
                    stack.push(Val::Const(Ty::F64, Num::F(f64::from_bits(value.bits()))))
                }
                O::LocalGet { local_index } => {
                    let v = self.reg(
                        self.locals[*local_index as usize],
                        format!("f{}l{local_index}", self.fid),
                    );
                    let v = if self.opt {
                        self.local_constants.get(local_index).cloned().unwrap_or(v)
                    } else {
                        let d = self.tmp(v.ty());
                        self.materialize(&v, Some(d))?
                    };
                    stack.push(v);
                }
                O::GlobalGet { global_index } => {
                    let v = self.reg(
                        self.m.globals[*global_index as usize].0,
                        format!("g{global_index}"),
                    );
                    stack.push(v);
                }
                O::LocalSet { local_index } | O::LocalTee { local_index } => {
                    self.fallthrough_ranges.clear();
                    let v = stack.pop().context("local.set stack")?;
                    let d = self.reg(v.ty(), format!("f{}l{local_index}", self.fid));
                    self.memory_values
                        .retain(|(p, _, _, _, v)| !p.depends(d.name()) && !v.depends(d.name()));
                    for s in &mut stack {
                        if s.depends(d.name()) {
                            let t = self.tmp(s.ty());
                            *s = self.materialize(s, Some(t))?;
                        }
                    }
                    self.materialize(&v, Some(d.clone()))?;
                    self.local_constants.remove(local_index);
                    if self.opt && matches!(v, Val::Const(Ty::I32, _)) {
                        self.local_constants.insert(*local_index, v.clone());
                    }
                    if matches!(op, O::LocalTee { .. }) {
                        stack.push(self.local_constants.get(local_index).cloned().unwrap_or(d));
                    }
                }
                O::GlobalSet { global_index } => {
                    self.fallthrough_ranges.clear();
                    let v = stack.pop().context("global.set stack")?;
                    let d = self.reg(v.ty(), format!("g{global_index}"));
                    self.memory_values
                        .retain(|(p, _, _, _, v)| !p.depends(d.name()) && !v.depends(d.name()));
                    for s in &mut stack {
                        if s.depends(d.name()) {
                            let t = self.tmp(s.ty());
                            *s = self.materialize(s, Some(t))?;
                        }
                    }
                    self.materialize(&v, Some(d))?;
                }
                O::Call { function_index } => {
                    self.fallthrough_ranges.clear();
                    self.memory_values.clear();
                    let f = *function_index as usize;
                    let n = self.m.signature(f).0.len();
                    ensure!(stack.len() >= n, "call stack underflow");
                    let argv = stack.split_off(stack.len() - n);
                    stack = self.freeze(stack)?;
                    stack.extend(self.call(f, argv)?);
                    self.ranges.clear();
                }
                O::CallIndirect {
                    type_index,
                    table_index,
                    ..
                } => {
                    self.fallthrough_ranges.clear();
                    self.memory_values.clear();
                    self.ranges.clear();
                    ensure!(*table_index == 0, "one table supported");
                    let idx = stack.pop().context("indirect index")?;
                    let idx = self.materialize(&idx, None)?;
                    let n = self.m.types[*type_index as usize].0.len();
                    let argv = stack.split_off(stack.len() - n);
                    stack = self.freeze(stack)?;
                    let argv = self.freeze(argv)?;
                    for (i, v) in argv.iter().enumerate() {
                        self.materialize(v, Some(Val::reg(v.ty(), format!("arg{i}"))))?;
                    }
                    self.cp("ind", idx.name());
                    self.emit(format!("function {}:table/t{type_index}", self.ns));
                    self.trap_check();
                    for (i, t) in self.m.types[*type_index as usize]
                        .1
                        .clone()
                        .iter()
                        .enumerate()
                    {
                        let d = self.tmp(*t);
                        stack.push(self.materialize(&Val::reg(*t, format!("ret{i}")), Some(d))?);
                    }
                }
                O::Select => {
                    let c = stack.pop().context("select condition")?;
                    let b = stack.pop().context("select rhs")?;
                    let a = stack.pop().context("select lhs")?;
                    let c = self.scalar(&c)?;
                    let d = self.tmp(a.ty());
                    self.materialize(&b, Some(d.clone()))?;
                    let name = self.fresh("select");
                    let old = std::mem::take(&mut self.lines);
                    self.materialize(&a, Some(d.clone()))?;
                    let lines = std::mem::replace(&mut self.lines, old);
                    self.function(&name, lines);
                    self.emit(format!(
                        "execute unless score {} matches 0 run function {}:{name}",
                        self.score(&c),
                        self.ns
                    ));
                    stack.push(d);
                }
                O::MemorySize { mem, .. } => {
                    ensure!(*mem == 0, "one memory");
                    let d = self.tmp(Ty::I32);
                    stack.push(self.materialize(&Val::reg(Ty::I32, "pages"), Some(d))?);
                }
                O::MemoryGrow { mem, .. } => {
                    ensure!(*mem == 0, "one memory");
                    let n = stack.pop().context("grow stack")?;
                    self.materialize(&n, Some(Val::reg(Ty::I32, "grow")))?;
                    self.rt("grow");
                    let d = self.tmp(Ty::I32);
                    stack.push(self.materialize(&Val::reg(Ty::I32, "grow_result"), Some(d))?);
                }
                O::MemoryCopy { dst_mem, src_mem } => {
                    ensure!(*dst_mem == 0 && *src_mem == 0, "one memory");
                    self.bulk(&mut stack, "memory_copy")?;
                }
                O::MemoryFill { mem } => {
                    ensure!(*mem == 0, "one memory");
                    self.bulk(&mut stack, "memory_fill")?;
                }
                _ => {
                    if let Some((ty, width, is_store, is_signed, offset, align)) = memory_op(op) {
                        let value = if is_store {
                            Some(stack.pop().context("store value")?)
                        } else {
                            None
                        };
                        let addr = stack.pop().context("memory address")?;
                        if is_store {
                            self.flush_reads(&mut stack)?;
                        }
                        if let Some(v) =
                            self.memory(ty, width, is_signed, offset, align, addr, value)?
                        {
                            stack.push(v);
                        }
                    } else {
                        let name = numeric_name(op)?;
                        let v = self
                            .numeric(&name, &mut stack)
                            .with_context(|| format!("function {} instruction {op:?}", self.fid))?;
                        stack.push(v);
                    }
                }
            }
        }
        self.jump(&stack, end)
    }
    fn trap_check(&mut self) {
        self.emit(format!(
            "execute if score {} matches 1.. run return fail",
            self.score("trap")
        ));
    }
    fn checked_guest(&self) -> bool {
        self.m.accurate && self.fid < self.m.helper_start.unwrap_or(usize::MAX)
    }
    fn fail_if(&mut self, condition: String, kind: &str) {
        self.emit(format!(
            "execute {condition} run return run function {}:rt/trap_{kind}",
            self.ns
        ));
    }
    fn bounds(&mut self, addr: Val, offset: u64, bytes: u64) -> Result<Val> {
        if !self.checked_guest() {
            return Ok(addr);
        }
        if let Val::Const(_, Num::I(n)) = addr {
            let end = n as u32 as u64 + offset + bytes;
            if end > crate::accurate::HELPER_BASE {
                self.emit(format!("return run function {}:rt/trap_memory", self.ns));
            } else if end > self.m.pages as u64 * 65536 {
                self.fail_if(
                    format!(
                        "unless score {} matches {}..",
                        self.score("pages"),
                        end.div_ceil(65536)
                    ),
                    "memory",
                );
            }
            return Ok(addr);
        }
        let addr = self.materialize(&addr, None)?;
        self.materialize(&addr, Some(Val::reg(Ty::I32, "bounds_addr")))?;
        if offset + bytes > crate::accurate::HELPER_BASE {
            self.emit(format!("return run function {}:rt/trap_memory", self.ns));
            return Ok(addr);
        }
        self.fail_if(
            format!("if score {} matches ..-1", self.score("bounds_addr")),
            "memory",
        );
        self.cp("bounds_limit", "pages");
        let c = self.c(65536);
        self.operation("bounds_limit", "*=", &c);
        let c = self.c((offset + bytes) as i64);
        self.operation("bounds_limit", "-=", &c);
        self.fail_if(
            format!(
                "if score {} > {}",
                self.score("bounds_addr"),
                self.score("bounds_limit")
            ),
            "memory",
        );
        Ok(addr)
    }
    fn bulk(&mut self, stack: &mut Vec<Val>, name: &str) -> Result<()> {
        let args = stack.split_off(stack.len() - 3);
        self.flush_reads(stack)?;
        self.memory_values.clear();
        let args = self.freeze(args)?;
        for (k, v) in ["dst", "src", "n"].iter().zip(&args) {
            self.materialize(v, Some(Val::reg(Ty::I32, *k)))?;
        }
        if self.checked_guest() {
            self.fail_if(
                format!("if score {} matches ..-1", self.score("n")),
                "memory",
            );
            for pointer in if name == "memory_copy" {
                vec!["dst", "src"]
            } else {
                vec!["dst"]
            } {
                self.fail_if(
                    format!("if score {} matches ..-1", self.score(pointer)),
                    "memory",
                );
                self.cp("bounds_limit", "pages");
                let c = self.c(65536);
                self.operation("bounds_limit", "*=", &c);
                self.operation("bounds_limit", "-=", pointer);
                self.fail_if(
                    format!(
                        "if score {} < {}",
                        self.score("bounds_limit"),
                        self.score("n")
                    ),
                    "memory",
                );
            }
        }
        self.rt(name);
        Ok(())
    }
    fn numeric(&mut self, full: &str, stack: &mut Vec<Val>) -> Result<Val> {
        let (typ, name) = full.split_once('.').unwrap();
        let ty = Ty::from_name(typ)?;
        let unary = [
            "clz", "ctz", "popcnt", "abs", "neg", "ceil", "floor", "trunc", "nearest", "sqrt",
            "eqz",
        ]
        .contains(&name)
            || [
                "extend",
                "wrap",
                "trunc_",
                "convert",
                "demote",
                "promote",
                "reinterpret",
            ]
            .iter()
            .any(|p| name.starts_with(p));
        let mut args = vec![stack.pop().context("numeric stack")?];
        if !unary {
            args.insert(0, stack.pop().context("numeric stack")?);
        }
        if self.m.accurate && !ty.float() && ["div_s", "div_u", "rem_s", "rem_u"].contains(&name) {
            args = self.freeze(args)?;
            for (v, k) in args.iter().zip(["check_a", "check_b"]) {
                self.materialize(v, Some(Val::reg(ty, k)))?;
            }
            let zero = if ty == Ty::I64 {
                format!(
                    "if score {} matches 0 if score {} matches 0",
                    self.score("check_b"),
                    self.score("check_bh")
                )
            } else {
                format!("if score {} matches 0", self.score("check_b"))
            };
            self.fail_if(zero, "integer");
            if name == "div_s" {
                let overflow = if ty == Ty::I64 {
                    format!("if score {} matches 0 if score {} matches -2147483648 if score {} matches -1 if score {} matches -1",self.score("check_a"),self.score("check_ah"),self.score("check_b"),self.score("check_bh"))
                } else {
                    format!(
                        "if score {} matches -2147483648 if score {} matches -1",
                        self.score("check_a"),
                        self.score("check_b")
                    )
                };
                self.fail_if(overflow, "integer");
            }
        }
        if [
            "eq", "ne", "lt", "gt", "le", "ge", "lt_s", "lt_u", "gt_s", "gt_u", "le_s", "le_u",
            "ge_s", "ge_u", "eqz",
        ]
        .contains(&name)
        {
            if name == "eqz" {
                args.push(Val::Const(ty, Num::I(0)));
            }
            return self.compare(ty, if name == "eqz" { "eq" } else { name }, args);
        }
        if [
            "extend",
            "wrap",
            "trunc_",
            "convert",
            "demote",
            "promote",
            "reinterpret",
        ]
        .iter()
        .any(|p| name.starts_with(p))
        {
            return self.convert(ty, name, args.remove(0));
        }
        if self.opt {
            if let Some(v) = fold(ty, name, &args) {
                return Ok(v);
            }
        }
        if ty == Ty::F64 {
            self.warnings
                .insert("f64 arithmetic uses native f32 precision".into());
        }
        if ty.float() && name == "nearest" {
            self.warnings
                .insert("nearest uses Minecraft ties-toward-positive-infinity rounding".into());
        }
        if ty.float() && name == "copysign" {
            let args = self.freeze(args)?;
            self.materialize(&args[0], Some(Val::reg(ty, "fa")))?;
            self.materialize(&args[1], Some(Val::reg(ty, "fb")))?;
            self.rt("fcopysign");
            let d = self.tmp(ty);
            return self.materialize(&Val::reg(ty, "fr"), Some(d));
        }
        let v = Val::Expr(ty, name.into(), args);
        if self.opt {
            Ok(v)
        } else {
            self.materialize(&v, None)
        }
    }
    fn compare(&mut self, ty: Ty, name: &str, args: Vec<Val>) -> Result<Val> {
        if self.opt && (ty == Ty::I32 || ty.float()) {
            let d = self.tmp(Ty::I32);
            // LLVM commonly combines unsigned power-of-two bounds with OR.
            // Lower the Wasm pattern exactly; do not depend on source-language
            // spelling or on operands being nonnegative.
            if ty == Ty::I32 && ["gt_u", "le_u"].contains(&name) {
                if let (Val::Expr(Ty::I32, op, values), Val::Const(Ty::I32, Num::I(mask))) =
                    (&args[0], &args[1])
                {
                    if op == "or"
                        && *mask >= 0
                        && *mask <= i32::MAX as i64
                        && ((*mask as u64) + 1).is_power_of_two()
                    {
                        let a = self.scalar(&values[0])?;
                        let b = self.scalar(&values[1])?;
                        self.set(d.name(), if name == "gt_u" { 1 } else { 0 });
                        self.emit(format!("execute if score {} matches 0..{mask} if score {} matches 0..{mask} run scoreboard players set {} {}",self.score(&a),self.score(&b),self.score(d.name()),if name=="gt_u" {0}else{1}));
                        if name == "gt_u" {
                            self.fallthrough_ranges
                                .insert(d.name().into(), vec![(a, (0, *mask)), (b, (0, *mask))]);
                        }
                        self.ranges.insert(d.name().into(), (0, 1));
                        return Ok(d);
                    }
                }
            }
            if ty.float() {
                let a = self.provider(&args[0])?;
                let b = self.provider(&args[1])?;
                let (condition, test) = match name {
                    "eq" => ("if", json!({"min":b,"max":b})),
                    "ne" => ("unless", json!({"min":b,"max":b})),
                    "lt" => ("unless", json!({"min":b})),
                    "le" => ("if", json!({"max":b})),
                    "gt" => ("unless", json!({"max":b})),
                    "ge" => ("if", json!({"min":b})),
                    _ => bail!("unknown float comparison {name}"),
                };
                self.set(d.name(), 0);
                self.emit(format!(
                    "execute {condition} predicate {} run scoreboard players set {} 1",
                    json!({"type":"float_value_check","value":a,"test":test}),
                    self.score(d.name())
                ));
            } else {
                let a = self.scalar(&args[0])?;
                let b = self.scalar(&args[1])?;
                let op = match name.trim_end_matches("_s").trim_end_matches("_u") {
                    "eq" | "ne" => "=",
                    "lt" => "<",
                    "le" => "<=",
                    "gt" => ">",
                    "ge" => ">=",
                    _ => bail!("unknown integer comparison {name}"),
                };
                self.set(d.name(), 0);
                if name.ends_with("_u") {
                    let (first, second) = if op.starts_with('<') {
                        ("0..", "..-1")
                    } else {
                        ("..-1", "0..")
                    };
                    self.emit(format!("execute if score {} matches {first} if score {} matches {second} run scoreboard players set {} 1",self.score(&a),self.score(&b),self.score(d.name())));
                    for sign in ["0..", "..-1"] {
                        self.emit(format!("execute if score {} matches {sign} if score {} matches {sign} if score {} {op} {} run scoreboard players set {} 1",self.score(&a),self.score(&b),self.score(&a),self.score(&b),self.score(d.name())));
                    }
                } else {
                    self.emit(format!(
                        "execute {} score {} {op} {} run scoreboard players set {} 1",
                        if name == "ne" { "unless" } else { "if" },
                        self.score(&a),
                        self.score(&b),
                        self.score(d.name())
                    ));
                }
            }
            self.ranges.insert(d.name().into(), (0, 1));
            return Ok(d);
        }
        let args = self.freeze(args)?;
        for (i, v) in args.iter().enumerate() {
            self.materialize(
                v,
                Some(Val::reg(
                    ty,
                    if ty.float() {
                        if i == 0 {
                            "fa"
                        } else {
                            "fb"
                        }
                    } else if i == 0 {
                        "a"
                    } else {
                        "b"
                    },
                )),
            )?;
        }
        if ty.float() {
            self.required.insert(format!("fcmp_{name}"));
            self.emit(format!(
                "function {}:rt/fcmp_{name} with storage {} r",
                self.ns, self.storage
            ));
        } else {
            self.rt(&format!("{}_{name}", ty.name()));
        }
        let d = self.tmp(Ty::I32);
        self.materialize(&Val::reg(Ty::I32, "r"), Some(d))
    }
    fn convert(&mut self, ty: Ty, name: &str, v: Val) -> Result<Val> {
        if name == "demote_f64" || name == "promote_f32" {
            self.warnings
                .insert("f64 represented at native f32 precision".into());
            return Ok(match v {
                Val::Const(_, n) => Val::Const(ty, n),
                Val::Reg(_, s) => Val::Reg(ty, s),
                Val::Expr(_, op, a) => Val::Expr(ty, op, a),
                Val::Read { .. } => unreachable!("integer memory read in float promotion"),
            });
        }
        if name == "wrap_i64" {
            let v = self.materialize(&v, None)?;
            return Ok(Val::reg(ty, v.name()));
        }
        if name.starts_with("extend_i32") {
            let d = self.tmp(ty);
            self.materialize(&v, Some(Val::reg(Ty::I32, d.name())))?;
            self.set(&(d.name().to_string() + "h"), 0);
            if name.ends_with("_s") {
                self.emit(format!(
                    "execute if score {} matches ..-1 run scoreboard players set {} -1",
                    self.score(d.name()),
                    self.score(&(d.name().to_string() + "h"))
                ));
            }
            return Ok(d);
        }
        if name.starts_with("convert_i32_s") {
            let v = self.materialize(&v, None)?;
            return Ok(Val::Expr(ty, "from_int".into(), vec![v]));
        }
        let input_float = v.ty().float();
        self.materialize(
            &v,
            Some(Val::reg(v.ty(), if input_float { "fa" } else { "a" })),
        )?;
        let helper = if name.starts_with("convert_i32_u") {
            "uint_float".into()
        } else if name.starts_with("convert_i64") {
            self.warnings.insert("i64 to float rounds to f32".into());
            format!("i64_float_{}", name.chars().last().unwrap())
        } else if name.starts_with("trunc_") {
            format!("float_{}_{}", ty.name(), name.chars().last().unwrap())
        } else if name.starts_with("reinterpret") {
            if ty == Ty::I64 || ty == Ty::F64 {
                self.warnings.insert(
                    "f64 bit reinterpretation passes through approximate f32 representation".into(),
                );
                if input_float {
                    "f64_bits".into()
                } else {
                    "bits_f64".into()
                }
            } else if input_float {
                "float_bits".into()
            } else {
                "bits_float".into()
            }
        } else if name.starts_with("extend") {
            format!("{}_{name}", ty.name())
        } else {
            bail!("unsupported conversion {name}")
        };
        self.rt(&helper);
        let d = self.tmp(ty);
        self.materialize(&Val::reg(ty, if ty.float() { "fr" } else { "r" }), Some(d))
    }
    fn memory(
        &mut self,
        ty: Ty,
        width: u32,
        sign: bool,
        offset: u64,
        _align: u8,
        addr: Val,
        value: Option<Val>,
    ) -> Result<Option<Val>> {
        let addr = self.bounds(addr, offset, width as u64 / 8)?;
        let ptr = if let Val::Const(_, Num::I(n)) = addr {
            Val::int(n.wrapping_add(offset as i64))
        } else if offset == 0 {
            addr
        } else {
            Val::Expr(Ty::I32, "add".into(), vec![addr, Val::int(offset as i64)])
        };
        if self.opt && value.is_none() {
            if let Some((_, _, _, _, v)) = self
                .memory_values
                .iter()
                .rev()
                .find(|(p, t, w, s, _)| p == &ptr && *t == ty && *w == width && *s == sign)
            {
                return Ok(Some(v.clone()));
            }
        }
        if value.is_some() {
            // Separate constant byte ranges cannot alias. Keep the older value
            // across unrelated stores, but never guess about dynamic pointers.
            self.memory_values.retain(|(p, _, w, _, _)| {
                if let (Val::Const(Ty::I32, Num::I(a)), Val::Const(Ty::I32, Num::I(b))) = (&ptr, p)
                {
                    *a >= 0 && *b >= 0 && (*a + width as i64 / 8 <= *b || *b + *w as i64 / 8 <= *a)
                } else {
                    false
                }
            });
        }
        // Evaluate value first, then address: float/64-bit helpers may use ptr.
        let value = value.map(|v| self.materialize(&v, None)).transpose()?;
        let fixed = if self.opt {
            if let Val::Const(_, Num::I(n)) = &ptr {
                if *n >= 0 {
                    Some(*n as u64)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        let direct = fixed.filter(|p| {
            (*p % 4 == 0 && width >= 32) || (*p % 4 + width as u64 / 8 <= 4 && width < 32)
        });
        if self.word_buffer() {
            if let Some(p) = direct {
                for word in p / 4..p / 4 + if width == 64 { 2 } else { 1 } {
                    self.emit(format!(
                        "execute if score {} matches {word} run function {}:rt/wb_flush",
                        self.score("wb_addr"),
                        self.ns
                    ));
                    if value.is_some() {
                        self.emit(format!(
                            "execute if score {} matches {word} run scoreboard players set {} -1",
                            self.score("wb_addr"),
                            self.score("wb_addr")
                        ));
                    }
                }
                // A direct store updates the canonical word, so a later cached
                // read must not see the previous buffered value.
            }
        }
        if value.is_none() && ty == Ty::I32 && !self.float_cache() {
            if let Some(p) = direct {
                return Ok(Some(Val::Read {
                    word: p / 4,
                    shift: (p % 4) as u32 * 8,
                    width,
                    signed: sign,
                }));
            }
        }
        if direct.is_none() {
            self.materialize(&ptr, Some(Val::reg(Ty::I32, "ptr")))?;
        }
        if let Some(v) = value {
            if self.float_cache() && ty == Ty::F32 {
                self.materialize(&v, Some(Val::reg(ty, "fa")))?;
                if let Some(p) = direct {
                    self.emit(format!(
                        "data modify storage {}:fmem w.a{}.v set from storage {} r.fa",
                        self.ns,
                        p / 4,
                        self.storage
                    ));
                    self.emit(format!(
                        "data modify storage {}:fmem w.a{}.d set value 1b",
                        self.ns,
                        p / 4
                    ));
                } else {
                    self.materialize(&ptr, Some(Val::reg(Ty::I32, "ptr")))?;
                    self.rt("store_f32");
                }
                self.memory_values.push((ptr, ty, width, false, v));
                return Ok(None);
            }
            if ty.float() {
                self.materialize(&v, Some(Val::reg(ty, "fa")))?;
                self.rt("float_bits");
                if ty == Ty::F64 {
                    self.rt("f32_to_f64_bits");
                }
                self.cp("val", "r");
                if width == 64 {
                    self.cp("valh", "rh");
                }
            } else {
                self.materialize(&v, Some(Val::reg(ty, "val")))?;
            }
            if let Some(p) = direct {
                if self.float_cache() {
                    if width < 32 {
                        self.emit(format!(
                            "data modify storage {} macro.p set value {}",
                            self.storage,
                            p / 4
                        ));
                        self.emit(format!("execute if data storage {}:fmem w.a{}.d run function {}:rt/flush_float with storage {} macro",self.ns,p/4,self.ns,self.storage));
                    }
                    self.emit(format!("data remove storage {}:fmem w.a{}", self.ns, p / 4));
                    if width == 64 {
                        self.emit(format!(
                            "data remove storage {}:fmem w.a{}",
                            self.ns,
                            p / 4 + 1
                        ));
                    }
                }
                if width >= 32 {
                    self.emit(self.write_word(&(p / 4).to_string(), "val"));
                    if width == 64 {
                        self.emit(self.write_word(&(p / 4 + 1).to_string(), "valh"));
                    }
                } else {
                    let word = self.word_provider(p / 4);
                    self.emit(format!(
                        "execute store result score {} run compute default integer {word}",
                        self.score("static_word")
                    ));
                    let lane = json!({"type":"floor_mod","left":{"type":"floor_div","left":word,"right":1u32<<((p%4)*8)},"right":1u32<<width});
                    self.emit(format!(
                        "execute store result score {} run compute default integer {lane}",
                        self.score("static_lane")
                    ));
                    let mask = self.c(1i64 << width);
                    self.operation("val", "%=", &mask);
                    self.operation("val", "-=", "static_lane");
                    if p % 4 != 0 {
                        let factor = self.c(1i64 << ((p % 4) * 8));
                        self.operation("val", "*=", &factor);
                    }
                    self.operation("val", "+=", "static_word");
                    self.emit(self.write_word(&(p / 4).to_string(), "val"));
                }
            } else {
                self.rt(&format!("store{width}"));
            }
            if self.float_cache() && ty == Ty::F32 {
                self.materialize(&ptr, Some(Val::reg(Ty::I32, "ptr")))?;
                self.rt("cache_f32");
            }
            if self.opt
                && width
                    == if ty == Ty::I64 || ty == Ty::F64 {
                        64
                    } else {
                        32
                    }
            {
                self.memory_values.push((ptr, ty, width, false, v));
            }
            Ok(None)
        } else {
            if self.float_cache() && ty == Ty::F32 {
                if let Some(p) = direct {
                    let word = p / 4;
                    let name = format!("load_f32_at_{word}");
                    self.function(&format!("rt/{name}"), vec![
                        format!("execute if data storage {}:fmem w.a{word} run return run data modify storage {} r.fr set from storage {}:fmem w.a{word}.v",self.ns,self.storage,self.ns),
                        format!("execute store result score {} run compute default integer {}",self.score("a"),self.word_provider(word)),
                        format!("function {}:rt/bits_float",self.ns),
                        format!("data modify storage {}:fmem w.a{word}.v set from storage {} r.fr",self.ns,self.storage),
                    ]);
                    self.rt(&name);
                } else {
                    self.materialize(&ptr, Some(Val::reg(Ty::I32, "ptr")))?;
                    self.rt("load_f32");
                }
                let d = self.tmp(ty);
                let result = self.materialize(&Val::reg(ty, "fr"), Some(d))?;
                self.memory_values
                    .push((ptr, ty, width, sign, result.clone()));
                return Ok(Some(result));
            }
            if let Some(p) = direct {
                if self.float_cache() {
                    for word in p / 4..p / 4 + if width == 64 { 2 } else { 1 } {
                        self.emit(format!(
                            "data modify storage {} macro.p set value {word}",
                            self.storage
                        ));
                        self.emit(format!("execute if data storage {}:fmem w.a{word}.d run function {}:rt/flush_float with storage {} macro",self.ns,self.ns,self.storage));
                    }
                }
                let mut provider = self.word_provider(p / 4);
                if width < 32 {
                    provider = json!({"type":"floor_mod","left":{"type":"floor_div","left":provider,"right":1u32<<((p%4)*8)},"right":1u32<<width});
                }
                self.emit(format!(
                    "execute store result score {} run compute default integer {provider}",
                    self.score("r")
                ));
                if width == 64 {
                    let provider = self.word_provider(p / 4 + 1);
                    self.emit(format!(
                        "execute store result score {} run compute default integer {provider}",
                        self.score("rh")
                    ));
                }
            } else {
                self.rt(&format!("load{width}"));
            }
            let name = if ty.float() {
                if ty == Ty::F64 {
                    self.rt("f64_to_f32_bits");
                }
                self.cp("a", "r");
                self.rt("bits_float");
                "fr"
            } else {
                if sign && width < 32 {
                    self.cp("a", "r");
                    self.rt(&format!("i32_extend{width}_s"));
                }
                if ty == Ty::I64 && width < 64 {
                    self.set("rh", 0);
                    if sign {
                        self.emit(format!(
                            "execute if score {} matches ..-1 run scoreboard players set {} -1",
                            self.score("r"),
                            self.score("rh")
                        ));
                    }
                }
                "r"
            };
            let d = self.tmp(ty);
            let result = self.materialize(&Val::reg(ty, name), Some(d))?;
            if ty == Ty::I32 && width < 32 {
                self.ranges.insert(
                    result.name().into(),
                    if sign {
                        (-(1i64 << (width - 1)), (1i64 << (width - 1)) - 1)
                    } else {
                        (0, (1i64 << width) - 1)
                    },
                );
            }
            if self.opt {
                self.memory_values
                    .push((ptr, ty, width, sign, result.clone()));
            }
            Ok(Some(result))
        }
    }
    fn call(&mut self, fid: usize, argv: Vec<Val>) -> Result<Vec<Val>> {
        if let Some((module, name)) = self.m.funcs[fid].import.clone() {
            return self.host(fid, &module, &name, argv);
        }
        let argv = self.freeze(argv)?;
        for (i, v) in argv.iter().enumerate() {
            self.materialize(v, Some(Val::reg(v.ty(), format!("arg{i}"))))?;
        }
        self.emit(format!("function {}:f{fid}/call", self.ns));
        self.trap_check();
        let mut out = vec![];
        for (i, t) in self.m.signature(fid).1.iter().enumerate() {
            let d = self.tmp(*t);
            out.push(self.materialize(&Val::reg(*t, format!("ret{i}")), Some(d))?);
        }
        Ok(out)
    }
    fn host(&mut self, fid: usize, module: &str, name: &str, argv: Vec<Val>) -> Result<Vec<Val>> {
        let returns = self.m.signature(fid).1.clone();
        let key = format!("{module}.{name}");
        if let Some(binding) = self.bindings.get(&key).cloned() {
            let command = binding
                .as_str()
                .or_else(|| binding.get("command").and_then(Json::as_str))
                .context("binding requires a command")?;
            ensure!(
                !command.contains(['\n', '\r']),
                "single line command template required"
            );
            let capture = binding
                .get("capture")
                .and_then(Json::as_str)
                .unwrap_or("result");
            ensure!(
                ["result", "success", "outcome"].contains(&capture),
                "invalid command capture mode"
            );
            ensure!(
                returns.is_empty()
                    || returns
                        == [if capture == "outcome" {
                            Ty::I64
                        } else {
                            Ty::I32
                        }],
                "command binding return signature does not match capture"
            );
            for (i, v) in argv.iter().enumerate() {
                ensure!(
                    v.ty() != Ty::I64,
                    "command template uses i32/f32/f64 arguments"
                );
                if v.ty().float() {
                    let a = self.materialize(v, None)?;
                    self.emit(format!(
                        "data modify storage {} host.a{i} set from storage {} r.{}",
                        self.storage,
                        self.storage,
                        a.name()
                    ));
                } else {
                    let s = self.scalar(v)?;
                    self.emit(format!("execute store result storage {} host.a{i} int 1 run scoreboard players get {}",self.storage,self.score(&s)));
                }
            }
            let fname = format!("host/f{fid}");
            // Two execute-store modifiers observe a single execution. Keep the
            // capture inside the macro so return-run cannot erase failure status.
            if capture == "outcome" || (module == "minecraft" && name.starts_with("wc1_")) {
                ensure!(
                    !returns.is_empty(),
                    "SDK command imports require a return value"
                );
                let d = self.tmp(if capture == "outcome" {
                    Ty::I64
                } else {
                    Ty::I32
                });
                let stores = if capture == "outcome" {
                    format!(
                        "store result score {} store success score {}",
                        self.score(d.name()),
                        self.score(&format!("{}h", d.name()))
                    )
                } else {
                    format!("store {capture} score {}", self.score(d.name()))
                };
                // Different call sites can have different result registers.
                let unique = format!("{fname}_{}", d.name());
                let line = format!("execute {stores} run {command}");
                if command.contains("$(") {
                    // An invalid expanded command prevents the macro body from
                    // running at all; never reuse a previous invocation's result.
                    self.set(d.name(), 0);
                    if capture == "outcome" {
                        self.set(&format!("{}h", d.name()), 0);
                    }
                    self.function(&unique, vec![format!("${line}")]);
                    self.emit(format!(
                        "function {}:{unique} with storage {} host",
                        self.ns, self.storage
                    ));
                } else if command.split_whitespace().any(|s| s == "return") {
                    // A command-level return must leave this command helper,
                    // not skip the remaining guest instructions in fN/call.
                    self.function(&unique, vec![line]);
                    self.emit(format!("function {}:{unique}", self.ns));
                } else {
                    self.emit(line);
                }
                return Ok(vec![d]);
            }
            let line = if command.contains("$(") {
                format!("$return run {command}")
            } else {
                format!("return run {command}")
            };
            self.function(&fname, vec![line]);
            if !returns.is_empty() {
                let capture = binding
                    .get("capture")
                    .and_then(Json::as_str)
                    .unwrap_or("result");
                ensure!(
                    ["result", "success"].contains(&capture),
                    "capture must be result or success"
                );
                let d = self.tmp(Ty::I32);
                self.emit(format!(
                    "execute store {capture} score {} run function {}:{fname} with storage {} host",
                    self.score(d.name()),
                    self.ns,
                    self.storage
                ));
                return Ok(vec![d]);
            }
            self.emit(format!(
                "function {}:{fname} with storage {} host",
                self.ns, self.storage
            ));
            return Ok(vec![]);
        }
        if [
            "sinf", "sin", "cosf", "cos", "sqrtf", "sqrt", "floorf", "floor", "ceilf", "ceil",
            "fabsf", "fabs", "fmaxf", "fminf", "fmodf",
        ]
        .contains(&name)
        {
            ensure!(
                (argv.len() == 1 || argv.len() == 2) && returns.len() == 1,
                "math import signature"
            );
            let op = match name {
                "sinf" => "sin",
                "cosf" => "cos",
                "sqrtf" => "sqrt",
                "floorf" => "floor",
                "ceilf" => "ceil",
                "fabsf" | "fabs" => "abs",
                "fmaxf" => "max",
                "fminf" => "min",
                "fmodf" => "mod",
                s => s,
            };
            return Ok(vec![Val::Expr(returns[0], op.into(), argv)]);
        }
        match name {
            "mc_command_utf8_result" => {
                ensure!(
                    argv.len() == 2
                        && argv.iter().all(|a| a.ty() == Ty::I32)
                        && returns == [Ty::I32],
                    "UTF-8 command ABI requires (i32, i32) -> i32"
                );
                let argv = self.freeze(argv)?;
                self.materialize(&argv[0], Some(Val::reg(Ty::I32, "src")))?;
                self.materialize(&argv[1], Some(Val::reg(Ty::I32, "n")))?;
                self.rt("read_utf8");
                let d = self.tmp(Ty::I32);
                self.emit(format!(
                    "execute store result score {} run function {}:rt/utf8_command",
                    self.score(d.name()),
                    self.ns
                ));
                Ok(vec![d])
            }
            "mc_command_static_result" | "mc_command_static_success" => {
                ensure!(
                    argv.len() == 2 && returns == [Ty::I32],
                    "static command signature is (i32, i32) -> i32"
                );
                let (ptr,len)=match (&argv[0],&argv[1]) {
                    (Val::Const(_,Num::I(p)),Val::Const(_,Num::I(n))) if *p>=0 && (0..=32767).contains(n) => (*p as usize,*n as usize),
                    _=>bail!("static command requires constant pointer/length; inline the wrapper or use a template binding"),
                };
                let mut bytes = vec![0u8; len];
                for (offset, data) in &self.m.data {
                    for (i, byte) in data.iter().enumerate() {
                        let address = offset + i;
                        if (ptr..ptr + len).contains(&address) {
                            bytes[address - ptr] = *byte;
                        }
                    }
                }
                let command = std::str::from_utf8(&bytes)?.trim_start_matches('/');
                ensure!(
                    !command.is_empty() && !command.contains(['\n', '\r', '\0']),
                    "command must be one nonempty UTF-8 line"
                );
                let d = self.tmp(Ty::I32);
                let mode = if name.ends_with("success") {
                    "success"
                } else {
                    "result"
                };
                self.emit(format!(
                    "execute store {mode} score {} run {command}",
                    self.score(d.name())
                ));
                Ok(vec![d])
            }
            "print" => {
                let s = self.scalar(&argv[0])?;
                self.emit(format!(
                    "tellraw @a {}",
                    json!({"score":{"name":format!("#{s}"),"objective":self.obj}})
                ));
                Ok(vec![])
            }
            "mc_command" | "mc_command_result" | "mc_command_success" => {
                ensure!(argv.len() == 2, "command ABI requires pointer and length");
                let argv = self.freeze(argv)?;
                self.materialize(&argv[0], Some(Val::reg(Ty::I32, "src")))?;
                self.materialize(&argv[1], Some(Val::reg(Ty::I32, "n")))?;
                self.rt("read_string");
                if !returns.is_empty() {
                    let d = self.tmp(Ty::I32);
                    let mode = if name.ends_with("success") {
                        "success"
                    } else {
                        "result"
                    };
                    self.emit(format!("execute store {mode} score {} run function {}:rt/command with storage {} host",self.score(d.name()),self.ns,self.storage));
                    Ok(vec![d])
                } else {
                    self.emit(format!(
                        "function {}:rt/command with storage {} host",
                        self.ns, self.storage
                    ));
                    Ok(vec![])
                }
            }
            "mc_putc" | "putchar" => {
                self.materialize(&argv[0], Some(Val::reg(Ty::I32, "char")))?;
                self.rt("putc");
                Ok(if returns.is_empty() {
                    vec![]
                } else {
                    vec![argv[0].clone()]
                })
            }
            _ => bail!("unbound import {key}; provide --bindings command templates"),
        }
    }
    fn build(&mut self) -> Result<()> {
        for fid in self.reachable.clone() {
            if self.m.funcs[fid].import.is_some() {
                continue;
            }
            self.fid = fid;
            self.temp = 0;
            let (params, returns) = self.m.signature(fid).clone();
            self.locals = params
                .iter()
                .chain(&self.m.funcs[fid].locals)
                .copied()
                .collect();
            self.local_constants.clear();
            self.memory_values.clear();
            if self.opt {
                for (i, t) in self.locals.iter().enumerate().skip(params.len()) {
                    if *t == Ty::I32 {
                        self.local_constants.insert(i as u32, Val::int(0));
                    }
                }
            }
            let end = Label {
                target: format!("f{fid}/exit"),
                height: 0,
                types: returns.clone(),
            };
            let body = self.m.funcs[fid].body.clone();
            self.block(
                &format!("f{fid}/entry"),
                &body,
                vec![],
                &[end.clone()],
                &end,
            )?;
            self.lines.clear();
            for (i, t) in returns.iter().enumerate() {
                let s = self.reg(*t, format!("f{fid}s{i}"));
                self.materialize(&s, Some(Val::reg(*t, format!("ret{i}"))))?;
            }
            self.emit("return 0");
            let lines = std::mem::take(&mut self.lines);
            self.function(&format!("f{fid}/exit"), lines);
            if self.recursive.contains(&fid) {
                self.emit(format!("function {}:f{fid}/save", self.ns));
            }
            for (i, t) in self.locals.clone().iter().enumerate() {
                let v = if i < params.len() {
                    Val::reg(*t, format!("arg{i}"))
                } else {
                    Val::Const(*t, Num::I(0))
                };
                let d = self.reg(*t, format!("f{fid}l{i}"));
                self.materialize(&v, Some(d))?;
            }
            self.emit(format!("function {}:f{fid}/entry", self.ns));
            if self.recursive.contains(&fid) {
                self.emit(format!("function {}:f{fid}/restore", self.ns));
            }
            self.emit("return 0");
            let lines = std::mem::take(&mut self.lines);
            self.function(&format!("f{fid}/call"), lines);
            if self.recursive.contains(&fid) {
                self.frames(fid);
            }
        }
        let mut export_names = BTreeSet::new();
        for (name, fid) in &self.m.exports {
            let safe = name
                .to_lowercase()
                .chars()
                .map(|c| {
                    if c.is_ascii_lowercase() || c.is_ascii_digit() || "_./-".contains(c) {
                        c
                    } else {
                        '_'
                    }
                })
                .collect::<String>();
            ensure!(
                !safe.contains("..") && !safe.starts_with('/') && export_names.insert(safe.clone()),
                "invalid or colliding export name: {name}"
            );
            ensure!(
                self.m.funcs[*fid].import.is_none(),
                "exported imports unsupported"
            );
            let mut entry = vec![format!("scoreboard players set {} 0", self.score("trap"))];
            if self.m.accurate {
                entry.push(format!("function {}:f{fid}/call", self.ns));
                if self.word_buffer() {
                    entry.push(format!("function {}:rt/wb_flush", self.ns));
                }
                let g = self.m.helper_global_start.unwrap();
                entry.push(format!(
                    "scoreboard players set {} {}",
                    self.score(&format!("g{g}")),
                    self.m.globals[g].1.i()
                ));
                entry.push("return 0".into());
            } else if self.word_buffer() {
                entry.push(format!("function {}:f{fid}/call", self.ns));
                entry.push(format!("function {}:rt/wb_flush", self.ns));
                entry.push("return 0".into());
            } else {
                entry.push(format!("return run function {}:f{fid}/call", self.ns));
            }
            self.function(&format!("export/{safe}"), entry);
        }
        for tid in 0..self.m.types.len() {
            let mut lines = vec![];
            for (i, fid) in self.m.table.iter().enumerate() {
                if let Some(fid) = fid {
                    if (if self.m.accurate {
                        self.m.original_types[self.m.funcs[*fid].tid] == self.m.original_types[tid]
                    } else {
                        self.m.signature(*fid) == &self.m.types[tid]
                    }) && self.reachable.contains(fid)
                    {
                        lines.push(format!("execute if score {} matches {i} run return run function {}:f{fid}/call",self.score("ind"),self.ns));
                    }
                }
            }
            lines.extend([
                format!("scoreboard players set {} 2", self.score("trap")),
                "return fail".into(),
            ]);
            self.function(&format!("table/t{tid}"), lines);
        }
        crate::runtime::generate(self)?;
        self.init()?;
        if self.opt {
            crate::optimize::localize_memory(&mut self.files, &self.ns);
            crate::optimize::inline_command_helpers(&mut self.files, &self.ns);
            crate::optimize::simplify_score_code(&mut self.files, &self.ns);
        }
        self.prune_functions();
        self.files.insert("pack.mcmeta".into(),json!({"pack":{"description":"wasmcompute native AOT","min_format":[121,0],"max_format":[121,0]}}).to_string()+"\n");
        self.files.insert("wasmcompute.json".into(),serde_json::to_string_pretty(&json!({"format":1,"target":"26.3-rc-1","pack_format":121,"namespace":self.ns,"optimized":self.opt,"copy_specializations":self.copy_profile,"float_cache_layout":"word-record-v1","memory_backend":if self.score_memory {"scoreboard"} else {"nbt"},"exports":self.m.exports,"warnings":self.warnings,"recursive_functions":self.recursive,"compiler":"Rust wasmcompute 0.1.0"}))?+"\n");
        if self.m.accurate {
            self.files.insert(
                "licenses/accurate-softfloat.txt".into(),
                include_str!("../runtime/accurate/vendor/softfloat/LICENSE").into(),
            );
            self.files.insert(
                "licenses/accurate-musl.txt".into(),
                include_str!("../runtime/accurate/vendor/musl/COPYRIGHT").into(),
            );
            let mut metadata: serde_json::Value =
                serde_json::from_str(&self.files["wasmcompute.json"])?;
            metadata["semantics"] = json!("accurate");
            self.files.insert(
                "wasmcompute.json".into(),
                serde_json::to_string_pretty(&metadata)? + "\n",
            );
        }
        // Every generated call must resolve, including runtime feature dependencies.
        for (p, text) in &self.files {
            if !p.ends_with(".mcfunction") {
                continue;
            }
            for line in text.lines() {
                let marker = format!("function {}:", self.ns);
                if let Some((_, tail)) = line.split_once(&marker) {
                    let target = tail.split_whitespace().next().unwrap_or("");
                    if !target.contains("$(") {
                        ensure!(
                            self.files.contains_key(&format!(
                                "data/{}/function/{target}.mcfunction",
                                self.ns
                            )),
                            "unimplemented runtime function {target}, referenced by {p}"
                        );
                    }
                }
            }
        }
        Ok(())
    }
    fn prune_functions(&mut self) {
        let base = format!("data/{}/function/", self.ns);
        let marker = format!("function {}:", self.ns);
        let mut todo = self
            .files
            .keys()
            .filter(|p| {
                p.starts_with(&(base.clone() + "export/"))
                    || *p == &(base.clone() + "init.mcfunction")
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut live = BTreeSet::new();
        while let Some(path) = todo.pop() {
            if !live.insert(path.clone()) {
                continue;
            }
            if let Some(text) = self.files.get(&path) {
                for line in text.lines() {
                    if let Some((_, tail)) = line.split_once(&marker) {
                        let target = tail.split_whitespace().next().unwrap_or("");
                        if let Some((prefix, _)) = target.split_once("$(") {
                            todo.extend(
                                self.files
                                    .keys()
                                    .filter(|p| p.starts_with(&(base.clone() + prefix)))
                                    .cloned(),
                            );
                        } else {
                            todo.push(format!("{base}{target}.mcfunction"));
                        }
                    }
                }
            }
        }
        self.files
            .retain(|p, _| !p.ends_with(".mcfunction") || live.contains(p));
    }
    fn frames(&mut self, fid: usize) {
        let mut save = vec![format!(
            "data modify storage {} frames prepend value {{}}",
            self.storage
        )];
        let mut restore = vec![];
        for (n, t) in self.regs.get(&fid).cloned().unwrap_or_default() {
            if !n.starts_with(&format!("f{fid}")) {
                continue;
            }
            if t.float() {
                save.push(format!(
                    "data modify storage {} frames[0].{n} set from storage {} r.{n}",
                    self.storage, self.storage
                ));
                restore.push(format!(
                    "data modify storage {} r.{n} set from storage {} frames[0].{n}",
                    self.storage, self.storage
                ));
            } else {
                for s in if t == Ty::I64 {
                    vec![n.clone(), n + "h"]
                } else {
                    vec![n]
                } {
                    save.push(format!("execute store result storage {} frames[0].{s} int 1 run scoreboard players get {}",self.storage,self.score(&s)));
                    restore.push(format!(
                        "execute store result score {} run data get storage {} frames[0].{s}",
                        self.score(&s),
                        self.storage
                    ));
                }
            }
        }
        restore.push(format!("data remove storage {} frames[0]", self.storage));
        self.function(&format!("f{fid}/save"), save);
        self.function(&format!("f{fid}/restore"), restore);
    }
    fn init(&mut self) -> Result<()> {
        self.lines = vec![
            format!("scoreboard objectives add {} dummy", self.obj),
            format!("data modify storage {} r set value {{}}", self.storage),
            format!("data modify storage {} frames set value []", self.storage),
            format!("data modify storage {} host set value {{}}", self.storage),
            format!("data modify storage {} macro set value {{}}", self.storage),
            format!("data modify storage {} flush set value {{}}", self.storage),
            format!("data modify storage {} w set value {{}}", self.mem),
            format!("data modify storage {}:fmem w set value {{}}", self.ns),
            format!("data remove storage {}:fmem dirty", self.ns),
        ];
        if self.score_memory {
            self.emit(format!("scoreboard objectives add {}_mem dummy", self.ns));
            self.emit(format!("scoreboard players reset * {}_mem", self.ns));
        }
        self.set("trap", 0);
        self.set("wb_addr", -1);
        self.set("wb_dirty", 0);
        self.set("pages", self.m.pages as i64);
        for (i, (t, n)) in self.m.globals.iter().enumerate() {
            self.materialize(&Val::Const(*t, *n), Some(Val::reg(*t, format!("g{i}"))))?;
        }
        let mut words = BTreeMap::<usize, u32>::new();
        for (offset, data) in &self.m.data {
            for (i, byte) in data.iter().enumerate() {
                let addr = offset + i;
                let word = words.entry(addr / 4).or_default();
                let shift = (addr % 4) * 8;
                *word = (*word & !(255 << shift)) | ((*byte as u32) << shift);
            }
        }
        let lazy = self.lazy_initial_memory();
        let mut eager = BTreeSet::new();
        if lazy {
            // Constant-address providers and specialized copies bypass word_read.
            // Seed every literal address they reference before guest execution.
            for text in self.files.values() {
                for suffix in text.split("#m").skip(1) {
                    let digits: String = suffix.chars().take_while(char::is_ascii_digit).collect();
                    if let Ok(addr) = digits.parse::<usize>() {
                        eager.insert(addr);
                    }
                }
            }
            self.emit(format!(
                "data modify storage {}:rom pages set value {{}}",
                self.ns
            ));
            let mut pages = BTreeMap::<usize, [i32; 256]>::new();
            for (&addr, &word) in &words {
                if word != 0 {
                    pages.entry(addr / 256).or_insert([0; 256])[addr % 256] = word as i32;
                }
            }
            for (page, values) in pages {
                let values = values
                    .iter()
                    .map(i32::to_string)
                    .collect::<Vec<_>>()
                    .join(",");
                self.emit(format!(
                    "data modify storage {}:rom pages.p{page} set value [I;{values}]",
                    self.ns
                ));
            }
        }
        for (addr, word) in words {
            if word != 0 {
                if self.score_memory {
                    if lazy && !eager.contains(&addr) {
                        continue;
                    }
                    self.emit(format!(
                        "scoreboard players set #m{addr} {}_mem {}",
                        self.ns, word as i32
                    ));
                } else {
                    self.emit(format!(
                        "data modify storage {} w.a{addr} set value {}",
                        self.mem, word as i32
                    ));
                }
            }
        }
        for (k, v) in self.constants.clone() {
            self.set(&k, v as i64);
        }
        self.emit(format!("function {}:rt/init", self.ns));
        if let Some(f) = self.m.start {
            self.emit(format!("function {}:f{f}/call", self.ns));
            if self.word_buffer() {
                self.rt("wb_flush");
            }
        }
        let lines = std::mem::take(&mut self.lines);
        self.function("init", lines);
        Ok(())
    }
}

fn fast_float_constant(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(-(f32::MAX as f64), f32::MAX as f64)
    }
}

fn fold(ty: Ty, op: &str, args: &[Val]) -> Option<Val> {
    let a = match args.first()? {
        Val::Const(_, v) => *v,
        _ => return None,
    };
    let b = match args.last()? {
        Val::Const(_, v) => *v,
        _ => return None,
    };
    if ty.float() {
        let (a, b) = (a.f(), b.f());
        let n = match op {
            "add" => a + b,
            "sub" => a - b,
            "mul" => a * b,
            "div" => a / b,
            "abs" => a.abs(),
            "neg" => -a,
            "floor" => a.floor(),
            "ceil" => a.ceil(),
            "trunc" => a.trunc(),
            "sqrt" => a.sqrt(),
            "min" => a.min(b),
            "max" => a.max(b),
            _ => return None,
        };
        if n.is_finite() {
            Some(Val::Const(ty, Num::F(n as f32 as f64)))
        } else {
            None
        }
    } else {
        let (a, b) = (a.i(), b.i());
        let bits = if ty == Ty::I32 { 32 } else { 64 };
        let n = match op {
            "add" => a.wrapping_add(b),
            "sub" => a.wrapping_sub(b),
            "mul" => a.wrapping_mul(b),
            "and" => a & b,
            "or" => a | b,
            "xor" => a ^ b,
            "shl" => a.wrapping_shl((b as u32) % (bits)),
            "shr_s" => a.wrapping_shr((b as u32) % bits),
            _ => return None,
        };
        Some(Val::Const(
            ty,
            Num::I(if ty == Ty::I32 { n as i32 as i64 } else { n }),
        ))
    }
}
pub(crate) fn memory_op(op: &O<'_>) -> Option<(Ty, u32, bool, bool, u64, u8)> {
    macro_rules! m {
        ($t:ident,$w:expr,$st:expr,$s:expr,$a:expr) => {
            Some((Ty::$t, $w, $st, $s, $a.offset, $a.align))
        };
    }
    match op {
        O::I32Load { memarg: a } => m!(I32, 32, false, false, a),
        O::I64Load { memarg: a } => m!(I64, 64, false, false, a),
        O::F32Load { memarg: a } => m!(F32, 32, false, false, a),
        O::F64Load { memarg: a } => m!(F64, 64, false, false, a),
        O::I32Load8S { memarg: a } => m!(I32, 8, false, true, a),
        O::I32Load8U { memarg: a } => m!(I32, 8, false, false, a),
        O::I32Load16S { memarg: a } => m!(I32, 16, false, true, a),
        O::I32Load16U { memarg: a } => m!(I32, 16, false, false, a),
        O::I64Load8S { memarg: a } => m!(I64, 8, false, true, a),
        O::I64Load8U { memarg: a } => m!(I64, 8, false, false, a),
        O::I64Load16S { memarg: a } => m!(I64, 16, false, true, a),
        O::I64Load16U { memarg: a } => m!(I64, 16, false, false, a),
        O::I64Load32S { memarg: a } => m!(I64, 32, false, true, a),
        O::I64Load32U { memarg: a } => m!(I64, 32, false, false, a),
        O::I32Store { memarg: a } => m!(I32, 32, true, false, a),
        O::I64Store { memarg: a } => m!(I64, 64, true, false, a),
        O::F32Store { memarg: a } => m!(F32, 32, true, false, a),
        O::F64Store { memarg: a } => m!(F64, 64, true, false, a),
        O::I32Store8 { memarg: a } => m!(I32, 8, true, false, a),
        O::I32Store16 { memarg: a } => m!(I32, 16, true, false, a),
        O::I64Store8 { memarg: a } => m!(I64, 8, true, false, a),
        O::I64Store16 { memarg: a } => m!(I64, 16, true, false, a),
        O::I64Store32 { memarg: a } => m!(I64, 32, true, false, a),
        _ => None,
    }
}
