pub fn main() -> i32 {
    let mut next = 0;
    minecraft::commands! {};
    minecraft::commands! {
        "scoreboard objectives add batch dummy";
        "scoreboard players set #first batch $(value)", value = { next += 1; next },;
        "execute if entity @a[tag=nonexistent_batch_test] run say unreachable";
        "scoreboard players set #second batch $(value)", value = { next += 1; next };
        r#"data modify storage batch:test value set value $(value)"#, value: f32 = 1.25;
    }
    minecraft::commands! { "scoreboard players set #last batch 7" }
    next
}
