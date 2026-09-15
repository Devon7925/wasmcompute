//! Runtime helpers emitted as vanilla commands, never native plugins or mods.
use crate::backend::{Backend, Val};
use crate::wasm::{Num, Ty};
use anyhow::Result;
use serde_json::json;

fn save(b: &mut Backend<'_, '_>, name: &str) {
    let lines = std::mem::take(&mut b.lines);
    b.function(&format!("rt/{name}"), lines);
}
fn raw(b: &mut Backend<'_, '_>, s: &str) {
    b.emit(
        s.replace("{o}", &b.obj)
            .replace("{s}", &b.storage)
            .replace("{m}", &b.mem)
            .replace("{n}", &b.ns),
    );
}
fn raw_lines(b: &mut Backend<'_, '_>, s: &str) {
    for l in s.lines() {
        if !l.trim().is_empty() {
            raw(b, l.trim());
        }
    }
}
fn op_const(b: &mut Backend<'_, '_>, d: &str, op: &str, n: i64) {
    let c = b.c(n);
    b.operation(d, op, &c);
}
fn float_expr(b: &mut Backend<'_, '_>, dst: &str, op: &str, args: Vec<Val>) -> Result<()> {
    b.materialize(
        &Val::Expr(Ty::F32, op.into(), args),
        Some(Val::reg(Ty::F32, dst)),
    )?;
    Ok(())
}
fn fr(s: &str) -> Val {
    Val::reg(Ty::F32, s)
}
fn fc(n: f64) -> Val {
    Val::Const(Ty::F32, Num::F(n))
}
fn ir(s: &str) -> Val {
    Val::reg(Ty::I32, s)
}
fn from_int(s: &str) -> Val {
    Val::Expr(Ty::F32, "from_int".into(), vec![ir(s)])
}
fn ie(b: &mut Backend<'_, '_>, dest: &str, p: serde_json::Value) {
    b.emit(format!(
        "execute store result score {} run compute default integer {p}",
        b.score(dest)
    ));
}

pub fn generate(b: &mut Backend<'_, '_>) -> Result<()> {
    b.lines.clear();
    for (name, code) in [("memory", 3), ("integer", 4)] {
        b.function(
            &format!("rt/trap_{name}"),
            vec![
                format!("scoreboard players set {} {code}", b.score("trap")),
                "return fail".into(),
            ],
        );
    }
    comparisons(b)?;
    integers32(b)?;
    integers64(b)?;
    memory(b)?;
    floats(b)?;
    host(b)?;
    let utf8 = b.m.funcs.iter().any(|f| {
        f.import
            .as_ref()
            .map_or(false, |(_, name)| name == "mc_command_utf8_result")
    });
    if utf8 {
        utf8_commands(b)?;
    }
    b.lines.clear();
    // Character lookup is ASCII/UTF-8 byte oriented. Commands reject NUL/newline.
    let chars = (0..256)
        .map(|i| {
            if i < 128 {
                char::from_u32(i).unwrap().to_string()
            } else {
                "?".into()
            }
        })
        .collect::<Vec<_>>();
    b.emit(format!(
        "data modify storage {} chars set value {}",
        b.storage,
        serde_json::to_string(&chars)?
    ));
    b.emit(format!(
        "data modify storage {} host set value {{cmd:\"\",ch:\"\"}}",
        b.storage
    ));
    if utf8 {
        b.emit(format!(
            "data modify storage {} hex set value {}",
            b.storage,
            serde_json::to_string(
                &"0123456789abcdef"
                    .chars()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
            )?
        ));
    }
    for name in ["and", "or", "xor"] {
        let mut table = serde_json::Map::new();
        for a in 0..16 {
            for c in 0..16 {
                table.insert(
                    format!("a{a}b{c}"),
                    json!(match name {
                        "and" => a & c,
                        "or" => a | c,
                        _ => a ^ c,
                    }),
                );
            }
        }
        b.emit(format!(
            "data modify storage {}:lookup {name} set value {}",
            b.ns,
            serde_json::Value::Object(table)
        ));
    }
    save(b, "init");
    Ok(())
}
fn comparisons(b: &mut Backend<'_, '_>) -> Result<()> {
    for (name, cmp) in [
        ("eq", "="),
        ("ne", "="),
        ("lt_s", "<"),
        ("gt_s", ">"),
        ("le_s", "<="),
        ("ge_s", ">="),
    ] {
        b.set("r", 0);
        raw(
            b,
            &format!(
                "execute {} score #a {{o}} {cmp} #b {{o}} run scoreboard players set #r {{o}} 1",
                if name == "ne" { "unless" } else { "if" }
            ),
        );
        save(b, &format!("i32_{name}"));
    }
    for (name, cmp, opposite) in [
        ("lt_u", "<", false),
        ("gt_u", ">", true),
        ("le_u", "<=", false),
        ("ge_u", ">=", true),
    ] {
        b.set("r", 0);
        raw(
            b,
            &format!(
                "execute if score #a {{o}} {cmp} #b {{o}} run scoreboard players set #r {{o}} 1"
            ),
        );
        raw(b,&format!("execute if score #a {{o}} matches ..-1 if score #b {{o}} matches 0.. run scoreboard players set #r {{o}} {}",if opposite{1}else{0}));
        raw(b,&format!("execute if score #b {{o}} matches ..-1 if score #a {{o}} matches 0.. run scoreboard players set #r {{o}} {}",if opposite{0}else{1}));
        save(b, &format!("i32_{name}"));
    }
    for name in ["eq", "ne", "lt", "gt", "le", "ge"] {
        let a = b.provider(&fr("fa"))?;
        let other = b.provider(&fr("fb"))?;
        let test = match name {
            "eq" | "ne" => other,
            "lt" | "ge" => json!({"min":other}),
            _ => json!({"max":other}),
        };
        let pred = json!({"type":"minecraft:float_value_check","value":a,"test":test});
        b.set("r", 0);
        b.emit(format!(
            "execute {} predicate {pred} run scoreboard players set {} 1",
            if ["ne", "lt", "gt"].contains(&name) {
                "unless"
            } else {
                "if"
            },
            b.score("r")
        ));
        save(b, &format!("fcmp_{name}"));
    }
    Ok(())
}
fn integers32(b: &mut Backend<'_, '_>) -> Result<()> {
    for bits in [8, 16, 32] {
        b.cp("r", "a");
        if bits < 32 {
            op_const(b, "r", "%=", 1 << bits);
            raw(b,&format!("execute if score #r {{o}} matches {}.. run scoreboard players remove #r {{o}} {}",1<<(bits-1),1<<bits));
        }
        save(b, &format!("i32_extend{bits}_s"));
    }
    // Constant-count shifts avoid table/macro overhead and preserve all 32 bits.
    for k in 0..32 {
        b.cp("r", "a");
        if k > 0 {
            op_const(b, "r", "*=", 1i64 << k);
        }
        save(b, &format!("shl/{k}"));
        b.cp("r", "a");
        if k == 31 {
            b.set("r", 0);
            raw(
                b,
                "execute if score #a {o} matches ..-1 run scoreboard players set #r {o} -1",
            );
        } else if k > 0 {
            op_const(b, "r", "/=", 1 << k);
        }
        save(b, &format!("shr_s/{k}"));
        b.cp("r", "a");
        if k == 31 {
            b.set("r", 0);
            raw(
                b,
                "execute if score #a {o} matches ..-1 run scoreboard players set #r {o} 1",
            );
        } else if k > 0 {
            op_const(b, "r", "/=", 1 << k);
            let c = b.c(1i64 << (32 - k));
            raw(b,&format!("execute if score #a {{o}} matches ..-1 run scoreboard players operation #r {{o}} += #{c} {{o}}"));
        }
        save(b, &format!("shr_u/{k}"));
    }
    for name in ["shl", "shr_s", "shr_u"] {
        op_const(b, "b", "%=", 32);
        raw(
            b,
            "execute store result storage {s} macro.k int 1 run scoreboard players get #b {o}",
        );
        raw(
            b,
            &format!("return run function {{n}}:rt/{name}_macro with storage {{s}} macro"),
        );
        save(b, &format!("i32_{name}"));
        raw(b, &format!("$return run function {{n}}:rt/{name}/$(k)"));
        save(b, &format!("{name}_macro"));
    }
    // Native integer provider evaluation replaces all nibble-table macros.
    for name in ["and", "or", "xor"] {
        let p = crate::bitwise::provider(
            name,
            b.provider(&Val::reg(Ty::I32, "a"))?,
            b.provider(&Val::reg(Ty::I32, "b"))?,
        );
        b.emit(format!(
            "execute store result score {} run compute default integer {p}",
            b.score("r")
        ));
        save(b, &format!("i32_{name}"));
    }
    for name in ["rotl", "rotr"] {
        b.cp("rotate_a", "a");
        b.cp("rotate_b", "b");
        b.rt(if name == "rotl" {
            "i32_shl"
        } else {
            "i32_shr_u"
        });
        b.cp("rotate_lo", "r");
        b.cp("a", "rotate_a");
        b.set("b", 32);
        b.operation("b", "-=", "rotate_b");
        op_const(b, "b", "%=", 32);
        b.rt(if name == "rotl" {
            "i32_shr_u"
        } else {
            "i32_shl"
        });
        raw(b,"execute unless score #b {o} matches 0 run scoreboard players operation #r {o} += #rotate_lo {o}");
        save(b, &format!("i32_{name}"));
    }
    for name in ["clz", "ctz", "popcnt"] {
        b.set("r", if name == "popcnt" { 0 } else { 32 });
        b.cp("bits", "a");
        if name == "popcnt" {
            for _ in 0..32 {
                b.cp("bit", "bits");
                op_const(b, "bit", "%=", 2);
                b.operation("r", "+=", "bit");
                op_const(b, "bits", "/=", 2);
            }
        } else {
            for i in 0..32 {
                let bit = if name == "clz" { 31 - i } else { i };
                b.cp("bit", "a");
                if bit == 31 {
                    b.set("bit", 0);
                    raw(b,"execute if score #a {o} matches ..-1 run scoreboard players set #bit {o} 1");
                } else {
                    if bit > 0 {
                        op_const(b, "bit", "/=", 1 << bit);
                    }
                    op_const(b, "bit", "%=", 2);
                }
                raw(b,&format!("execute if score #r {{o}} matches 32 if score #bit {{o}} matches 1 run scoreboard players set #r {{o}} {i}"));
            }
        }
        save(b, &format!("i32_{name}"));
    }
    // Unsigned division by restoring long division; no floating precision loss.
    raw_lines(b,"scoreboard players set #q {o} 0\nscoreboard players set #rem {o} 0\nscoreboard players operation #dividend {o} = #a {o}\nscoreboard players operation #divisor {o} = #b {o}");
    for k in (0..32).rev() {
        b.cp("a", "dividend");
        b.set("b", k);
        b.rt("i32_shr_u");
        op_const(b, "r", "%=", 2);
        op_const(b, "rem", "*=", 2);
        b.operation("rem", "+=", "r");
        b.cp("a", "rem");
        b.cp("b", "divisor");
        b.rt("i32_ge_u");
        raw(b,"execute if score #r {o} matches 1 run scoreboard players operation #rem {o} -= #divisor {o}");
        let c = b.c(1i64 << k);
        raw(b,&format!("execute if score #r {{o}} matches 1 run scoreboard players operation #q {{o}} += #{c} {{o}}"));
    }
    save(b, "udiv32");
    b.rt("udiv32");
    b.cp("r", "q");
    save(b, "i32_div_u");
    b.rt("udiv32");
    b.cp("r", "rem");
    save(b, "i32_rem_u");
    Ok(())
}

