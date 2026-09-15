# Third-party attribution

The Doom demo vendors the freestanding Linux Doom engine from
https://github.com/theMagicalKarp/wasmdoom at
`dd321b50b89b5085698cfbf2ff01b2f741da8206`. Its GPL-2.0 notice is
`demos/doom/ENGINE-LICENSE`. The shareware IWAD is a separate build input and is
not included in the demo's source tree; locally generated Wasm embeds that input.

The terrain demo vendors a portable subset of SteelMC and steel-math from
https://github.com/Steel-Foundation/SteelMC at
`5fbba4982efced6d7da606cba9fa0f96eec8b188`. Its upstream AGPL-3.0-or-later notice is
`demos/terrain/kernel/STEEL-LICENSE`. This includes adapted noise, climate,
surface, random-source and generation-codegen modules. Minecraft 26.2 registry/worldgen data is an external build input, omitted
from the public source export. Its expected hashes are recorded in
`demos/terrain/kernel/ASSET_INPUTS.json`; generated tables are also excluded. The compiler's 0BSD license does not replace these third-party licenses.

Accurate-mode mcfunctions additionally contain code derived from Berkeley
SoftFloat (BSD license, commit `a0c6494cdc11865811dec815d5c0049fba9d82a8`) and selected
musl math routines (MIT license). Their source, original notices and content hashes
are in `runtime/accurate`. Generated accurate packs include both notices under
`licenses/`; retain these notices when distributing those packs. These components
are independent of wasmcraft2.

The compiler implementation and generated runtime are a new rewrite. wasmcraft2 by SuperTails inspired the task and is used as an external benchmark baseline: https://github.com/SuperTails/wasmcraft2 at 5431ca420e25cc8ec0fe880da4825d083f6937b6. Its source and executable are not distributed in this archive.

Celeste Classic originates with Maddy Thorson and Noel Berry; the measured C port is https://github.com/SuperTails/cceleste-in-mc at 35488069f0a2b04ddfd2d663d31387bf1b1bdbbc. The original game/port source, assets and compiled game Wasm are fetched separately, not distributed here. The adapter and minimal headers in demos/celeste-common were written for this benchmark. Do not infer that the project's 0BSD license licenses third-party game code.

Minecraft is a Mojang/Microsoft product. Server jars and Java distributions are not included. Use of the separate test server is subject to the Minecraft EULA. Server operators must read and accept the EULA themselves before starting a server.

The compiler links open-source Rust dependencies, principally wasmparser, serde/serde_json, clap and anyhow. Their package metadata and original license/notice files are preserved in third-party-licenses/. This inventory includes development/build dependencies as well as runtime dependencies. Cargo.lock contains exact versions and registry checksums.

The separate Serde workload uses serde 1.0.162 and serde-json-core 0.6.0, fetched through Cargo according to its own lockfile. Binaryen and the Python Wasmtime validation package are external tools. The separate Fabric accelerator embeds Chicory 1.7.5 (Apache-2.0) and ASM 9.9.1 (BSD-3-Clause), relocated into its own Java package. Their full license notices are in `accelerator/licenses/` and inside the jar. Its SoftFloat/musl helper retains the notices described above. Wasmtime is a comparison/validation tool and is not shipped in the mod.

The bundled Rust proc-macro SDK and source-pack Serde example also use the dependencies in this inventory. Their Cargo.lock files retain exact registry versions/checksums. Generated example Wasm includes Serde/serde-json-core code and is covered by the corresponding included notices.
