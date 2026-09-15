# Accelerator runtime checks

Run the packaged, shaded jar with JDK 25 on either Windows or Linux:

```sh
python tools/test_accelerator.py --jdk /path/to/jdk-25 --jar bin/wasmcompute-accelerator-0.1.0.jar --build target/accelerator-tests
```

The 1,648 ordered numeric vectors in `numeric-cases.json` record results and
trap codes previously matched by accurate mcfunctions and Wasmtime. They cover
floating arithmetic, NaN payloads, signed zero, conversions, wide integers,
indirect calls, memory bounds and growth. Memory tests share one instance, so
their order matters. Values are raw integer bits; result fields are ignored
when a trap is expected. These are regression vectors, not a full Wasm
conformance suite.

`guards.wat` checks command validation, reentry, memory maxima, stack-pointer
restoration, closed instances, unconditional/conditional/table loop limits,
recursion and interruption. The test runner has a process timeout as well.
Neither suite starts Minecraft or accesses a GPU.

The paired `.wat` files are the source for the checked-in `.wasm` fixtures.
Regenerate them with a standard WAT assembler after editing; review changes to
expected numeric results against accurate compilation before updating them.