fn integers64(b: &mut Backend<'_, '_>) -> Result<()> {
    b.rt("i64_add");
    b.cp("a", "r");
    b.cp("ah", "rh");
    save(b, "div_pow2_bias");
    for name in ["add", "sub"] {
        b.cp("lo", "a");
        b.cp("hi", "ah");
        b.operation("lo", if name == "add" { "+=" } else { "-=" }, "b");
        b.operation("hi", if name == "add" { "+=" } else { "-=" }, "bh");
        if name == "add" {
            b.cp("b", "a");
            b.cp("a", "lo");
        } // carry: sum < a; borrow: a < b
        b.rt("i32_lt_u");
        raw(
            b,
            &format!(
                "execute if score #r {{o}} matches 1 run scoreboard players {} #hi {{o}} 1",
                if name == "add" { "add" } else { "remove" }
            ),
        );
        b.cp("r", "lo");
        b.cp("rh", "hi");
        save(b, &format!("i64_{name}"));
    }
    for name in ["and", "or", "xor"] {
        b.cp("save_ah", "ah");
        b.cp("save_bh", "bh");
        b.rt(&format!("i32_{name}"));
        b.cp("save_lo", "r");
        b.cp("a", "save_ah");
        b.cp("b", "save_bh");
        b.rt(&format!("i32_{name}"));
        b.cp("rh", "r");
        b.cp("r", "save_lo");
        save(b, &format!("i64_{name}"));
    }
    for name in [
        "eq", "ne", "lt_s", "lt_u", "gt_s", "gt_u", "le_s", "le_u", "ge_s", "ge_u",
    ] {
        b.cp("cmp_a", "a");
        b.cp("cmp_b", "b");
        b.cp("cmp_ah", "ah");
        b.cp("cmp_bh", "bh");
        b.cp("a", "ah");
        b.cp("b", "bh");
        b.rt(&format!("i32_{name}"));
        b.cp("cmp_high", "r");
        b.cp("a", "cmp_a");
        b.cp("b", "cmp_b");
        let low = if name.ends_with("_s") {
            name.replace("_s", "_u")
        } else {
            name.into()
        };
        b.rt(&format!("i32_{low}"));
        if name == "eq" {
            raw(b,"execute unless score #cmp_ah {o} = #cmp_bh {o} run scoreboard players set #r {o} 0");
        } else if name == "ne" {
            raw(b,"execute unless score #cmp_ah {o} = #cmp_bh {o} run scoreboard players set #r {o} 1");
        } else {
            raw(b,"execute unless score #cmp_ah {o} = #cmp_bh {o} run scoreboard players operation #r {o} = #cmp_high {o}");
        }
        save(b, &format!("i64_{name}"));
    }
    // 16-bit limbs: product terms remain representable modulo 2^32. Carry uses
    // logical shifts; this is exact integer arithmetic, including RNG overflow.
    for (k, src, shift) in [
        ("a0", "a", false),
        ("a1", "a", true),
        ("a2", "ah", false),
        ("a3", "ah", true),
        ("b0", "b", false),
        ("b1", "b", true),
        ("b2", "bh", false),
        ("b3", "bh", true),
    ] {
        b.cp(k, src);
        if shift {
            op_const(b, k, "/=", 65536);
        }
        op_const(b, k, "%=", 65536);
    }
    b.set("r", 0);
    b.set("rh", 0);
    b.set("carry", 0);
    for k in 0..4 {
        b.cp("limb", "carry");
        b.set("carry", 0);
        for i in 0..=k {
            let j = k - i;
            b.cp("prod", &format!("a{i}"));
            b.operation("prod", "*=", &format!("b{j}"));
            b.cp("pl", "prod");
            op_const(b, "pl", "%=", 65536);
            b.operation("limb", "+=", "pl");
            b.cp("ph", "prod");
            op_const(b, "ph", "/=", 65536);
            op_const(b, "ph", "%=", 65536);
            b.operation("carry", "+=", "ph");
        }
        b.cp("excess", "limb");
        op_const(b, "excess", "/=", 65536);
        b.operation("carry", "+=", "excess");
        op_const(b, "limb", "%=", 65536);
        if k % 2 == 1 {
            op_const(b, "limb", "*=", 65536);
        }
        b.operation(if k < 2 { "r" } else { "rh" }, "+=", "limb");
    }
    save(b, "i64_mul");
    for bits in [8, 16, 32] {
        b.rt(&format!("i32_extend{bits}_s"));
        b.set("rh", 0);
        raw(
            b,
            "execute if score #r {o} matches ..-1 run scoreboard players set #rh {o} -1",
        );
        save(b, &format!("i64_extend{bits}_s"));
    }
    for name in ["shl", "shr_s", "shr_u"] {
        for k in 0..64 {
            if k == 0 {
                b.cp("r", "a");
                b.cp("rh", "ah");
            } else if name == "shl" {
                if k < 32 {
                    b.cp("shift_a", "a");
                    b.cp("shift_ah", "ah");
                    b.rt(&format!("shl/{k}"));
                    b.cp("shift_low", "r");
                    b.cp("a", "shift_ah");
                    b.rt(&format!("shl/{k}"));
                    b.cp("shift_hi", "r");
                    b.cp("a", "shift_a");
                    b.rt(&format!("shr_u/{}", 32 - k));
                    b.operation("shift_hi", "+=", "r");
                    b.cp("r", "shift_low");
                    b.cp("rh", "shift_hi");
                } else {
                    b.cp("a", "a");
                    b.rt(&format!("shl/{}", k - 32));
                    b.cp("rh", "r");
                    b.set("r", 0);
                }
            } else if k < 32 {
                b.cp("shift_a", "a");
                b.cp("shift_ah", "ah");
                b.rt(&format!("shr_u/{k}"));
                b.cp("shift_low", "r");
                b.cp("a", "shift_ah");
                b.rt(&format!("shl/{}", 32 - k));
                b.operation("shift_low", "+=", "r");
                b.cp("a", "shift_ah");
                b.rt(&format!("{name}/{k}"));
                b.cp("rh", "r");
                b.cp("r", "shift_low");
            } else {
                b.cp("a", "ah");
                b.rt(&format!("{name}/{}", k - 32));
                b.set("rh", 0);
                if name == "shr_s" {
                    raw(b,"execute if score #ah {o} matches ..-1 run scoreboard players set #rh {o} -1");
                }
            }
            save(b, &format!("i64_{name}/{k}"));
        }
        op_const(b, "b", "%=", 64);
        raw(
            b,
            "execute store result storage {s} macro.k int 1 run scoreboard players get #b {o}",
        );
        raw(
            b,
            &format!("return run function {{n}}:rt/i64_{name}_macro with storage {{s}} macro"),
        );
        save(b, &format!("i64_{name}"));
        raw(b, &format!("$return run function {{n}}:rt/i64_{name}/$(k)"));
        save(b, &format!("i64_{name}_macro"));
    }
    // Restoring 64-bit unsigned division with the 65th remainder bit retained.
    // Quotient/remainder are exact; signed wrappers normalize magnitudes first.
    b.cp("dv_al", "a");
    b.cp("dv_ah", "ah");
    b.cp("dv_bl", "b");
    b.cp("dv_bh", "bh");
    for n in ["dv_ql", "dv_qh", "dv_rl", "dv_rh"] {
        b.set(n, 0);
    }
    for _ in 0..64 {
        b.set("dv_over", 0);
        raw(
            b,
            "execute if score #dv_rh {o} matches ..-1 run scoreboard players set #dv_over {o} 1",
        );
        op_const(b, "dv_rh", "*=", 2);
        raw(
            b,
            "execute if score #dv_rl {o} matches ..-1 run scoreboard players add #dv_rh {o} 1",
        );
        op_const(b, "dv_rl", "*=", 2);
        raw(
            b,
            "execute if score #dv_ah {o} matches ..-1 run scoreboard players add #dv_rl {o} 1",
        );
        op_const(b, "dv_ah", "*=", 2);
        raw(
            b,
            "execute if score #dv_al {o} matches ..-1 run scoreboard players add #dv_ah {o} 1",
        );
        op_const(b, "dv_al", "*=", 2);
        op_const(b, "dv_qh", "*=", 2);
        raw(
            b,
            "execute if score #dv_ql {o} matches ..-1 run scoreboard players add #dv_qh {o} 1",
        );
        op_const(b, "dv_ql", "*=", 2);
        b.cp("a", "dv_rl");
        b.cp("ah", "dv_rh");
        b.cp("b", "dv_bl");
        b.cp("bh", "dv_bh");
        b.rt("i64_ge_u");
        raw(
            b,
            "execute if score #dv_over {o} matches 1 run scoreboard players set #r {o} 1",
        );
        raw(
            b,
            "execute if score #r {o} matches 1 run function {n}:rt/div64_sub",
        );
    }
    save(b, "udiv64");
    b.cp("a", "dv_rl");
    b.cp("ah", "dv_rh");
    b.cp("b", "dv_bl");
    b.cp("bh", "dv_bh");
    b.rt("i64_sub");
    b.cp("dv_rl", "r");
    b.cp("dv_rh", "rh");
    raw(b, "scoreboard players add #dv_ql {o} 1");
    save(b, "div64_sub");
    op_const(b, "r", "*=", -1);
    op_const(b, "rh", "*=", -1);
    raw(
        b,
        "execute unless score #r {o} matches 0 run scoreboard players remove #rh {o} 1",
    );
    save(b, "neg64_r");
    for name in ["div_u", "rem_u", "div_s", "rem_s"] {
        if name.ends_with("_s") {
            b.cp("signed_al", "a");
            b.cp("signed_ah", "ah");
            b.cp("signed_bl", "b");
            b.cp("signed_bh", "bh");
            b.cp("r", "a");
            b.cp("rh", "ah");
            raw(
                b,
                "execute if score #signed_ah {o} matches ..-1 run function {n}:rt/neg64_r",
            );
            b.cp("abs_al", "r");
            b.cp("abs_ah", "rh");
            b.cp("r", "signed_bl");
            b.cp("rh", "signed_bh");
            raw(
                b,
                "execute if score #signed_bh {o} matches ..-1 run function {n}:rt/neg64_r",
            );
            b.cp("b", "r");
            b.cp("bh", "rh");
            b.cp("a", "abs_al");
            b.cp("ah", "abs_ah");
        }
        b.rt("udiv64");
        b.cp(
            "r",
            if name.starts_with("div") {
                "dv_ql"
            } else {
                "dv_rl"
            },
        );
        b.cp(
            "rh",
            if name.starts_with("div") {
                "dv_qh"
            } else {
                "dv_rh"
            },
        );
        if name == "rem_s" {
            raw(
                b,
                "execute if score #signed_ah {o} matches ..-1 run function {n}:rt/neg64_r",
            );
        }
        if name == "div_s" {
            raw(b,"execute if score #signed_ah {o} matches ..-1 if score #signed_bh {o} matches 0.. run function {n}:rt/neg64_r");
            raw(b,"execute if score #signed_bh {o} matches ..-1 if score #signed_ah {o} matches 0.. run function {n}:rt/neg64_r");
        }
        save(b, &format!("i64_{name}"));
    }
    for name in ["clz", "ctz", "popcnt"] {
        b.cp("count_hi", "ah");
        b.cp("count_lo", "a");
        if name == "clz" {
            b.cp("a", "ah");
        }
        b.rt(&format!("i32_{name}"));
        b.cp("count_first", "r");
        b.cp(
            "a",
            if name == "clz" {
                "count_lo"
            } else {
                "count_hi"
            },
        );
        b.rt(&format!("i32_{name}"));
        if name == "popcnt" {
            b.operation("r", "+=", "count_first");
        } else {
            raw(b, "scoreboard players add #r {o} 32");
            raw(b,"execute unless score #count_first {o} matches 32 run scoreboard players operation #r {o} = #count_first {o}");
        }
        b.set("rh", 0);
        save(b, &format!("i64_{name}"));
    }
    for name in ["rotl", "rotr"] {
        b.cp("rot_al", "a");
        b.cp("rot_ah", "ah");
        b.cp("rot_b", "b");
        b.rt(if name == "rotl" {
            "i64_shl"
        } else {
            "i64_shr_u"
        });
        b.cp("rot_lo", "r");
        b.cp("rot_hi", "rh");
        b.cp("a", "rot_al");
        b.cp("ah", "rot_ah");
        b.set("b", 64);
        b.operation("b", "-=", "rot_b");
        op_const(b, "b", "%=", 64);
        b.cp("rot_count", "b");
        b.rt(if name == "rotl" {
            "i64_shr_u"
        } else {
            "i64_shl"
        });
        raw(b,"execute unless score #rot_count {o} matches 0 run scoreboard players operation #r {o} += #rot_lo {o}");
        raw(b,"execute unless score #rot_count {o} matches 0 run scoreboard players operation #rh {o} += #rot_hi {o}");
        save(b, &format!("i64_{name}"));
    }
    Ok(())
}

