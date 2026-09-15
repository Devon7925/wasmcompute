"""Import exact external Minecraft data from a locally prepared SteelMC checkout."""
import argparse
import hashlib
import json
import shutil
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--steel", type=Path, required=True)
    args = parser.parse_args()
    kernel = Path(__file__).resolve().parents[1] / "demos/terrain/kernel"
    records = json.loads((kernel / "ASSET_INPUTS.json").read_text())["sha256"]
    resources = args.steel / "minecraft-src/minecraft/resources"
    verified = []
    for relative, expected in records.items():
        if relative.startswith("assets/worldgen/"):
            source = resources / "data/minecraft/worldgen" / relative[len("assets/worldgen/"):]
        elif relative.startswith("assets/"):
            source = resources / "datagen-reports" / Path(relative).name
        else:
            source = args.steel / "steel-worldgen" / relative
        if not source.is_file():
            raise SystemExit(f"Missing external input: {source}. See docs/demos.md.")
        if hashlib.sha256(source.read_bytes()).hexdigest() != expected:
            raise SystemExit(f"Input hash mismatch: {source}; expected Minecraft 26.2 inputs.")
        destination = (kernel / relative).resolve()
        if not destination.is_relative_to(kernel.resolve()):
            raise SystemExit("Invalid input manifest path")
        verified.append((source, destination))
    for source, destination in verified:
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
    print(f"Imported {len(verified)} verified external inputs; no game code was imported.")


if __name__ == "__main__":
    main()
