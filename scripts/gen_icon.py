#!/usr/bin/env python3
# Generates WD-40's macOS app icon: one spray can on a squircle, nothing else.
# Exports: main(); writes AppIcon.iconset, AppIcon.icns, docs/icon-preview.png.
# Deps: Pillow for drawing and downsampling; iconutil to pack the .icns.

from __future__ import annotations

import math
import subprocess
from pathlib import Path
from typing import Sequence, Tuple

from PIL import Image, ImageDraw, ImageFilter

RGB = Tuple[int, int, int]

MASTER = 1024
# Draw large and downsample once: cheap antialiasing without per-shape work.
SUPERSAMPLE = 4
ICON_SIZES: Sequence[int] = (16, 32, 128, 256, 512)

# Apple's icon silhouette is a superellipse, not a rounded rectangle.
SQUIRCLE_N = 5.0
SQUIRCLE_INSET = 0.035

BG_TOP: RGB = (86, 170, 240)
BG_BOTTOM: RGB = (14, 74, 158)
CAN_LIGHT: RGB = (247, 249, 252)
CAN_SHADE: RGB = (205, 215, 230)
BAND: RGB = (255, 199, 44)
CAP: RGB = (222, 49, 41)
NOZZLE: RGB = (58, 66, 80)


def superellipse(box: Tuple[float, float, float, float], n: float, steps: int = 720):
    """Parametric superellipse points inscribed in `box`."""
    left, top, right, bottom = box
    center_x, center_y = (left + right) / 2.0, (top + bottom) / 2.0
    half_w, half_h = (right - left) / 2.0, (bottom - top) / 2.0
    exponent = 2.0 / n
    points = []
    for step in range(steps):
        theta = 2.0 * math.pi * step / steps
        x = center_x + half_w * _signed_pow(math.cos(theta), exponent)
        y = center_y + half_h * _signed_pow(math.sin(theta), exponent)
        points.append((x, y))
    return points


def _signed_pow(value: float, exponent: float) -> float:
    magnitude = abs(value) ** exponent
    return magnitude if value >= 0 else -magnitude


def vertical_gradient(size: int, top: RGB, bottom: RGB) -> Image.Image:
    column = Image.new("RGB", (1, size))
    pixels = column.load()
    for y in range(size):
        ratio = y / max(size - 1, 1)
        pixels[0, y] = tuple(
            round(top[channel] + (bottom[channel] - top[channel]) * ratio) for channel in range(3)
        )
    return column.resize((size, size), Image.BILINEAR)


def draw_can(canvas: ImageDraw.ImageDraw, size: int) -> None:
    """One upright can: nozzle, cap, body, band. Nothing that vanishes at 16px."""
    unit = size / 1024.0
    body_w, body_h = 350.0 * unit, 452.0 * unit
    left = (size - body_w) / 2.0
    top = size * 0.415
    radius = 48.0 * unit

    cap_w, cap_h = 182.0 * unit, 108.0 * unit
    cap_left = (size - cap_w) / 2.0
    cap_top = top - cap_h + 14.0 * unit
    canvas.rounded_rectangle(
        (cap_left, cap_top, cap_left + cap_w, cap_top + cap_h), radius=28.0 * unit, fill=CAP
    )

    nozzle_w, nozzle_h = 46.0 * unit, 44.0 * unit
    nozzle_left = (size - nozzle_w) / 2.0
    canvas.rounded_rectangle(
        (nozzle_left, cap_top - nozzle_h + 10.0 * unit, nozzle_left + nozzle_w, cap_top + 12.0 * unit),
        radius=15.0 * unit,
        fill=NOZZLE,
    )

    body_box = (left, top, left + body_w, top + body_h)
    canvas.rounded_rectangle(body_box, radius=radius, fill=CAN_LIGHT)
    band_top = top + body_h * 0.40
    canvas.rectangle((left, band_top, left + body_w, band_top + body_h * 0.24), fill=BAND)


def apply_body_shading(can: Image.Image, size: int) -> Image.Image:
    """Darken the right edge of the can, clipped to whatever the can already is,
    so the shading never shows an outline of its own."""
    unit = size / 1024.0
    body_w = 350.0 * unit
    left = (size - body_w) / 2.0

    shade = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    ImageDraw.Draw(shade).rectangle((left + body_w * 0.72, 0, size, size), fill=(22, 42, 72, 46))
    shade.putalpha(Image.composite(shade.getchannel("A"), Image.new("L", (size, size), 0), can.getchannel("A")))
    return Image.alpha_composite(can, shade)


def render_master() -> Image.Image:
    size = MASTER * SUPERSAMPLE
    inset = size * SQUIRCLE_INSET
    box = (inset, inset, size - inset, size - inset)

    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).polygon(superellipse(box, SQUIRCLE_N), fill=255)

    background = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    background.paste(vertical_gradient(size, BG_TOP, BG_BOTTOM), (0, 0), mask)

    # Contact shadow on its own layer, clipped to the squircle.
    shadow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).ellipse(
        (size * 0.31, size * 0.845, size * 0.69, size * 0.895), fill=(6, 32, 70, 130)
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(size * 0.018))
    empty = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    icon = Image.alpha_composite(background, Image.composite(shadow, empty, mask))

    can = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw_can(ImageDraw.Draw(can), size)
    icon = Image.alpha_composite(icon, apply_body_shading(can, size))

    return icon.resize((MASTER, MASTER), Image.LANCZOS)


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    master = render_master()

    docs = root / "docs"
    docs.mkdir(exist_ok=True)
    master.save(docs / "icon-preview.png")

    iconset = root / "AppIcon.iconset"
    iconset.mkdir(exist_ok=True)
    for size in ICON_SIZES:
        master.resize((size, size), Image.LANCZOS).save(iconset / f"icon_{size}x{size}.png")
        double = size * 2
        master.resize((double, double), Image.LANCZOS).save(iconset / f"icon_{size}x{size}@2x.png")

    subprocess.run(
        ["iconutil", "-c", "icns", str(iconset), "-o", str(root / "AppIcon.icns")], check=True
    )
    print(f"wrote {root / 'AppIcon.icns'} and {docs / 'icon-preview.png'}")


if __name__ == "__main__":
    main()