fn memory(b: &mut Backend<'_, '_>) -> Result<()> {
    // Sparse word compound: absent words read as zero. No block entities, scans,
    // or giant zero-filled NBT arrays. Read/write macros use integer addresses.
    raw(b,"execute store result storage {s} macro.p int 1 run compute default integer {type:floor_div,left:{type:score,target:{type:fixed,name:\"#ptr\"},score:\"{o}\"},right:4}");
    save(b, "address");
    if b.word_buffer() {
        raw(b,"$execute if score #wb_addr {o} matches $(p) run return run scoreboard players get #wb_word {o}");
    }
    if b.float_cache() {
        raw(b,"$execute if data storage {n}:fmem w.a$(p).d run function {n}:rt/flush_float with storage {s} macro");
    }
    if b.score_memory {
        if b.lazy_initial_memory() {
            raw(b, "$execute if score #m$(p) {n}_mem matches -2147483648..2147483647 run return run scoreboard players get #m$(p) {n}_mem");
            raw(b, "$scoreboard players set #rom_address {o} $(p)");
            raw(b, "execute store result storage {s} rom.page int 1 run compute default integer {type:floor_div,left:{type:score,target:{type:fixed,name:\"#rom_address\"},score:\"{o}\"},right:256}");
            raw(b, "execute store result storage {s} rom.index int 1 run compute default integer {type:mod,left:{type:score,target:{type:fixed,name:\"#rom_address\"},score:\"{o}\"},right:256}");
            raw(b, "$execute store result score #m$(p) {n}_mem run function {n}:rt/rom_read with storage {s} rom");
        }
        raw(b, "$return run scoreboard players get #m$(p) {n}_mem");
    } else {
        raw(b, "$return run data get storage {m} w.a$(p)");
    }
    save(b, "word_read");
    if b.lazy_initial_memory() {
        raw(
            b,
            "$return run data get storage {n}:rom pages.p$(page)[$(index)]",
        );
        save(b, "rom_read");
    }
    if b.word_buffer() {
        raw(
            b,
            "$execute unless score #wb_addr {o} matches $(p) run function {n}:rt/wb_flush",
        );
        b.cp("wb_word", "val");
        raw(b, "$scoreboard players set #wb_addr {o} $(p)");
        b.set("wb_dirty", 1);
    } else {
        b.emit(format!("${}", b.write_word("$(p)", "val")));
    }
    if b.float_cache() {
        raw(b, "$data remove storage {n}:fmem w.a$(p)");
    }
    save(b, "word_write");
    raw(b, "execute if score #wb_dirty {o} matches 0 run return 0");
    raw(
        b,
        "execute store result storage {s} flush.p int 1 run scoreboard players get #wb_addr {o}",
    );
    raw(b, "function {n}:rt/wb_flush_macro with storage {s} flush");
    b.set("wb_dirty", 0);
    save(b, "wb_flush");
    b.emit(format!("${}", b.write_word("$(p)", "wb_word")));
    save(b, "wb_flush_macro");
    b.rt("address");
    raw(
        b,
        "execute store result score #r {o} run function {n}:rt/word_read with storage {s} macro",
    );
    save(b, "load_word");
    b.rt("address");
    raw(b, "function {n}:rt/word_write with storage {s} macro");
    save(b, "store_word");
    // A byte operation uses one specialized provider, without a scoreboard RMW
    // sequence. The top byte is signed when reconstructing the word so every
    // intermediate remains within i32 range, including 0x80000000/0xffffffff.
    b.rt("address");
    raw(b,"execute store result storage {s} macro.l int 1 run compute default integer {type:floor_mod,left:{type:score,target:{type:fixed,name:\"#ptr\"},score:\"{o}\"},right:4}");
    save(b, "byte_address");
    b.rt("byte_address");
    raw(b,"execute store result score #byte_word {o} run function {n}:rt/word_read with storage {s} macro");
    raw(b,"execute store result score #r {o} run function {n}:rt/load8_dispatch with storage {s} macro");
    save(b, "load8");
    b.rt("byte_address");
    raw(b,"execute store result score #byte_word {o} run function {n}:rt/word_read with storage {s} macro");
    raw(b,"execute store result score #val {o} run function {n}:rt/store8_dispatch with storage {s} macro");
    raw(b, "function {n}:rt/word_write with storage {s} macro");
    save(b, "store8");
    for kind in ["load", "store"] {
        raw(
            b,
            &format!("$return run function {{n}}:rt/{kind}8_lane_$(l)"),
        );
        save(b, &format!("{kind}8_dispatch"));
        for lane in 0..4 {
            let factor = 1u32 << (lane * 8);
            let word = b.provider(&ir("byte_word"))?;
            let old = json!({"type":"floor_mod","left":{"type":"floor_div","left":word,"right":factor},"right":256});
            let val = json!({"type":"floor_mod","left":b.provider(&ir("val"))?,"right":256});
            if kind == "load" {
                b.emit(format!("return run compute default integer {old}"));
            } else {
                let p = if lane == 3 {
                    json!({"type":"add","inputs":[{"type":"floor_mod","left":word,"right":16777216},{"type":"mul","inputs":[{"type":"sub","left":{"type":"floor_mod","left":{"type":"add","inputs":[val,128]},"right":256},"right":128},16777216]}]})
                } else {
                    json!({"type":"add","inputs":[{"type":"sub","left":word,"right":{"type":"mul","inputs":[old,factor]}},{"type":"mul","inputs":[val,factor]}]})
                };
                b.emit(format!("return run compute default integer {p}"));
            }
            save(b, &format!("{kind}8_lane_{lane}"));
        }
    }
    for width in [16, 32] {
        if width == 32 {
            // Read the first word directly in this helper. The common aligned
            // case avoids an additional function/return transition. Unaligned
            // accesses still use the byte fallback below.
            b.rt("address");
            raw(b,"execute store result score #r {o} run function {n}:rt/word_read with storage {s} macro");
            b.cp("lane", "ptr");
            op_const(b, "lane", "%=", 4);
            raw(b, "execute if score #lane {o} matches 0 run return 0");
        }
        b.set("load_acc", 0);
        b.cp("load_ptr", "ptr");
        for k in 0..width / 8 {
            b.cp("ptr", "load_ptr");
            if k > 0 {
                raw(b, &format!("scoreboard players add #ptr {{o}} {k}"));
            }
            b.rt("load8");
            if k > 0 {
                op_const(b, "r", "*=", 1 << (8 * k));
            }
            b.operation("load_acc", "+=", "r");
        }
        b.cp("r", "load_acc");
        b.cp("ptr", "load_ptr");
        save(b, &format!("load{width}"));
        if width == 32 {
            b.rt("address");
            b.cp("lane", "ptr");
            op_const(b, "lane", "%=", 4);
            raw(
                b,
                "execute if score #lane {o} matches 0 run return run function {n}:rt/word_write with storage {s} macro",
            );
        }
        b.cp("store_value", "val");
        b.cp("store_ptr", "ptr");
        for k in 0..width / 8 {
            b.cp("ptr", "store_ptr");
            if k > 0 {
                raw(b, &format!("scoreboard players add #ptr {{o}} {k}"));
            }
            b.cp("val", "store_value");
            b.rt("store8");
            op_const(b, "store_value", "/=", 256);
        }
        b.cp("ptr", "store_ptr");
        save(b, &format!("store{width}"));
    }
    b.cp("wide_ptr", "ptr");
    b.rt("load32");
    b.cp("wide_low", "r");
    b.cp("ptr", "wide_ptr");
    raw(b, "scoreboard players add #ptr {o} 4");
    b.rt("load32");
    b.cp("rh", "r");
    b.cp("r", "wide_low");
    b.cp("ptr", "wide_ptr");
    save(b, "load64");
    b.cp("wide_ptr", "ptr");
    b.cp("wide_high", "valh");
    b.rt("store32");
    b.cp("ptr", "wide_ptr");
    raw(b, "scoreboard players add #ptr {o} 4");
    b.cp("val", "wide_high");
    b.rt("store32");
    b.cp("ptr", "wide_ptr");
    save(b, "store64");
    b.cp("grow_result", "pages");
    b.cp("new_pages", "pages");
    b.operation("new_pages", "+=", "grow");
    raw(b,&format!("execute unless score #new_pages {{o}} matches 0..{} run scoreboard players set #grow_result {{o}} -1",b.m.max_pages));
    raw(
        b,
        "execute if score #grow {o} matches ..-1 run scoreboard players set #grow_result {o} -1",
    );
    raw(b,"execute unless score #grow_result {o} matches -1 run scoreboard players operation #pages {o} = #new_pages {o}");
    save(b, "grow");
    // memmove direction preserves overlap. Byte loop is the conservative fallback.
    for (i, r) in b.copy_profile.clone().iter().enumerate() {
        raw(b,&format!("execute if score #src {{o}} matches {} if score #dst {{o}} matches {} if score #n {{o}} matches {} run return run function {{n}}:rt/copy_profile/{i}",r.src,r.dst,r.len));
    }
    raw(b, "execute if score #n {o} matches ..0 run return 0");
    raw(b, "execute if score #dst {o} = #src {o} run return 0");
    b.cp("copy_alignment", "dst");
    b.operation("copy_alignment", "-=", "src");
    op_const(b, "copy_alignment", "%=", 4);
    b.set("direction", 1);
    if !b.word_buffer() {
        b.cp("copy_lane", "src");
        op_const(b, "copy_lane", "%=", 4);
        raw(b,"execute if score #dst {o} < #src {o} if score #n {o} matches 4.. if score #copy_alignment {o} matches 0 if score #copy_lane {o} matches 0 run return run function {n}:rt/copy_forward_words");
    }
    raw(
        b,
        "execute if score #dst {o} > #src {o} run function {n}:rt/copy_backwards",
    );
    raw(b, "return run function {n}:rt/copy_loop");
    save(b, "memory_copy");
    for (i, r) in b.copy_profile.clone().iter().enumerate() {
        if b.word_buffer() {
            b.rt("wb_flush");
            b.set("wb_addr", -1);
        }
        for k in 0..r.len / 4 {
            let src = r.src / 4 + k;
            let dst = r.dst / 4 + k;
            if b.score_memory {
                raw(b,&format!("execute store result score #m{dst} {{n}}_mem run scoreboard players get #m{src} {{n}}_mem"));
            } else {
                raw(b,&format!("execute store result storage {{m}} w.a{dst} int 1 run data get storage {{m}} w.a{src}"));
            }
            if b.float_cache() {
                raw_lines(b,&format!("execute if data storage {{n}}:fmem w.a{src} run data modify storage {{n}}:fmem w.a{dst} set from storage {{n}}:fmem w.a{src}\nexecute unless data storage {{n}}:fmem w.a{src} if data storage {{n}}:fmem w.a{dst} run data remove storage {{n}}:fmem w.a{dst}"));
            }
        }
        if r.len % 4 != 0 {
            b.set("src", (r.src + r.len / 4 * 4) as i64);
            b.set("dst", (r.dst + r.len / 4 * 4) as i64);
            b.set("n", (r.len % 4) as i64);
            b.set("direction", 1);
            b.set("copy_alignment", 0);
            raw(b, "return run function {n}:rt/copy_loop");
        }
        save(b, &format!("copy_profile/{i}"));
    }
    // Hoist address division, alignment and direction out of a forward word
    // copy. Copy the typed cache and dirty flags alongside the raw word, so
    // float-containing records never need an encode/decode round trip here.
    b.cp("copy_src_word", "src");
    op_const(b, "copy_src_word", "/=", 4);
    b.cp("copy_dst_word", "dst");
    op_const(b, "copy_dst_word", "/=", 4);
    b.cp("copy_words", "n");
    op_const(b, "copy_words", "/=", 4);
    b.cp("copy_bytes", "copy_words");
    op_const(b, "copy_bytes", "*=", 4);
    b.operation("src", "+=", "copy_bytes");
    b.operation("dst", "+=", "copy_bytes");
    b.operation("n", "-=", "copy_bytes");
    b.rt("copy_forward_loop");
    raw(b, "return run function {n}:rt/copy_loop");
    save(b, "copy_forward_words");
    raw(b,"execute store result storage {s} macro.p int 1 run scoreboard players get #copy_src_word {o}");
    raw(b,"execute store result storage {s} macro.q int 1 run scoreboard players get #copy_dst_word {o}");
    raw(b, "function {n}:rt/copy_pair with storage {s} macro");
    raw_lines(b,"scoreboard players add #copy_src_word {o} 1\nscoreboard players add #copy_dst_word {o} 1\nscoreboard players remove #copy_words {o} 1\nexecute if score #copy_words {o} matches 1.. run return run function {n}:rt/copy_forward_loop");
    save(b, "copy_forward_loop");
    if b.lazy_initial_memory() {
        raw(b, "execute store result score #val {o} run function {n}:rt/word_read with storage {s} macro");
    } else if b.score_memory {
        raw(
            b,
            "$execute store result score #val {o} run scoreboard players get #m$(p) {n}_mem",
        );
    } else {
        raw(
            b,
            "$execute store result score #val {o} run data get storage {m} w.a$(p)",
        );
    }
    b.emit(format!("${}", b.write_word("$(q)", "val")));
    if b.float_cache() {
        raw_lines(b,"$execute if data storage {n}:fmem w.a$(p) run data modify storage {n}:fmem w.a$(q) set from storage {n}:fmem w.a$(p)\n$execute unless data storage {n}:fmem w.a$(p) if data storage {n}:fmem w.a$(q) run data remove storage {n}:fmem w.a$(q)");
    }
    save(b, "copy_pair");
    b.set("direction", -1);
    b.operation("dst", "+=", "n");
    b.operation("src", "+=", "n");
    raw_lines(
        b,
        "scoreboard players remove #dst {o} 1\nscoreboard players remove #src {o} 1",
    );
    save(b, "copy_backwards");
    raw(b, "execute if score #n {o} matches ..0 run return 0");
    raw(b,"execute if score #n {o} matches 4.. if score #copy_alignment {o} matches 0 run return run function {n}:rt/copy_try_word");
    raw(b, "return run function {n}:rt/copy_byte");
    save(b, "copy_loop");
    b.cp("copy_lane", "src");
    op_const(b, "copy_lane", "%=", 4);
    raw(b,"execute if score #direction {o} matches 1 if score #copy_lane {o} matches 0 run return run function {n}:rt/copy_word");
    raw(b,"execute if score #direction {o} matches -1 if score #copy_lane {o} matches 3 run return run function {n}:rt/copy_word");
    raw(b, "return run function {n}:rt/copy_byte");
    save(b, "copy_try_word");
    b.cp("ptr", "src");
    raw(
        b,
        "execute if score #direction {o} matches -1 run scoreboard players remove #ptr {o} 3",
    );
    b.rt("load_word");
    b.cp("val", "r");
    b.cp("ptr", "dst");
    raw(
        b,
        "execute if score #direction {o} matches -1 run scoreboard players remove #ptr {o} 3",
    );
    b.rt("store_word");
    b.cp("copy_step", "direction");
    op_const(b, "copy_step", "*=", 4);
    b.operation("src", "+=", "copy_step");
    b.operation("dst", "+=", "copy_step");
    raw_lines(
        b,
        "scoreboard players remove #n {o} 4\nreturn run function {n}:rt/copy_loop",
    );
    save(b, "copy_word");
    b.cp("ptr", "src");
    b.rt("load8");
    b.cp("val", "r");
    b.cp("ptr", "dst");
    b.rt("store8");
    b.operation("src", "+=", "direction");
    b.operation("dst", "+=", "direction");
    raw_lines(
        b,
        "scoreboard players remove #n {o} 1\nreturn run function {n}:rt/copy_loop",
    );
    save(b, "copy_byte");
    b.cp("fill_word", "src");
    op_const(b, "fill_word", "%=", 256);
    op_const(b, "fill_word", "*=", 16843009);
    raw(b, "return run function {n}:rt/fill_loop");
    save(b, "memory_fill");
    raw(b, "execute if score #n {o} matches ..0 run return 0");
    b.cp("fill_lane", "dst");
    op_const(b, "fill_lane", "%=", 4);
    raw(b,"execute if score #n {o} matches 4.. if score #fill_lane {o} matches 0 run return run function {n}:rt/fill_word");
    b.cp("val", "src");
    b.cp("ptr", "dst");
    b.rt("store8");
    raw_lines(b,"scoreboard players add #dst {o} 1\nscoreboard players remove #n {o} 1\nreturn run function {n}:rt/fill_loop");
    save(b, "fill_loop");
    b.cp("val", "fill_word");
    b.cp("ptr", "dst");
    b.rt("store_word");
    raw_lines(b,"scoreboard players add #dst {o} 4\nscoreboard players remove #n {o} 4\nreturn run function {n}:rt/fill_loop");
    save(b, "fill_word");
    Ok(())
}

