# Changelog

## 0.1.0 — initial public preview

- Rust WebAssembly-to-mcfunction compiler for Minecraft 26.3-rc-1.
- Datapack-shaped Rust sources and a flat command SDK, including command blocks.
- Fast native-provider lowering and Accurate software floating point/checks.
- Optional server-only Fabric accelerator using pure-Java Chicory: one jar for Windows and Linux, with numeric parity and execution-limit tests.
- Celeste, Doom, Serde and finite-area terrain demos with documented parity,
  timings and external asset requirements.
- 0BSD for original code, preserved third-party licenses, portable compiler
  bundles, Windows/Linux CI and a clean source export.

Compatibility is limited to the contracts in docs/. Published source and downloads are available on
[GitHub](https://github.com/Devon7925/wasmcompute). The crate is not published to crates.io.
