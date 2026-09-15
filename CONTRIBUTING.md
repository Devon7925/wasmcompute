# Contributing

Start with a small reproducer and the exact Minecraft, Rust and compiler versions.
For compiler issues, include the smallest redistributable `.wat`/`.wasm` input,
mode, compiler diagnostics, and expected versus actual results. Do not attach
private worlds, server credentials or copyrighted game assets.

Build with Rust 1.92.0 and `wasm32-unknown-unknown`. Run the commands under
Development in the README. Changes to lowering or runtime semantics need tests
that distinguish actual Wasm behavior, plus headless Minecraft validation when
command behavior changes. Keep Fast's approximations explicit. Preserve the
original game code when optimizing compiler translations; do not tailor game
expressions to LLVM optimization accidents.

Use `cargo fmt` for compiler/SDK changes. Keep expensive demos out of ordinary
CI and document benchmark inputs, warmup, timer scope, output parity and failed
baselines. CPU-only/headless runs avoid interfering with other GPU workloads.

Original contributions are accepted under 0BSD. Changes to vendored code retain
its existing license; identify upstream revisions and modifications. Do not
add an IWAD, Minecraft jar, world save, extracted assets, credentials or a
prebuilt game payload to Git. See THIRD_PARTY.md and docs/releasing.md.
