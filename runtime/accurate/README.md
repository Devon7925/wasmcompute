# Accurate numeric runtime

Berkeley SoftFloat source is vendored at commit
`a0c6494cdc11865811dec815d5c0049fba9d82a8`. Its BSD license is in
`vendor/softfloat/LICENSE`. The selected musl math files and MIT license are in
`vendor/musl`; their exact content hashes are in `PROVENANCE.json`.

Rebuild from the project root with a Wasm-capable Clang:

```
python tools/build_accurate_runtime.py --clang /path/to/clang --output /path/to/build
```

The release helper was built with Clang 18.1.8. The generated bridge, reference
library, and selected musl routines are linked into `src/accurate_runtime.wasm`.
`BUILD.json` records the compiler flags and embedded payload hash. Cargo embeds
this file into the Rust compiler. No C compiler or SoftFloat installation is
needed by users compiling their own Wasm into mcfunctions.

The helper exports an integer ABI. The Rust lowering pass recursively replaces
its remaining musl float instructions with software-float calls, relocates
function/global/type indices, and emits only ordinary vanilla commands.
