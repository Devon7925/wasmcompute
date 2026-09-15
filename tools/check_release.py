"""CPU-only release checks: source boundary, metadata, docs, licenses and payload hashes."""
import argparse
import hashlib
import json
import os
import re
import tomllib
from pathlib import Path
from export_source import ROOT, WASM, PUBLIC_DOCS, selected, source_files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--public-tree", action="store_true", help="Reject files outside the release allowlist")
    args = parser.parse_args()
    if args.public_tree:
        for directory, dirs, entries in os.walk(ROOT):
            dirs[:] = [d for d in dirs if d not in {".git", "target", "dist", "bin", "generated", "__pycache__"}]
            for entry in entries:
                relative = (Path(directory) / entry).relative_to(ROOT).as_posix()
                assert relative == "SOURCE_SHA256SUMS.json" or selected(relative), f"Non-release file present: {relative}"
    files = list(source_files())
    names = {p.relative_to(ROOT).as_posix() for p in files}
    assert WASM <= names, "Required embedded runtime/example Wasm is absent"
    for name in ["Cargo.toml", "sdk/rust/minecraft/Cargo.toml",
                 "sdk/rust/minecraft-macros/Cargo.toml"]:
        assert tomllib.loads((ROOT / name).read_text())["package"]["license"] == "0BSD", name
    assert json.loads((ROOT / "accelerator/resources/fabric.mod.json").read_text())["license"] == "0BSD"
    for name in ["runtime/accurate/vendor/softfloat/LICENSE", "runtime/accurate/vendor/musl/COPYRIGHT",
                 "demos/doom/ENGINE-LICENSE", "demos/terrain/kernel/STEEL-LICENSE",
                 "accelerator/licenses/CHICORY-LICENSE", "accelerator/licenses/ASM-LICENSE"]:
        assert name in names and (ROOT / name).stat().st_size > 500, name
    for forbidden in ["bin/compiler.exe", "demos/doom/game.wad", "reports/foo/game.wasm",
                      "demos/terrain/kernel/assets/blocks.json", "demos/terrain/kernel/src/generated/x.rs",
                      "tools/launch_demo.py", "tools/benchmark.py", "tools/read_world_blocks.py",
                      "reports/demo-expansion/REPORT.md", "benchmarks/celeste/harness.cpp",
                      "docs/images/README.md", "docs/reproduce.md", "docs/release-validation.md",
                      "src/target/x.rs", "accelerator/native/Cargo.toml",
                      "accelerator/lib.so", "accelerator/BUILD.json", "server.properties"]:
        assert not selected(forbidden), forbidden
    assert "Permission to use, copy, modify" in (ROOT / "LICENSE").read_text()
    primary_docs = ["README.md", "CONTRIBUTING.md", "SECURITY.md", "THIRD_PARTY.md", "CHANGELOG.md",
                    *["docs/" + name for name in sorted(PUBLIC_DOCS) if name.endswith(".md")],
                    "examples/hello/README.md", "examples/source-pack/README.md",
                    "demos/celeste-direct/README.md", "demos/doom/README.md", "demos/terrain/README.md"]
    links = 0
    for name in primary_docs:
        path = ROOT / name
        text = path.read_text(encoding="utf-8")
        assert not re.search(r'(?:\.\./|\b)(?:reports|benchmarks)/|reproduce\.md|images/README\.md', text), name
        targets = re.findall(r'\]\(([^)]+)\)', text) + re.findall(r'src="([^"]+)"', text)
        for target in targets:
            if "://" in target or target.startswith("#"):
                continue
            destination = (path.parent / target.split("#")[0]).resolve()
            assert destination.is_relative_to(ROOT.resolve()) and destination.exists(), (name, target)
            relative = destination.relative_to(ROOT.resolve()).as_posix()
            assert relative in names or any(n.startswith(relative + "/") for n in names), (name, target, "omitted from export")
            links += 1
    for path in files:
        relative = path.relative_to(ROOT).as_posix()
        assert path.stat().st_size < 50_000_000, f"Oversized source file: {relative}"
        if path.suffix in {".json", ".mcmeta"}:
            json.loads(path.read_text(encoding="utf-8-sig"))
    manifest = ROOT / "SOURCE_SHA256SUMS.json"
    if manifest.exists():
        for name, expected in json.loads(manifest.read_text()).items():
            assert hashlib.sha256((ROOT / name).read_bytes()).hexdigest() == expected, name
    print(f"Release checks passed: {len(files)} source files, {links} local guide links, licenses and JSON valid.")


if __name__ == "__main__":
    main()
