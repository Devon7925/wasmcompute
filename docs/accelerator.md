# Fabric accelerator

The server-only Fabric mod compiles the original source-pack Wasm into JVM
bytecode using [Chicory 1.7.5](https://chicory.dev/docs/usage/runtime-compiler/).
The JVM executes and optimizes it on the CPU. The entire accelerator is Java;
one jar works across Windows and Linux, without JNI, a DLL or a shared library.
The datapack compiler remains Rust. Clients need no mod or resource pack.

## Compatibility

This build targets **Minecraft 26.3-rc-1, Fabric Loader 0.19.5, Java 25**. The same jar was validated on **Windows 11 x64 and Ubuntu
24.04 x64 under WSL**. Build `bin/wasmcompute-accelerator-0.1.0.jar` using the instructions below, then
copy it into the server's `mods` folder. Source exports do not include a prebuilt mod.
No Fabric API dependency is required. The runtime has no OS-specific code or
architecture-specific binaries; macOS and ARM have not yet been integration-tested.
Minecraft/Fabric compatibility remains version-specific even though the jar is
OS-independent.

Build a source pack normally with `wasmcompute build source -o dist` or add
`--accurate`. The compiler now emits `wasmcompute-accelerator.json` alongside the
unchanged source Wasm and vanilla fallback. Keep these files together when shipping
a datapack, including when packaging it as a zip.

Existing public function IDs, load/tick tags, native functions, advancements,
worldgen JSON and `schedule function` remain Minecraft resources. The accelerator
is selected per generated public resource during function loading. A default fast
pack runs with **accurate Wasm semantics** when accelerated. Removing the
mod restores its fast fallback; use an accurate build when matching numerical
evolution across both backends matters. Reload resets guest state; live stack or
arbitrary memory migration between backends is not implemented.

## Semantics and integration

- Genuine f32/f64, checked coherent linear memory, native integer wrapping and
  Wasm traps. The guest memory cap matches accurate vanilla: 32,767 64-KiB pages.
- Core operations compile into JVM arithmetic and checked memory operations. Scalar min/max NaN results are
  normalized to accurate mcfunctions' canonical NaN; representation-preserving
  operations retain their bits. Other arithmetic follows the supported accurate
  contract. The bundled low-memory SoftFloat/musl helper pins imported `sinf`,
  `fmodf`, `fminf` and `fmaxf`; `sqrtf` and `fabsf` use native scalar operations.
- Integer command templates support result, success and combined outcome.
  Runtime command text is validated as UTF-8, with the same 32,767-byte limit and
  control-byte rejection as the SDK's vanilla implementation.
- Commands run synchronously through Minecraft's own command dispatcher with
  the caller's source, executor, position, rotation, dimension, anchor and
  permissions. Results are captured once. A child command execution context is
  drained before returning to Wasm, and the enclosing context is restored.
- Calls can nest across native mcfunctions and multiple Wasm modules, including
  A -> B -> A callbacks. Guest globals and memory persist; the Rust ABI stack
  pointer is restored after each public call, including a trapped call. World
  effects before a trap remain applied. A trapped nested public invocation fails
  that Minecraft command without trapping its Wasm caller automatically.
- Selected function bytes and original Wasm must match SHA-256 metadata from the
  **same resource pack**. An edited public function, changed payload, or winning
  native override disables acceleration for that resource. Minecraft pack
  precedence remains authoritative. Hashes are integrity checks, not signatures.
- Guest instances are closed when the function library is replaced. Initialization
  and lazy first calls follow the generated load hooks; no duplicate vanilla and
  accelerated execution occurs.

Like accurate compilation, this release covers the tested scalar subset, not
every Wasm proposal. No WASI, native OS, filesystem, network, thread or GPU imports
are exposed to the guest. Floating command-template arguments, arbitrary custom
host imports, legacy standalone command ABIs and incoming function macro
arguments are outside this accelerator release's supported source-pack ABI.
Unsupported guest imports produce diagnostics. Standalone export manifests used
by the numeric test harness are an internal testing interface, not an additional
author API.

Each outer invocation receives 100 million execution checks, shared by nested
modules, with a maximum of 64 nested public callbacks. Checks at backward
branches and function calls bound loops and recursion and respect Java thread
interruption. These checks are not Wasmtime fuel units or a count of every Wasm
instruction. Existing Minecraft command/fork limits still govern dispatched
commands. Execution-budget, stack and callback exhaustion use trap code 5;
scalar traps retain accurate codes 1–4. Resource cutoffs need not match vanilla
command budgets. Large allocations can still exhaust the JVM heap, so the Wasm
page cap is not a guarantee that every permitted allocation will succeed. Private `#accelerated_calls` scores identify
which backend executed an entry without mirroring guest memory into scoreboards.

## Other Fabric mods

This is a regular Fabric mod, not a replacement Minecraft server. Commands
registered by another mod travel through the same dispatcher as native commands.
A separate compatibility fixture mod validates command registration, permission
checks, return values and exactly-once effects. It is not shipped in the release.

The bridge injects into `ServerFunctionLibrary`'s selected-resource compilation
and `ServerFunctionManager.replaceLibrary`, and accesses the current command
execution context. Mods that replace these paths or assume every command runs
in a single shared queued context may need compatibility work. Passing the
fixture is not a claim of compatibility with every Fabric mod or future snapshot.

## Build the Fabric jar

Install **JDK 25** and **Python 3.11+**. From the repository root:

```sh
python tools/build_accelerator.py --jdk /path/to/jdk-25 --build target/fabric-build
python tools/test_accelerator.py --jdk /path/to/jdk-25 --jar bin/wasmcompute-accelerator-0.1.0.jar --build target/accelerator-tests
```

On Windows, use a path such as `C:/Java/jdk-25`. The result is
`bin/wasmcompute-accelerator-0.1.0.jar`, plus its SHA-256 sidecar. Copy that same
jar into the server's `mods/` folder and restart. Install the complete generated
pack in the world's `datapacks/` folder, then run `/reload`.

The build tool downloads the pinned target's official Minecraft server bundle
and Fabric metadata as compile inputs. Minecraft classes and assets are never
merged into the output. It does not start a server or accept the EULA. To reuse
local build inputs, supply all three of `--server-jar`, `--libraries` and
`--fabric-metadata`; the server jar must be the inner jar from the Mojang bundle.
Rust is needed for the datapack compiler and Rust packs, but not for this mod build.

Chicory and ASM dependencies are pinned by SHA-256 in
`accelerator/java-dependencies.json`. Both are relocated under
`dev.wasmcompute.shaded` to avoid competing with Fabric's ASM or other mods'
versions. Their license notices are included in the jar. Gson is supplied by
Minecraft. The parity adapter uses pinned Chicory compiler internals; an upgrade
requires rerunning the numeric, execution-budget and server integration checks.

There is no silent interpreter fallback. A function that exceeds JVM bytecode
method limits, or an unsupported module, fails compilation with a diagnostic.
Module compilation happens at initialization; its cost is separate from warmed
execution. This release still needs broader cold-start and memory-footprint
profiling for very large packs.

## Validation

The same packaged runtime passed 1,672 checks on Windows and Linux, including
1,648 numeric result-bit/trap vectors, command validation, reentry, memory growth,
stack restoration and loop/recursion limits. The maintained standalone test
fixtures ship in `accelerator/tests`; they require no Minecraft installation.
CI builds a universal jar once, then downloads and tests those exact bytes on
Windows and Linux. It does not start Minecraft or accept its EULA.

Local headless Fabric validation also exercises SDK commands and caller context,
A → B → A → B callbacks with accurate fallback/restoration, and a separate mod's
command registration and exactly-once effects. These tests supplement the numeric
suite; neither establishes every Wasm proposal or every Fabric mod combination.
See [benchmarks](benchmarks.md) for the Java/Wasmtime tradeoff and current demos.
