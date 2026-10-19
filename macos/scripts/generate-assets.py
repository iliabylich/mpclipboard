#!/usr/bin/env python3

from __future__ import annotations

import json
import subprocess
from pathlib import Path


BASE_ICON = Path("../icon_512x512.png")
ASSETS_DIR = Path("Assets.xcassets")
INFO = {"author": "xcode", "version": 1}

IMAGESET_SIZE = 24
IMAGESET_SCALES = (1, 2, 3)
IMAGESETS = (
    ("green", "rgb(23, 199, 23)"),
    ("red", "rgb(195, 40, 40)"),
    ("yellow", "rgb(236, 223, 41)"),
)

APPICON_COLOR = "rgb(55, 54, 46)"
APPICON_SIZES = (16, 32, 128, 256, 512)
APPICON_SCALES = (1, 2)


def scale_suffix(scale: int) -> str:
    return "" if scale == 1 else f"@{scale}x"


def write_json(path: Path, data) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")


def colorize(color: str, pixels: int, output: Path, density: int | None = None) -> None:
    args = ["magick", str(BASE_ICON)]
    if density is not None:
        args += ["-density", str(density)]
    args += [
        "-resize", f"{pixels}x{pixels}",
        "-alpha", "set",
        "-channel", "rgba",
        "-fill", color,
        "-colorize", "100%",
        "-strip",
        str(output),
    ]
    subprocess.run(args, check=True)


def generate_imageset(name: str, color: str) -> None:
    directory = ASSETS_DIR / f"{name}.imageset"
    directory.mkdir(parents=True, exist_ok=True)

    images = []
    for scale in IMAGESET_SCALES:
        filename = f"{name}{scale_suffix(scale)}.png"
        colorize(color, IMAGESET_SIZE * scale, directory / filename, density=72 * scale)
        images.append({"filename": filename, "idiom": "universal", "scale": f"{scale}x"})

    write_json(directory / "Contents.json", {"images": images, "info": INFO})


def generate_appicon() -> None:
    directory = ASSETS_DIR / "AppIcon.appiconset"
    directory.mkdir(parents=True, exist_ok=True)

    images = []
    for size in APPICON_SIZES:
        for scale in APPICON_SCALES:
            filename = f"icon-{size}x{size}{scale_suffix(scale)}.png"
            colorize(APPICON_COLOR, size * scale, directory / filename)
            images.append(
                {"filename": filename, "idiom": "mac", "scale": f"{scale}x", "size": f"{size}x{size}"}
            )

    write_json(directory / "Contents.json", {"images": images, "info": INFO})


def main() -> None:
    write_json(ASSETS_DIR / "Contents.json", {"info": INFO})

    for name, color in IMAGESETS:
        generate_imageset(name, color)

    generate_appicon()


if __name__ == "__main__":
    main()
