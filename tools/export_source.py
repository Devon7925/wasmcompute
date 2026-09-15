"""Export a public source tree without local builds, game payloads or server data."""
import argparse
import hashlib
import json
import os
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOPS = {".github", "accelerator", "demos", "docs", "examples",
        "runtime", "sdk", "src", "tests", "third-party-licenses", "tools"}
ROOT_FILES = {"Cargo.toml", "Cargo.lock", "README.md", "LICENSE", "THIRD_PARTY.md",
              "CONTRIBUTING.md", "SECURITY.md", "CHANGELOG.md", ".gitignore", ".gitattributes"}
WASM = {"src/accurate_runtime.wasm", "src/patterns/forward-copy.wasm",
        "accelerator/runtime/math.wasm", "accelerator/tests/numeric.wasm",
        "accelerator/tests/guards.wasm", "examples/rust_commands.wasm"}
EXTERNAL = {"demos/terrain/kernel/assets", "demos/terrain/kernel/build_assets"}
LOCAL_TOOLS = {"tools/launch_demo.py", "tools/launch_celeste.py", "tools/prepare_celeste_client.py"}

PUBLIC_TOOLS = {
    "build_accelerator.py", "build_accurate_runtime.py", "import_terrain_assets.py",
    "export_source.py", "package_release.py", "check_release.py", "ci_smoke.py", "verify_bundle.py",
    "test_accelerator.py",
}
PUBLIC_DOCS = {
    "accelerator.md", "accurate.md", "architecture.md", "benchmarks.md", "demos.md",
    "other-languages.md", "releasing.md", "semantics.md", "source-packs.md",
    "images/celeste.png", "images/doom.png",
}
PUBLIC_DEMOS = {"celeste-direct", "celeste-common", "doom", "terrain"}


def selected(relative):
    p = Path(relative)
    parts = p.parts
    name = p.as_posix()
    if len(parts) == 1:
        return name in ROOT_FILES
    if parts[0] not in TOPS or {"target", "generated", "__pycache__", ".git", ".pytest_cache"}.intersection(parts):
        return False
    if name == "accelerator/BUILD.json":
        return False
    if name.startswith("accelerator/native/"):
        return False
    if parts[0] == "tools" and name.removeprefix("tools/") not in PUBLIC_TOOLS:
        return False
    if parts[0] == "docs" and name.removeprefix("docs/") not in PUBLIC_DOCS:
        return False
    if parts[0] == "demos" and parts[1] not in PUBLIC_DEMOS:
        return False
    if any(name.startswith(prefix + "/") for prefix in EXTERNAL) or name in LOCAL_TOOLS:
        return False
    if p.suffix.lower() in {".exe", ".dll", ".so", ".dylib", ".jar", ".wad", ".zip", ".jfr", ".log", ".pyc", ".dat", ".rgb"}:
        return False
    if p.suffix == ".wasm" and name not in WASM:
        return False
    if p.name in {"server.properties", "ops.json", "whitelist.json", "usercache.json"} or p.name.startswith(".env"):
        return False
    return True


def source_files(root=ROOT):
    for directory, dirs, files in os.walk(root):
        here = Path(directory)
        dirs[:] = [d for d in dirs if d not in {"target", "generated", "__pycache__", ".git", "dist"}
                   and ((here / d).relative_to(root).parts[0] in TOPS)]
        for name in sorted(files):
            path = here / name
            if selected(path.relative_to(root)):
                if path.is_symlink() or not path.resolve().is_relative_to(root.resolve()):
                    raise ValueError(f"Source symlink escapes export policy: {path}")
                yield path


def public_bytes(path, root=ROOT):
    data = path.read_bytes()
    # Avoid exporting accidental machine-specific paths in maintained documentation.
    if path.relative_to(root).parts[0] == "docs" and path.suffix in {".md", ".json", ".txt"}:
        try:
            text = data.decode("utf-8-sig")
        except UnicodeDecodeError:
            text = data.decode("cp1252")
        workspace = root.parent.parent
        variants = {str(workspace), workspace.as_posix(), str(workspace).replace("\\", "\\\\")}
        for prefix in sorted(variants, key=len, reverse=True):
            text = text.replace(prefix, "<workspace>")
        text = re.sub(r"C:[/\\]+Users[/\\]+impor(?=[/\\])", "<user>", text, flags=re.I)
        data = text.encode("utf-8")
    return data


def export(destination, root=ROOT):
    destination = Path(destination).resolve()
    root = root.resolve()
    if destination == root or destination.is_relative_to(root) or root.is_relative_to(destination):
        raise ValueError("Export must be outside the source tree and cannot contain it")
    if destination.exists() and any(destination.iterdir()):
        raise ValueError("Export destination must be empty; existing files are never removed")
    destination.mkdir(parents=True, exist_ok=True)
    hashes = {}
    for path in sorted(source_files(root)):
        relative = path.relative_to(root)
        data = public_bytes(path, root)
        out = destination / relative
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_bytes(data)
        hashes[relative.as_posix()] = hashlib.sha256(data).hexdigest()
    (destination / "SOURCE_SHA256SUMS.json").write_text(json.dumps(hashes, indent=2) + "\n")
    return hashes


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    hashes = export(args.output)
    print(f"Exported {len(hashes)} files to {args.output}; source hashes recorded.")