fn floats(b: &mut Backend<'_, '_>) -> Result<()> {
    // Float stores keep a typed dirty value. Materialize IEEE bits only when a
    // byte/integer reader observes that word; full-word overwrites discard it.
    raw(
        b,
        "$data modify storage {s} r.fa set from storage {n}:fmem w.a$(p).v",
    );
    b.rt("float_bits");
    b.emit(format!("${}", b.write_word("$(p)", "r")));
    raw(b, "$data remove storage {n}:fmem w.a$(p).d");
    save(b, "flush_float");
    b.cp("cache_align", "ptr");
    op_const(b, "cache_align", "%=", 4);
    raw(b,"execute unless score #cache_align {o} matches 0 run return run function {n}:rt/store_f32_unaligned");
    b.rt("address");
    raw(
        b,
        "return run function {n}:rt/store_f32_cached with storage {s} macro",
    );
    save(b, "store_f32");
    raw(
        b,
        "$data modify storage {n}:fmem w.a$(p).v set from storage {s} r.fa",
    );
    raw(b, "$data modify storage {n}:fmem w.a$(p).d set value 1b");
    save(b, "store_f32_cached");
    b.rt("float_bits");
    b.cp("val", "r");
    b.rt("store32");
    save(b, "store_f32_unaligned");
    b.cp("cache_align", "ptr");
    op_const(b, "cache_align", "%=", 4);
    raw(b,"execute unless score #cache_align {o} matches 0 run return run function {n}:rt/load_f32_uncached");
    b.rt("address");
    raw(
        b,
        "return run function {n}:rt/load_f32_cached with storage {s} macro",
    );
    save(b, "load_f32");
    raw(b,"$execute if data storage {n}:fmem w.a$(p) run return run data modify storage {s} r.fr set from storage {n}:fmem w.a$(p).v");
    b.rt("load_f32_uncached");
    raw(
        b,
        "$data modify storage {n}:fmem w.a$(p).v set from storage {s} r.fr",
    );
    save(b, "load_f32_cached");
    b.rt("load32");
    b.cp("a", "r");
    b.rt("bits_float");
    save(b, "load_f32_uncached");
    b.cp("cache_align", "ptr");
    op_const(b, "cache_align", "%=", 4);
    raw(
        b,
        "execute unless score #cache_align {o} matches 0 run return 0",
    );
    b.rt("address");
    raw(b, "function {n}:rt/cache_f32_macro with storage {s} macro");
    save(b, "cache_f32");
    raw(
        b,
        "$data modify storage {n}:fmem w.a$(p).v set from storage {s} r.fa",
    );
    save(b, "cache_f32_macro");
    float_expr(b, "fr", "from_int", vec![ir("a")])?;
    let p = b.provider(&Val::Expr(
        Ty::F32,
        "add".into(),
        vec![fr("fr"), fc(4294967296.)],
    ))?;
    raw(b,&format!("execute if score #a {{o}} matches ..-1 run data modify storage {{s}} r.fr set compute default float {p}"));
    save(b, "uint_float");
    // Exact finite f32 bit decoding: mantissa * 2^(exponent-150), with subnormals.
    b.cp("fbits", "a");
    b.cp("mant", "a");
    op_const(b, "mant", "%=", 8388608);
    b.cp("exp", "a");
    op_const(b, "exp", "/=", 8388608);
    op_const(b, "exp", "%=", 256);
    raw(
        b,
        "execute unless score #exp {o} matches 0 run scoreboard players add #mant {o} 8388608",
    );
    raw(
        b,
        "execute if score #exp {o} matches 0 run scoreboard players set #exp {o} 1",
    );
    raw(b, "scoreboard players remove #exp {o} 150");
    let p = json!({"type":"mul","inputs":[b.provider(&from_int("mant"))?,{"type":"pow","base":2.0,"exponent":b.provider(&from_int("exp"))?}]});
    b.emit(format!(
        "data modify storage {} r.fr set compute default float {p}",
        b.storage
    ));
    let neg = b.provider(&Val::Expr(Ty::F32, "neg".into(), vec![fr("fr")]))?;
    raw(b,&format!("execute if score #fbits {{o}} matches ..-1 run data modify storage {{s}} r.fr set compute default float {neg}"));
    save(b, "bits_float");
    // Encode via exponent binary search, never decimal fixed point. Normals and
    // zeros are supported; exceptional values follow documented fast semantics.
    float_expr(b, "abs", "abs", vec![fr("fa")])?;
    b.set("exp", -126);
    for step in [128, 64, 32, 16, 8, 4, 2, 1] {
        b.cp("try_exp", "exp");
        raw(b, &format!("scoreboard players add #try_exp {{o}} {step}"));
        let power = json!({"type":"pow","base":2.0,"exponent":b.provider(&from_int("try_exp"))?});
        let pred = json!({"type":"float_value_check","value":b.provider(&fr("abs"))?,"test":{"min":power}});
        b.emit(format!(
            "execute if predicate {pred} run scoreboard players operation {} = {}",
            b.score("exp"),
            b.score("try_exp")
        ));
    }
    let mant = json!({"type":"div","left":b.provider(&fr("abs"))?,"right":{"type":"pow","base":2.0,"exponent":{"type":"sub","left":b.provider(&from_int("exp"))?,"right":23.0}}});
    ie(b, "mant", json!({"type":"from_float","input":mant}));
    b.cp("r", "exp");
    raw(b, "scoreboard players add #r {o} 127");
    op_const(b, "r", "*=", 8388608);
    b.operation("r", "+=", "mant");
    raw(b, "scoreboard players remove #r {o} 8388608");
    let sub = json!({"type":"float_value_check","value":b.provider(&fr("abs"))?,"test":{"max":1.1754942106924411e-38}});
    b.emit(format!(
        "execute if predicate {sub} run scoreboard players operation {} = {}",
        b.score("r"),
        b.score("mant")
    ));
    let sign =
        json!({"type":"float_value_check","value":b.provider(&fr("fa"))?,"test":{"min":0.0}});
    let min = b.c(i32::MIN as i64);
    b.emit(format!(
        "execute unless predicate {sign} run scoreboard players operation {} += {}",
        b.score("r"),
        b.score(&min)
    ));
    save(b, "float_bits");
    for mode in ["s", "u"] {
        ie(
            b,
            "r",
            json!({"type":"from_float","input":b.provider(&fr("fa"))?}),
        );
        if mode == "u" {
            let pred = json!({"type":"float_value_check","value":b.provider(&fr("fa"))?,"test":{"min":2147483648.0}});
            let p = json!({"type":"from_float","input":{"type":"sub","left":b.provider(&fr("fa"))?,"right":4294967296.0}});
            b.emit(format!(
                "execute if predicate {pred} store result score {} run compute default integer {p}",
                b.score("r")
            ));
        }
        save(b, &format!("float_i32_{mode}"));
    }
    for mode in ["s", "u"] {
        let negative = json!({"type":"float_value_check","value":b.provider(&fr("fa"))?,"test":{"max":-0.000000000000000000000000000000000000000000001}});
        b.set("float_negative", 0);
        b.emit(format!(
            "execute if predicate {negative} run scoreboard players set {} 1",
            b.score("float_negative")
        ));
        float_expr(b, "fa", "abs", vec![fr("fa")])?;
        float_expr(b, "wide_float", "trunc", vec![fr("fa")])?;
        float_expr(
            b,
            "wide_hi",
            "floor",
            vec![Val::Expr(
                Ty::F32,
                "div".into(),
                vec![fr("wide_float"), fc(4294967296.)],
            )],
        )?;
        b.materialize(&fr("wide_hi"), Some(fr("fa")))?;
        b.rt("float_i32_u");
        b.cp("float_high_bits", "r");
        float_expr(
            b,
            "fa",
            "sub",
            vec![
                fr("wide_float"),
                Val::Expr(Ty::F32, "mul".into(), vec![fr("wide_hi"), fc(4294967296.)]),
            ],
        )?;
        b.rt("float_i32_u");
        b.cp("rh", "float_high_bits");
        if mode == "s" {
            raw(
                b,
                "execute if score #float_negative {o} matches 1 run function {n}:rt/neg64_r",
            );
        }
        save(b, &format!("float_i64_{mode}"));
    }
    b.rt("float_bits");
    b.rt("f32_to_f64_bits");
    save(b, "f64_bits");
    b.cp("r", "a");
    b.cp("rh", "ah");
    b.rt("f64_to_f32_bits");
    b.cp("a", "r");
    b.rt("bits_float");
    save(b, "bits_f64");
    // Fast f64 storage encoding: f32 input expanded into IEEE double bits.
    b.cp("a", "r");
    b.cp("mant", "r");
    op_const(b, "mant", "%=", 8388608);
    b.cp("exp", "r");
    op_const(b, "exp", "/=", 8388608);
    op_const(b, "exp", "%=", 256);
    b.cp("r", "mant");
    op_const(b, "r", "*=", 1 << 29);
    b.cp("rh", "mant");
    op_const(b, "rh", "/=", 8);
    raw(
        b,
        "execute unless score #exp {o} matches 0 run scoreboard players add #exp {o} 896",
    );
    op_const(b, "exp", "*=", 1 << 20);
    b.operation("rh", "+=", "exp");
    let c = b.c(i32::MIN as i64);
    raw(b,&format!("execute if score #a {{o}} matches ..-1 run scoreboard players operation #rh {{o}} += #{c} {{o}}"));
    save(b, "f32_to_f64_bits");
    b.cp("mant", "rh");
    op_const(b, "mant", "%=", 1 << 20);
    op_const(b, "mant", "*=", 8);
    b.cp("a", "r");
    b.rt("shr_u/29");
    b.operation("mant", "+=", "r");
    b.cp("exp", "rh");
    op_const(b, "exp", "/=", 1 << 20);
    op_const(b, "exp", "%=", 2048);
    raw(
        b,
        "execute unless score #exp {o} matches 0 run scoreboard players remove #exp {o} 896",
    );
    op_const(b, "exp", "*=", 1 << 23);
    b.cp("r", "mant");
    b.operation("r", "+=", "exp");
    let c = b.c(i32::MIN as i64);
    raw(b,&format!("execute if score #rh {{o}} matches ..-1 run scoreboard players operation #r {{o}} += #{c} {{o}}"));
    save(b, "f64_to_f32_bits");
    float_expr(b, "fr", "abs", vec![fr("fa")])?;
    let pred =
        json!({"type":"float_value_check","value":b.provider(&fr("fb"))?,"test":{"min":0.0}});
    let neg = b.provider(&Val::Expr(Ty::F32, "neg".into(), vec![fr("fr")]))?;
    b.emit(format!("execute unless predicate {pred} run data modify storage {} r.fr set compute default float {neg}",b.storage));
    save(b, "fcopysign");
    for mode in ["s", "u"] {
        b.cp("float_original_sign", "ah");
        if mode == "s" {
            b.cp("r", "a");
            b.cp("rh", "ah");
            raw(b, "execute if score #float_original_sign {o} matches ..-1 run function {n}:rt/neg64_r");
            b.cp("a", "r");
            b.cp("ah", "rh");
        }
        b.cp("float_hi", "ah");
        b.rt("uint_float");
        float_expr(b, "lo_float", "add", vec![fr("fr"), fc(0.)])?;
        b.cp("a", "float_hi");
        b.rt("uint_float");
        float_expr(
            b,
            "fr",
            "add",
            vec![
                fr("lo_float"),
                Val::Expr(Ty::F32, "mul".into(), vec![fr("fr"), fc(4294967296.)]),
            ],
        )?;
        if mode == "s" {
            let neg = b.provider(&Val::Expr(Ty::F32, "neg".into(), vec![fr("fr")]))?;
            b.emit(format!("execute if score {} matches ..-1 run data modify storage {} r.fr set compute default float {neg}",b.score("float_original_sign"),b.storage));
        }
        save(b, &format!("i64_float_{mode}"));
    }
    Ok(())
}
// Build command macro arguments per Unicode scalar rather than concatenating
// strings through SNBT. The final substitution is command text, so quotes and
// backslashes are preserved exactly and never re-parsed as an accumulator.
fn utf8_commands(b: &mut Backend<'_, '_>) -> Result<()> {
    for capacity in [64usize, 256, 1024, 4096, 32768] {
        let fields = (0..capacity)
            .map(|i| format!("c{i}:\"\""))
            .collect::<Vec<_>>()
            .join(",");
        raw(
            b,
            &format!("data modify storage {{s}} host.text set value {{{fields}}}"),
        );
        b.set("utf_capacity", capacity as i64);
        save(b, &format!("utf8_init/{capacity}"));
        let parts = (0..capacity)
            .map(|i| format!("$(c{i})"))
            .collect::<String>();
        raw(b, &format!("$return run {parts}"));
        save(b, &format!("utf8_run/{capacity}"));
    }
    raw_lines(b,"scoreboard players set #host_error {o} 0\nscoreboard players set #utf_cont {o} 0\nscoreboard players set #utf_index {o} 0\nexecute unless score #n {o} matches 1..32767 run scoreboard players set #host_error {o} 1\nexecute unless score #src {o} matches 0.. run scoreboard players set #host_error {o} 1");
    // Max guest memory is 256 pages in this backend. Positive signed bounds
    // suffice, and validating the entire range prevents every load from going
    // through a per-byte bounds check.
    let size = json!({"type":"mul","inputs":[b.provider(&ir("pages"))?,65536]});
    ie(
        b,
        "utf_available",
        json!({"type":"sub","left":size,"right":b.provider(&ir("src"))?}),
    );
    raw_lines(b,"execute if score #n {o} > #utf_available {o} run scoreboard players set #host_error {o} 1\nexecute if score #host_error {o} matches 1.. run return fail");
    for (lo, hi, capacity) in [
        (1, 64, 64),
        (65, 256, 256),
        (257, 1024, 1024),
        (1025, 4096, 4096),
        (4097, 32767, 32768),
    ] {
        raw(b,&format!("execute if score #n {{o}} matches {lo}..{hi} run function {{n}}:rt/utf8_init/{capacity}"));
    }
    b.rt("utf8_loop");
    raw(
        b,
        "execute unless score #utf_cont {o} matches 0 run scoreboard players set #host_error {o} 1",
    );
    save(b, "read_utf8");
    raw_lines(b,"execute if score #n {o} matches ..0 run return 0\nexecute if score #host_error {o} matches 1.. run return fail");
    b.cp("ptr", "src");
    b.rt("load8");
    b.cp("utf_byte", "r");
    raw_lines(b,"execute if score #utf_cont {o} matches 1.. run function {n}:rt/utf8_continue\nexecute if score #utf_cont {o} matches 0 if score #utf_byte {o} matches 0.. run function {n}:rt/utf8_lead\nscoreboard players add #src {o} 1\nscoreboard players remove #n {o} 1\nreturn run function {n}:rt/utf8_loop");
    save(b, "utf8_loop");
    raw_lines(b,"execute unless score #utf_byte {o} matches 128..191 run scoreboard players set #host_error {o} 1\nexecute if score #host_error {o} matches 1.. run return fail");
    ie(
        b,
        "utf_code",
        json!({"type":"add","inputs":[{"type":"mul","inputs":[b.provider(&ir("utf_code"))?,64]},b.provider(&ir("utf_byte"))?,-128]}),
    );
    raw_lines(b,"scoreboard players remove #utf_cont {o} 1\nscoreboard players set #utf_byte {o} -1\nexecute unless score #utf_cont {o} matches 0 run return 0\nexecute if score #utf_code {o} < #utf_min {o} run scoreboard players set #host_error {o} 1\nexecute if score #utf_code {o} matches 55296..57343 run scoreboard players set #host_error {o} 1\nexecute if score #utf_code {o} matches 1114112.. run scoreboard players set #host_error {o} 1\nexecute if score #host_error {o} matches 1.. run return fail");
    b.rt("utf8_append");
    save(b, "utf8_continue");
    b.cp("utf_code", "utf_byte");
    raw(
        b,
        "execute if score #utf_byte {o} matches 32..127 run return run function {n}:rt/utf8_append",
    );
    for (lo, hi, offset, count, min) in [
        (194, 223, 192, 1, 128),
        (224, 239, 224, 2, 2048),
        (240, 244, 240, 3, 65536),
    ] {
        raw(b,&format!("execute if score #utf_byte {{o}} matches {lo}..{hi} run return run function {{n}}:rt/utf8_lead{count}"));
        let parent = std::mem::take(&mut b.lines);
        raw(
            b,
            &format!("scoreboard players remove #utf_code {{o}} {offset}"),
        );
        b.set("utf_cont", count);
        b.set("utf_min", min);
        save(b, &format!("utf8_lead{count}"));
        b.lines = parent;
    }
    b.set("host_error", 1);
    raw(b, "return fail");
    save(b, "utf8_lead");
    // Fast printable ASCII path is already represented in the runtime table.
    raw_lines(b,"execute store result storage {s} macro.i int 1 run scoreboard players get #utf_index {o}\nexecute store result storage {s} macro.c int 1 run scoreboard players get #utf_code {o}\nexecute if score #utf_code {o} matches 0..127 run function {n}:rt/utf8_ascii with storage {s} macro\nexecute if score #utf_code {o} matches 128.. run function {n}:rt/utf8_unicode\nscoreboard players add #utf_index {o} 1");
    save(b, "utf8_append");
    raw(
        b,
        "$data modify storage {s} host.text.c$(i) set from storage {s} chars[$(c)]",
    );
    save(b, "utf8_ascii");
    for digit in 0..6 {
        let p = json!({"type":"floor_mod","left":{"type":"floor_div","left":b.provider(&ir("utf_code"))?,"right":1u32 << (digit*4)},"right":16});
        raw(
            b,
            &format!("data modify storage {{s}} macro.d{digit} set compute default integer {p}"),
        );
    }
    raw(b, "function {n}:rt/utf8_hex with storage {s} macro");
    raw(b, "function {n}:rt/utf8_scalar with storage {s} macro");
    save(b, "utf8_unicode");
    for digit in 0..6 {
        raw(
            b,
            &format!(
                "$data modify storage {{s}} macro.h{digit} set from storage {{s}} hex[$(d{digit})]"
            ),
        );
    }
    save(b, "utf8_hex");
    raw(b,"$data modify storage {s} host.text.c$(i) set value \"\\U00$(h5)$(h4)$(h3)$(h2)$(h1)$(h0)\"");
    save(b, "utf8_scalar");
    raw(
        b,
        "execute if score #host_error {o} matches 1.. run return fail",
    );
    for capacity in [64, 256, 1024, 4096, 32768] {
        raw(b,&format!("execute if score #utf_capacity {{o}} matches {capacity} run return run function {{n}}:rt/utf8_run/{capacity} with storage {{s}} host.text"));
    }
    raw(b, "return fail");
    save(b, "utf8_command");
    Ok(())
}

