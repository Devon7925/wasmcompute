# Other languages and standalone Wasm

The compiler accepts a core Wasm binary independently of the Rust source-pack
frontend. Use a freestanding wasm32 target with no WASI imports. Configure your
compiler to emit only the [supported scalar subset](semantics.md). A language
being able to emit Wasm does not guarantee its standard runtime is supported.

```sh
cargo run --release -- examples/rust_commands.wasm -o generated/commands --namespace commands --bindings examples/bindings.json
```

The checked-in [Rust example](../examples/rust_commands.rs) and
[C example](../examples/commands.c) show low-level imports. The
[binding file](../examples/bindings.json) maps them to command templates.
Inspect `wasmcompute.json` for exports and function indices;
standalone programs use internal argument/result scores and explicit runtime
initialization. This interface is lower-level than `/function` source packs.
After installing this standalone example, run `/function commands:init`, then
`/function commands:export/run`. The example creates the `rustdemo` objective,
writes `example = 42`, and leaves its return value in `#ret commands`.
Unlike source packs, standalone initialization is explicit.
See [architecture](architecture.md) for details.

`--accurate` selects the checked backend. `-O 0` disables optimizations;
`-O 1` is the default. Experimental `--nbt-memory`, `--word-buffer`, and guarded
`--copy-profile` tuning are described in the architecture and benchmark notes.
They are workload-dependent, not recommended defaults.

The Fabric accelerator's supported author interface is the Rust source-pack
ABI. Compiling arbitrary standalone Wasm does not automatically make it an
accelerator-compatible source pack. C libraries can be linked into Rust source
packs, as the Celeste and Doom demos demonstrate.
