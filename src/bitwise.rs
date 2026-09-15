//! Exact i32 bit operations using native integer providers, without macro tables.
use serde_json::{json, Value};

pub fn provider(op: &str, a: Value, b: Value) -> Value {
    if a == b {
        return if op == "xor" { json!(0) } else { a };
    }
    if a.as_i64().is_some() && b.as_i64().is_none() {
        return provider(op, b, a);
    }
    let constant = b.as_i64().map(|n| n as i32);
    if let Some(k) = constant {
        if k == 0 {
            return if op == "and" { json!(0) } else { a };
        }
        if k == -1 {
            return match op {
                "and" => a,
                "or" => json!(-1),
                // -(a + 1) can overflow an intermediate at MAX; -1 - a cannot.
                _ => json!({"type":"sub","left":-1,"right":a}),
            };
        }
        let mask = k as u32;
        if (op == "and" && k == i32::MAX) || (op == "or" && k == i32::MIN) {
            let low = json!({"type":"add","inputs":[{"type":"floor_mod","left":a,"right":1073741824},{"type":"mul","inputs":[{"type":"floor_mod","left":{"type":"floor_div","left":a,"right":1073741824},"right":2},1073741824]}]});
            return if op == "and" {
                low
            } else {
                json!({"type":"add","inputs":[low,i32::MIN]})
            };
        }
        if mask.wrapping_add(1).is_power_of_two() && k != i32::MAX {
            let base = mask + 1;
            if op == "and" {
                return json!({"type":"floor_mod","left":a,"right":base});
            }
            if op == "or" {
                return json!({"type":"add","inputs":[{"type":"mul","inputs":[{"type":"floor_div","left":a,"right":base},base]},k]});
            }
        }
        let low = !mask;
        if low.wrapping_add(1).is_power_of_two() && low < i32::MAX as u32 {
            let base = low + 1;
            if op == "or" {
                return json!({"type":"add","inputs":[{"type":"floor_mod","left":a,"right":base},k]});
            }
            if op == "and" {
                return json!({"type":"mul","inputs":[{"type":"floor_div","left":a,"right":base},base]});
            }
        }
    }
    fn bit(v: &Value, i: u32) -> Value {
        if let Some(n) = v.as_i64() {
            return json!(((n as u32) >> i) & 1);
        }
        let shifted = if i == 31 {
            // Every provider constant/intermediate stays inside signed i32.
            json!({"type":"floor_div","left":{"type":"floor_div","left":v,"right":1073741824},"right":2})
        } else if i == 0 {
            v.clone()
        } else {
            json!({"type":"floor_div","left":v,"right":1u32<<i})
        };
        json!({"type":"floor_mod","left":shifted,"right":2})
    }
    let mut terms = Vec::new();
    for i in 0..32 {
        let x = bit(&a, i);
        let y = bit(&b, i);
        let p = match (op, y.as_i64()) {
            ("and", Some(0)) | ("or", Some(1)) => y,
            ("and", Some(1)) | ("or" | "xor", Some(0)) => x,
            ("xor", Some(1)) => json!({"type":"sub","left":1,"right":x}),
            ("and", _) => json!({"type":"min","inputs":[x,y]}),
            ("or", _) => json!({"type":"max","inputs":[x,y]}),
            ("xor", _) => json!({"type":"abs","input":{"type":"sub","left":x,"right":y}}),
            _ => unreachable!(),
        };
        if p.as_i64() == Some(0) {
            continue;
        }
        let weight = if i == 31 { i32::MIN } else { 1i32 << i };
        terms.push(if p.as_i64() == Some(1) {
            json!(weight)
        } else if i == 0 {
            p
        } else {
            json!({"type":"mul","inputs":[p,weight]})
        });
    }
    // Positive terms sum to <= MAX; the final sign term is 0 or MIN.
    json!({"type":"add","inputs":terms})
}
