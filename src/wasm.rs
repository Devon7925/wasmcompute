use anyhow::{bail, ensure, Result};
use std::collections::{BTreeMap, BTreeSet};
use wasmparser::{
    BlockType, DataKind, ElementItems, ElementKind, ExternalKind, Operator, Parser, Payload, Type,
    TypeRef, ValType, Validator,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    I32,
    I64,
    F32,
    F64,
}
impl Ty {
    pub fn float(self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::F32 => "f32",
            Self::F64 => "f64",
        }
    }
    pub fn from_name(s: &str) -> Result<Self> {
        Ok(match s {
            "i32" => Self::I32,
            "i64" => Self::I64,
            "f32" => Self::F32,
            "f64" => Self::F64,
            _ => bail!("unsupported type {s}"),
        })
    }
}
impl TryFrom<ValType> for Ty {
    type Error = anyhow::Error;
    fn try_from(t: ValType) -> Result<Self> {
        Ok(match t {
            ValType::I32 => Self::I32,
            ValType::I64 => Self::I64,
            ValType::F32 => Self::F32,
            ValType::F64 => Self::F64,
            _ => bail!("reference/SIMD value type unsupported: {t:?}"),
        })
    }
}

#[derive(Debug, Clone)]
pub enum Node<'a> {
    Structured {
        kind: &'static str,
        params: Vec<Ty>,
        results: Vec<Ty>,
        body: Vec<Node<'a>>,
        other: Vec<Node<'a>>,
    },
    Op(Operator<'a>),
}
#[derive(Debug, Clone)]
pub struct Function<'a> {
    pub tid: usize,
    pub locals: Vec<Ty>,
    pub body: Vec<Node<'a>>,
    pub import: Option<(String, String)>,
}
#[derive(Default, Debug, Clone)]
pub struct Module<'a> {
    pub types: Vec<(Vec<Ty>, Vec<Ty>)>,
    pub funcs: Vec<Function<'a>>,
    pub globals: Vec<(Ty, Num)>,
    pub exports: BTreeMap<String, usize>,
    pub table: Vec<Option<usize>>,
    pub data: Vec<(usize, Vec<u8>)>,
    pub pages: u32,
    pub max_pages: u32,
    pub start: Option<usize>,
    pub global_bits: Vec<Option<u64>>,
    pub declared_max_pages: Option<u64>,
    pub accurate: bool,
    pub helper_start: Option<usize>,
    pub helper_global_start: Option<usize>,
    pub original_types: Vec<(Vec<Ty>, Vec<Ty>)>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Num {
    I(i64),
    F(f64),
}
impl Num {
    pub fn i(self) -> i64 {
        match self {
            Self::I(v) => v,
            Self::F(v) => v as i64,
        }
    }
    pub fn f(self) -> f64 {
        match self {
            Self::I(v) => v as f64,
            Self::F(v) => v,
        }
    }
}

