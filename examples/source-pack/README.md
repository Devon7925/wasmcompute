# Rust functions and Serde config example

Build this source pack before installing it:

```powershell
cargo run --release -- build examples/source-pack -o generated/source-pack
```

The compiler bundles the `minecraft` SDK and generates exports. `Cargo.toml` is an ordinary shared-library manifest for `arena_helpers`, Serde and serde-json-core. Sources use `no_std` and bounded buffers.

After installing the built output and reloading:

```mcfunction
data modify storage arena:config rules set value {height:4}
function arena:config/export
data get storage arena:config share
function arena:config/roundtrip
function arena:counter
function arena:callback
execute as @p at @s run function arena:paint
```

`config/export` uses Serde to store the shareable JSON string `{"height":4}`. Heights outside 1–16 return `-1` and preserve the previous exported string. `config/roundtrip` deserializes and validates a bounded in-guest JSON example, then serializes it back. Reading arbitrary incoming JSON text from Minecraft storage is future work.

`counter` and `callback` demonstrate persistent Rust library state and nested calls through native function resolution. `paint` demonstrates a flat `fill!` command with an integer binding; it places an 8×4×8 stone region relative to the caller. Invoke it where those block changes are intended. The advancement retains its ordinary reward function reference. Tick remains an authored `.mcfunction` that increments a score.

See [source-pack documentation](../../docs/source-packs.md) for command outcomes, Unicode runtime text, initialization and current limits.
