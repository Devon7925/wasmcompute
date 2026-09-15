//! Integer-bit lowering for accurate vanilla compilation. The embedded library
//! is itself compiled to mcfunctions; no native execution or mod is involved.
use crate::wasm::{numeric_name, Module, Node, Num, Ty};
use anyhow::{bail, ensure, Context, Result};
use std::collections::BTreeMap;
use wasmparser::Operator as O;

pub const HELPER_BASE: u64 = 0x7fff0000;
fn integer(t: Ty) -> Ty {
    match t {
        Ty::F32 => Ty::I32,
        Ty::F64 => Ty::I64,
        t => t,
    }
}

fn relocate(nodes: &mut [Node<'_>], functions: u32, types: u32, globals: u32) {
    for node in nodes {
        match node {
            Node::Structured { body, other, .. } => {
                relocate(body, functions, types, globals);
                relocate(other, functions, types, globals);
            }
            Node::Op(op) => match op {
                O::Call { function_index } => *function_index += functions,
                O::CallIndirect { type_index, .. } => *type_index += types,
                O::GlobalGet { global_index } | O::GlobalSet { global_index } => {
                    *global_index += globals
                }
                _ => {}
            },
        }
    }
}
fn lower_nodes(nodes: &mut [Node<'_>], helpers: &BTreeMap<String, usize>) -> Result<()> {
    for node in nodes {
        match node {
            Node::Structured {
                params,
                results,
                body,
                other,
                ..
            } => {
                params
                    .iter_mut()
                    .chain(results)
                    .for_each(|t| *t = integer(*t));
                lower_nodes(body, helpers)?;
                lower_nodes(other, helpers)?;
            }
            Node::Op(op) => {
                let replacement = match op {
                    O::F32Const { value } => Some(O::I32Const {
                        value: value.bits() as i32,
                    }),
                    O::F64Const { value } => Some(O::I64Const {
                        value: value.bits() as i64,
                    }),
                    O::F32Load { memarg } => Some(O::I32Load { memarg: *memarg }),
                    O::F64Load { memarg } => Some(O::I64Load { memarg: *memarg }),
                    O::F32Store { memarg } => Some(O::I32Store { memarg: *memarg }),
                    O::F64Store { memarg } => Some(O::I64Store { memarg: *memarg }),
                    O::F32ReinterpretI32
                    | O::I32ReinterpretF32
                    | O::F64ReinterpretI64
                    | O::I64ReinterpretF64 => Some(O::Nop),
                    _ => {
                        let text = format!("{op:?}");
                        if text.starts_with("F32")
                            || text.starts_with("F64")
                            || text.starts_with("I32TruncF")
                            || text.starts_with("I64TruncF")
                            || text.starts_with("I32TruncSatF")
                            || text.starts_with("I64TruncSatF")
                        {
                            let name = numeric_name(op)?.replace('.', "_");
                            Some(O::Call {
                                function_index: *helpers.get(&name).with_context(|| {
                                    format!("accurate operation not implemented: {name}")
                                })? as u32,
                            })
                        } else {
                            None
                        }
                    }
                };
                if let Some(value) = replacement {
                    *op = value;
                }
            }
        }
    }
    Ok(())
}

pub fn lower<'a>(input: &Module<'a>) -> Result<Module<'a>> {
    ensure!(!input.accurate, "module already lowered");
    ensure!(
        input.pages <= 32767,
        "accurate runtime supports initial memory up to 32767 pages"
    );
    for (offset, data) in &input.data {
        ensure!(
            (*offset as u64) + data.len() as u64 <= input.pages as u64 * 65536,
            "data segment exceeds initial memory"
        );
    }
    let mut m = input.clone();
    let mut library = Module::parse(include_bytes!("accurate_runtime.wasm"))?;
    ensure!(
        library.table.iter().all(Option::is_none)
            && library.start.is_none()
            && library.funcs.iter().all(|f| f.import.is_none()),
        "invalid embedded accurate library"
    );
    let first = m.funcs.len();
    let tid = m.types.len();
    let gid = m.globals.len();
    ensure!(
        library.globals.len() == 1 && library.globals[0].0 == Ty::I32,
        "embedded helper must have one private stack global"
    );
    let helpers: BTreeMap<_, _> = library
        .exports
        .iter()
        .map(|(k, v)| (k.clone(), first + v))
        .collect();
    for f in &mut library.funcs {
        f.tid += tid;
        relocate(&mut f.body, first as u32, tid as u32, gid as u32);
    }
    for fid in 0..first {
        if let Some((module, name)) = m.funcs[fid].import.clone() {
            let signature = m.signature(fid);
            if signature.0.iter().chain(&signature.1).any(|t| t.float()) {
                let helper=match name.as_str() {
                    "sinf"=>"host_sinf", "fmodf"=>"host_fmodf", "fminf"=>"host_fminf", "fmaxf"=>"host_fmaxf", "sqrtf"=>"f32_sqrt", "fabsf"=>"f32_abs",
                    _=>bail!("accurate floating host import {module}.{name} is not implemented (core Wasm float operations are supported)"),
                };
                let target = helpers[helper];
                let helper_sig = &library.types[library.funcs[target - first].tid - tid];
                ensure!(
                    signature.0.iter().copied().map(integer).collect::<Vec<_>>() == helper_sig.0
                        && signature.1.iter().copied().map(integer).collect::<Vec<_>>()
                            == helper_sig.1,
                    "accurate math import signature mismatch"
                );
                let mut body = (0..signature.0.len())
                    .map(|i| {
                        Node::Op(O::LocalGet {
                            local_index: i as u32,
                        })
                    })
                    .collect::<Vec<_>>();
                body.push(Node::Op(O::Call {
                    function_index: target as u32,
                }));
                m.funcs[fid].import = None;
                m.funcs[fid].body = body;
            }
        }
    }
    m.types.extend(library.types);
    m.funcs.extend(library.funcs);
    m.globals.extend(library.globals);
    m.global_bits.extend(library.global_bits);
    ensure!(
        library
            .data
            .iter()
            .all(|(p, d)| *p as u64 >= HELPER_BASE && *p as u64 + d.len() as u64 <= 0x80000000),
        "helper data outside reserved region"
    );
    m.data.extend(library.data);
    m.original_types = m.types.clone();
    for (params, results) in &mut m.types {
        params
            .iter_mut()
            .chain(results)
            .for_each(|t| *t = integer(*t));
    }
    for (index, (ty, value)) in m.globals.iter_mut().enumerate() {
        if ty.float() {
            let bits = m
                .global_bits
                .get(index)
                .copied()
                .flatten()
                .unwrap_or_else(|| {
                    if *ty == Ty::F32 {
                        (value.f() as f32).to_bits() as u64
                    } else {
                        value.f().to_bits()
                    }
                });
            *value = Num::I(if *ty == Ty::F32 {
                bits as i32 as i64
            } else {
                bits as i64
            });
            *ty = integer(*ty);
        }
    }
    for f in &mut m.funcs {
        f.locals.iter_mut().for_each(|t| *t = integer(*t));
        lower_nodes(&mut f.body, &helpers)?;
    }
    m.accurate = true;
    m.helper_start = Some(first);
    m.helper_global_start = Some(gid);
    m.max_pages = m.declared_max_pages.unwrap_or(32767).min(32767) as u32;
    Ok(m)
}