fn constant(e: wasmparser::ConstExpr<'_>) -> Result<Num> {
    let mut r = e.get_operators_reader();
    let v = match r.read()? {
        Operator::I32Const { value } => Num::I(value as i64),
        Operator::I64Const { value } => Num::I(value),
        Operator::F32Const { value } => Num::F(f32::from_bits(value.bits()) as f64),
        Operator::F64Const { value } => Num::F(f64::from_bits(value.bits())),
        x => bail!("unsupported constant initializer {x:?}"),
    };
    ensure!(
        matches!(r.read()?, Operator::End),
        "extended constant expression unsupported"
    );
    Ok(v)
}
fn tree<'a>(
    r: &mut wasmparser::OperatorsReader<'a>,
    types: &[(Vec<Ty>, Vec<Ty>)],
) -> Result<(Vec<Node<'a>>, bool)> {
    let mut nodes = Vec::new();
    loop {
        let op = r.read()?;
        match op {
            Operator::End => return Ok((nodes, false)),
            Operator::Else => return Ok((nodes, true)),
            Operator::Block { blockty } | Operator::Loop { blockty } | Operator::If { blockty } => {
                let kind = match op {
                    Operator::Block { .. } => "block",
                    Operator::Loop { .. } => "loop",
                    _ => "if",
                };
                let (params, results) = match blockty {
                    BlockType::Empty => (vec![], vec![]),
                    BlockType::Type(t) => (vec![], vec![t.try_into()?]),
                    BlockType::FuncType(i) => types[i as usize].clone(),
                };
                let (body, has_else) = tree(r, types)?;
                let other = if has_else {
                    ensure!(kind == "if", "else outside if");
                    tree(r, types)?.0
                } else {
                    vec![]
                };
                nodes.push(Node::Structured {
                    kind,
                    params,
                    results,
                    body,
                    other,
                });
            }
            _ => nodes.push(Node::Op(op)),
        }
    }
}
impl<'a> Module<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        Validator::new().validate_all(bytes)?;
        let mut m = Self {
            max_pages: 256,
            ..Default::default()
        };
        let mut tids = Vec::new();
        let mut code = 0;
        for payload in Parser::new(0).parse_all(bytes) {
            match payload? {
                Payload::Version { .. }
                | Payload::End(_)
                | Payload::CustomSection(_)
                | Payload::DataCountSection { .. } => {}
                Payload::TypeSection(r) => {
                    for ty in r {
                        let Type::Func(f) = ty?;
                        m.types.push((
                            f.params()
                                .iter()
                                .map(|x| (*x).try_into())
                                .collect::<Result<_>>()?,
                            f.results()
                                .iter()
                                .map(|x| (*x).try_into())
                                .collect::<Result<_>>()?,
                        ));
                    }
                }
                Payload::ImportSection(r) => {
                    for i in r {
                        let i = i?;
                        match i.ty {
                            TypeRef::Func(tid) => m.funcs.push(Function {
                                tid: tid as usize,
                                locals: vec![],
                                body: vec![],
                                import: Some((i.module.into(), i.name.into())),
                            }),
                            _ => bail!("only function imports supported"),
                        }
                    }
                }
                Payload::FunctionSection(r) => {
                    tids = r
                        .into_iter()
                        .map(|x| x.map(|i| i as usize))
                        .collect::<std::result::Result<_, _>>()?
                }
                Payload::MemorySection(r) => {
                    ensure!(r.count() == 1, "one memory supported");
                    for mem in r {
                        let mem = mem?;
                        ensure!(!mem.memory64 && !mem.shared, "shared/memory64 unsupported");
                        m.pages = mem.initial as u32;
                        m.declared_max_pages = mem.maximum;
                        m.max_pages = mem.maximum.unwrap_or(256).min(32767) as u32;
                    }
                }
                Payload::TableSection(r) => {
                    ensure!(r.count() == 1, "one table supported");
                    for t in r {
                        let t = t?;
                        ensure!(
                            t.ty.element_type == wasmparser::RefType::FUNCREF,
                            "funcref table required"
                        );
                        m.table.resize(t.ty.initial as usize, None);
                    }
                }
                Payload::GlobalSection(r) => {
                    for g in r {
                        let g = g?;
                        m.global_bits
                            .push(match g.init_expr.get_operators_reader().read()? {
                                Operator::F32Const { value } => Some(value.bits() as u64),
                                Operator::F64Const { value } => Some(value.bits()),
                                _ => None,
                            });
                        m.globals
                            .push((g.ty.content_type.try_into()?, constant(g.init_expr)?));
                    }
                }
                Payload::ExportSection(r) => {
                    for e in r {
                        let e = e?;
                        if e.kind == ExternalKind::Func {
                            m.exports.insert(e.name.into(), e.index as usize);
                        }
                    }
                }
                Payload::StartSection { func, .. } => m.start = Some(func as usize),
                Payload::ElementSection(r) => {
                    for e in r {
                        let e = e?;
                        match (e.kind, e.items) {
                            (
                                ElementKind::Active {
                                    table_index,
                                    offset_expr,
                                },
                                ElementItems::Functions(fs),
                            ) => {
                                ensure!(table_index.unwrap_or(0) == 0, "one table supported");
                                let offset = constant(offset_expr)?.i() as usize;
                                for (i, f) in fs.into_iter().enumerate() {
                                    ensure!(offset + i < m.table.len(), "element out of bounds");
                                    m.table[offset + i] = Some(f? as usize);
                                }
                            }
                            _ => bail!("only active function index table segments supported"),
                        }
                    }
                }
                Payload::CodeSectionStart { .. } => {}
                Payload::CodeSectionEntry(b) => {
                    let mut locals = vec![];
                    for l in b.get_locals_reader()? {
                        let (n, t) = l?;
                        locals.extend(std::iter::repeat(Ty::try_from(t)?).take(n as usize));
                    }
                    let (body, _) = tree(&mut b.get_operators_reader()?, &m.types)?;
                    m.funcs.push(Function {
                        tid: tids[code],
                        locals,
                        body,
                        import: None,
                    });
                    code += 1;
                }
                Payload::DataSection(r) => {
                    for d in r {
                        let d = d?;
                        match d.kind {
                            DataKind::Active {
                                memory_index,
                                offset_expr,
                            } => {
                                ensure!(memory_index == 0, "one memory supported");
                                m.data
                                    .push((constant(offset_expr)?.i() as usize, d.data.to_vec()));
                            }
                            _ => bail!("passive data segments unsupported"),
                        }
                    }
                }
                x => bail!("unsupported WASM section {x:?}"),
            }
        }
        Ok(m)
    }
    pub fn signature(&self, f: usize) -> &(Vec<Ty>, Vec<Ty>) {
        &self.types[self.funcs[f].tid]
    }
    pub fn callees(&self, f: usize) -> BTreeSet<usize> {
        fn walk(nodes: &[Node<'_>], out: &mut BTreeSet<usize>, table: &[Option<usize>]) {
            for n in nodes {
                match n {
                    Node::Structured { body, other, .. } => {
                        walk(body, out, table);
                        walk(other, out, table);
                    }
                    Node::Op(Operator::Call { function_index }) => {
                        out.insert(*function_index as usize);
                    }
                    Node::Op(Operator::CallIndirect { .. }) => {
                        out.extend(table.iter().flatten().copied())
                    }
                    _ => {}
                }
            }
        }
        let mut out = BTreeSet::new();
        walk(&self.funcs[f].body, &mut out, &self.table);
        out
    }
}

/// Numeric operators have no immediates. Convert wasmparser's variant name to
/// the canonical WASM spelling; field-bearing/unsupported operators are rejected.
pub fn numeric_name(op: &Operator<'_>) -> Result<String> {
    let s = format!("{op:?}");
    ensure!(
        !s.contains(' ') && s.len() > 3,
        "unsupported instruction {s}"
    );
    let prefix = s[..3].to_lowercase();
    ensure!(
        ["i32", "i64", "f32", "f64"].contains(&prefix.as_str()),
        "unsupported instruction {s}"
    );
    let mut out = prefix + ".";
    for (i, c) in s[3..].chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    Ok(out)
}
