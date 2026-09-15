# Rust functions inside ordinary datapacks

This implements the source-pack frontend and flat Rust command SDK from proposal v2. Builds emit fast vanilla mcfunctions by default, or accurate mcfunctions with `--accurate`. The [Fabric accelerator](accelerator.md) executes the retained source Wasm with accurate Wasm semantics.

## Build

Install Rust and the `wasm32-unknown-unknown` target. Keep the distributed `sdk/` directory beside `bin/`.

```powershell
rustup target add wasm32-unknown-unknown
cargo run --release -- build examples/source-pack -o generated/source-pack
```

Add `--offline` when Cargo dependencies are already cached. The existing `wasmcompute program.wasm -o output --namespace demo` command still works.

`build --copy-profile path/to/profile.json` enables the same guarded bulk-copy specializations as standalone compilation. Addresses must match the linked source module. The Celeste demo includes a guarded profile for its current fixed-point renderer.

The source must contain `pack.mcmeta` for Minecraft **26.3-rc-1**, with `min_format` and `max_format` both `121` or `[121,0]`. This release rejects other formats because it emits snapshot compute commands. Output must be outside the source directory. Install the resulting directory in the world's `datapacks/` and reload.

## Author functions

```text
my-pack/
  pack.mcmeta
  data/
    minecraft/tags/function/load.json
    arena/function/load.rs
    arena/function/paint.rs
    arena/function/tick.mcfunction
    arena/advancement/start.json
    arena/worldgen/configured_feature/marker.json
```

`data/arena/function/paint.rs` defines `arena:paint`:

```rust
use minecraft::{command, fill};

pub fn main() -> i32 {
    let height = command!("data get storage arena:config rules.height");
    fill!("~ ~ ~ ~7 ~$(height) ~7 minecraft:stone", height = height)
}
```

Call it normally: `execute as @p at @s run function arena:paint`. Its Rust result is the Minecraft function's integer return. Zero is a successful return value. A guest panic/trap fails the invocation, retains earlier world effects and records the public function ID and trap code in `<private-namespace>:diagnostics last_trap`. Find that namespace in the output's `wasmcompute-build.json`.

Source files must expose `pub fn main() -> i32`; the compiler supplies Wasm exports and a panic handler. Every `.rs` file below a `function/` directory is an entry point. Put helpers in ordinary libraries or outside that directory. Duplicate public IDs, including a `.rs` and `.mcfunction` with the same stem, are errors. Names use Minecraft's lowercase resource-ID characters.

Small packs require no Cargo manifest. For dependencies or shared code, define an ordinary library package in `Cargo.toml` and `src/lib.rs`. The generated entry crate depends on that library and on the bundled SDK. The [example](../examples/source-pack/README.md) uses Serde and serde-json-core with fixed buffers. Packs default to `no_std`. Helpers that directly call the SDK must declare their own path dependency on `sdk/rust/minecraft`.

For allocation and portable parts of Rust's standard library, add this to the source pack's Cargo manifest:

```toml
[package.metadata.wasmcompute]
std = true
```

The target remains `wasm32-unknown-unknown`: `Vec`, maps and allocation work, but this does not provide an operating system or WASI. Packages requiring files, sockets, threads or OS randomness need suitable adapters. The terrain demo uses this option for its generation state and caches.

Generated Cargo glue and dependency builds live in the source's `target/wasmcompute/`. Cargo diagnostics point to the original source files. The generated manifest can also be opened with Rust tooling for indexing. An authored `Cargo.lock` seeds dependency resolution; generated glue/SDK entries are added in the generated lockfile.

`CARGO_TARGET_DIR` can place dependency artifacts elsewhere. On Windows, deeply nested packs automatically use a short directory under the system temporary directory to avoid native linker path limits. The generated entry project and a copy of the original `module.wasm` remain under `target/wasmcompute/entry/`.

## Commands

| API | Return |
| --- | --- |
| `command!("...")` | Command integer result; ordinary failure returns zero |
| `commands! { "..."; "...", x = value; }` | `()`; runs each command in order |
| `fill!("...")`, `execute!("...")`, etc. | Same, with the command name prefixed |
| `command_success!("...")` | Boolean command success |
| `command_outcome!("...")` | `CommandOutcome { result: i32, success: bool }` |
| `command(text: &str)` | Runtime command integer result |

`command_outcome!` executes once. It distinguishes successful zero from failure. Static commands emit directly unless a command-level `return` requires a helper to keep Rust control flow intact. Numeric templates use native Minecraft macros and need no authored bindings file.

```rust
let outcome = minecraft::command_outcome!(
    "scoreboard players add #count arena $(n)", n = 3
);
minecraft::command!(
    "data modify storage arena:out speed set value $(speed)f",
    speed: f32 = 3.25
);
```