fn host(b: &mut Backend<'_, '_>) -> Result<()> {
    raw(
        b,
        "execute if score #host_error {o} matches 1.. run return fail",
    );
    raw(b, "$return run $(cmd)");
    save(b, "command");
    raw_lines(
        b,
        "scoreboard players set #host_error {o} 0\ndata modify storage {s} host.cmd set value \"\"\nexecute unless score #n {o} matches 0..32767 run scoreboard players set #host_error {o} 1\nreturn run function {n}:rt/string_loop",
    );
    save(b, "read_string");
    raw(b, "execute if score #n {o} matches ..0 run return 0");
    raw(
        b,
        "execute if score #host_error {o} matches 1.. run return fail",
    );
    b.cp("ptr", "src");
    b.rt("load8");
    b.cp("char", "r");
    b.rt("char_append");
    raw_lines(b,"scoreboard players add #src {o} 1\nscoreboard players remove #n {o} 1\nreturn run function {n}:rt/string_loop");
    save(b, "string_loop");
    raw(b,"execute unless score #char {o} matches 32..126 run scoreboard players set #host_error {o} 1");
    raw(
        b,
        "execute if score #char {o} matches 39 run scoreboard players set #host_error {o} 1",
    );
    raw(
        b,
        "execute if score #char {o} matches 92 run scoreboard players set #host_error {o} 1",
    );
    raw(
        b,
        "execute if score #host_error {o} matches 1.. run return fail",
    );
    raw_lines(b,"execute store result storage {s} macro.c int 1 run scoreboard players get #char {o}\nfunction {n}:rt/char_read with storage {s} macro\nfunction {n}:rt/char_concat with storage {s} host");
    save(b, "char_append");
    raw(
        b,
        "$data modify storage {s} host.ch set from storage {s} chars[$(c)]",
    );
    save(b, "char_read");
    raw(
        b,
        "$data modify storage {s} host.cmd set value '$(cmd)$(ch)'",
    );
    save(b, "char_concat");
    b.rt("char_append");
    save(b, "putc");
    Ok(())
}
