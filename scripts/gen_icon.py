#!/usr/bin/env python3
# Generates Rust Cleaner macOS icons programmatically.
# Exports: main() which creates AppIcon.iconset and PNG assets using Pillow.
# Deps: Pillow for image drawing and gradient fills.

from __future__ import annotations

from pathlib import Path
from typing import Sequence, Tuple

import json

from PIL import Image, ImageDraw

ICON_SIZES: Sequence[int] = (16, 32, 128, 256, 512)
BACKGROUND_START: Tuple[int, int, int] = (0, 75, 145)
BACKGROUND_END: Tuple[int, int, int] = (0, 45, 100)
CAN_BODY_COLOR: Tuple[int, int, int] = (0, 90, 170)
CAN_OUTLINE_COLOR: Tuple[int, int, int] = (0, 50, 110)
NOZZLE_COLOR: Tuple[int, int, int] = (200, 50, 50)
ACCENT_COLOR: Tuple[int, int, int] = (255, 205, 0)
ICNS_ENTRIES: Sequence[Tuple[int, int, str]] = (
    (16, 1, "icp4"),
    (16, 2, "ic11"),
    (32, 1, "icp5"),
    (32, 2, "ic12"),
    (128, 1, "ic07"),
    (128, 2, "ic13"),
    (256, 1, "ic08"),
    (256, 2, "ic14"),
    (512, 1, "ic09"),
    (512, 2, "ic10"),
)


def blend(color_start: Tuple[int, int, int], color_end: Tuple[int, int, int], ratio: float) -> Tuple[int, int, int]:
    return tuple(
        int(color_start[i] + (color_end[i] - color_start[i]) * ratio) for i in range(3)
    )


def create_background(size: int) -> Image.Image:
    gradient = Image.new("RGBA", (1, size))
    drawable = ImageDraw.Draw(gradient)
    for row in range(size):
        ratio = row / (size - 1) if size > 1 else 0.0
        drawable.point((0, row), fill=blend(BACKGROUND_START, BACKGROUND_END, ratio) + (255,))
    gradient = gradient.resize((size, size), resample=Image.BILINEAR)
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, size, size), radius=int(size * 0.18), fill=255)
    background = Image.new("RGBA", (size, size))
    background.paste(gradient, (0, 0), mask)
    return background


def draw_spray_can(canvas: Image.Image) -> None:
    width = canvas.width
    height = canvas.height
    draw = ImageDraw.Draw(canvas)
    body_width = int(width * 0.38)
    body_height = int(height * 0.55)
    body_left = (width - body_width) // 2
    body_top = int(height * 0.3)
    body_right = body_left + body_width
    body_bottom = body_top + body_height
    corner_radius = max(1, body_width // 5)
    outline_width = max(1, width // 64)
    draw.rounded_rectangle(
        (body_left, body_top, body_right, body_bottom),
        radius=corner_radius,
        fill=CAN_BODY_COLOR,
        outline=CAN_OUTLINE_COLOR,
        width=outline_width,
    )
    nozzle_width = max(1, width // 16)
    nozzle_top = body_top - nozzle_width * 2
    draw.rectangle(
        (body_left + body_width // 5, nozzle_top, body_right - body_width // 5, body_top),
        fill=NOZZLE_COLOR,
    )
    draw.rectangle(
        (body_left + body_width // 3, nozzle_top - nozzle_width // 2, body_right - body_width // 3, nozzle_top),
        fill=ACCENT_COLOR,
    )
    highlight_y = body_top + body_height // 4
    draw.line(
        (body_left + body_width // 4, highlight_y, body_right - body_width // 4, highlight_y),
        fill=ACCENT_COLOR,
        width=max(1, outline_width),
    )
    draw.line(
        (body_left + body_width // 4, highlight_y + body_height // 5, body_right - body_width // 4, highlight_y + body_height // 5),
        fill=ACCENT_COLOR,
        width=1,
    )


def compose_icon(size: int) -> Image.Image:
    canvas = create_background(size)
    draw_spray_can(canvas)
    return canvas


def icon_filename(size: int, scale: int) -> str:
    suffix = "@2x" if scale == 2 else ""
    return f"icon_{size}x{size}{suffix}.png"


def write_contents_json(iconset_dir: Path) -> None:
    images = []
    for size in ICON_SIZES:
        for scale in (1, 2):
            images.append(
                {
                    "idiom": "mac",
                    "size": f"{size}x{size}",
                    "scale": f"{scale}x",
                    "filename": icon_filename(size, scale),
                }
            )
    data = {"images": images, "info": {"version": 1, "author": "xcode"}}
    (iconset_dir / "Contents.json").write_text(json.dumps(data, indent=2))


def build_icns(iconset_dir: Path, output_path: Path) -> None:
    chunks = bytearray()
    for size, scale, chunk_code in ICNS_ENTRIES:
        png_path = iconset_dir / icon_filename(size, scale)
        data = png_path.read_bytes()
        length = 8 + len(data)
        chunks.extend(chunk_code.encode("ascii"))
        chunks.extend(length.to_bytes(4, "big"))
        chunks.extend(data)
    header = bytearray(b"icns")
    header.extend((8 + len(chunks)).to_bytes(4, "big"))
    output_path.write_bytes(header + chunks)


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    iconset_dir = root / "AppIcon.iconset"
    iconset_dir.mkdir(exist_ok=True)
    for existing in iconset_dir.iterdir():
        if existing.is_file():
            existing.unlink()
    for base_size in ICON_SIZES:
        icon = compose_icon(base_size)
        icon.save(iconset_dir / f"icon_{base_size}x{base_size}.png", format="PNG")
        retina = compose_icon(base_size * 2)
        retina.save(
            iconset_dir / f"icon_{base_size}x{base_size}@2x.png",
            format="PNG",
        )
    write_contents_json(iconset_dir)
    build_icns(iconset_dir, root / "AppIcon.icns")


if __name__ == "__main__":
    main()