Bindings default to `i32`; annotate `f32` or `f64` explicitly. Values are evaluated once. Missing, duplicate and unused bindings fail at the source macro. Floating bindings must be finite; the fast backend retains its existing approximate floating semantics. Arbitrary strings/objects are not interpolated as numeric parameters. JSON/SNBT braces remain literal; Unicode, quotes and backslashes in command literals work normally.

Use `commands!` for setup sequences whose results you do not need:

```rust
let column = 12;
minecraft::commands! {
    "time set noon";
    "setblock $(x) 80 64 minecraft:stone strict", x = column;
    "data modify storage arena:out speed set value $(speed)f", speed: f32 = 3.25;
}
```

Each statement has the same literal and numeric binding syntax as `command!`.
The final semicolon is optional, and an empty batch is allowed. Binding
expressions run exactly once when their statement is reached. Ordinary command
failure does not skip later commands. A batch expands to ordinary SDK calls;
it introduces no runtime command parser or additional dispatch layer. Keep Rust
branches and loops outside the batch, and use `command!` for results that drive
Rust control flow.

Runtime `command(&str)` supports valid UTF-8 up to **32,767 bytes**, including apostrophes, backslashes, BMP characters and supplementary Unicode characters. It validates the buffer and UTF-8 before dispatch and rejects control bytes below space. Failed validation sets the private `#host_error` score and returns zero without running a partial command. This path is substantially more expensive than a literal/template: it decodes characters into macro arguments and dispatches one assembled command. It does not escape user-supplied command syntax for you. Serialize values for their intended grammar position, as in the Serde example.

The older standalone `mc_command_result` import keeps its old ASCII contract for compatibility. The new SDK uses `mc_command_utf8_result`.

## Native resources and callbacks

Tags, advancements, worldgen definitions, structures, unknown resource folders and other files below `data/` pass through byte-for-byte. `pack.mcmeta` and `pack.png` are preserved. Rust sources, Cargo files and caches are not shipped as runtime resources. Native-only packs can also be copied through the build command.

There is one intentional output transformation: the generated private initializer is prepended to `minecraft:load`. Existing values retain their order, and fields such as `replace` remain intact. The authored source tag is never changed. This initializes the module before this pack's authored load entries. A function also initializes lazily if called before its first load hook. Another pack that replaces/reorders load tags controls Minecraft's resulting load order; a call before this pack's hook on a later reload can still observe the previous instance until that hook executes.

Minecraft executor, position, dimension, rotation and other command-source context flow through native function calls. Nested Rust → command → Rust calls save caller registers, command buffers and internal frames. Guest memory/global mutations remain visible. The Rust ABI stack pointer is restored after ordinary calls and traps. Ordinary top-level calls keep their bookkeeping in scores; only nested calls allocate an external NBT frame. Nested calls still save all generated scratch registers, so large modules have extra callback cost. Standalone benchmark exports do not pay this boundary cost.

Public calls still pass through Minecraft resource resolution. A higher-priority native pack can override a Rust function normally. Native tags, advancements, `schedule function` and tick rules remain the scheduling interface.

Guest state persists between invocations and resets on this pack's initialization/reload. Persist application saves explicitly in scores/storage. Rebuilds use a namespace derived from function IDs and Wasm contents, so distinct builds do not deliberately share scratch storage. This short namespace hash is allocation bookkeeping, not a cryptographic integrity check.

## Rebuild behavior and current boundaries

The build writes only to an empty directory or an output with its own build record. It tracks generated files, removes stale generated files on rebuild, preserves unrelated files, and refuses to overwrite a new unowned file. Path traversal and resource symlinks are rejected. Build/compile failures occur before output writes; disk failures during the final write are not transactionally rolled back.

Still outstanding from the full proposal: C/C++ source-file frontends, incoming function macro arguments, general storage-to-guest string access, an optional allocator. `build ... --accurate` enables [accurate vanilla lowering](accurate.md) with a separate private namespace and the same public function IDs. The stored original Wasm is executed by the optional accelerator without imitating mcfunction fast math. Generated SHA-256 metadata binds the selected public function and payload; edited resources and native overrides fall back to mcfunctions.

The Serde example exports a shareable JSON string and demonstrates bounded deserialization of an in-guest JSON input. Importing an arbitrary JSON string from Minecraft storage is not implemented. The separate [playable demos](../demos/celeste-direct/README.md) now render original pixels with Rust input and lifecycle functions; performance depends on the selected backend.

## Development checks

```sh
cargo test
cargo test --manifest-path sdk/rust/minecraft-macros/Cargo.toml
```

The release CI also builds the hello and Serde examples in both vanilla modes.
See [benchmark scope](benchmarks.md) for the measured demo results.
