"""Builds the ZipMount icon from the ready-made set in `tools/icon-set`.

Each size in the set is drawn separately — that is the whole point of it.
Scaling one large drawing down to sixteen pixels is useless: its lines are
thinner than a pixel and turn into grey haze. The set has its own variants
for the small sizes: at 16 the disk has no inner ring, at 32 the lightning
bolt has three zigzags instead of two, and so on.

So 16, 24, 32 and 48 are taken from the set as they are, pixel for pixel.
Sizes the set lacks — 256 for the desktop and the package tile — come from
`zipmount-48-4x.png`: the same drawing at 192 pixels, with resolution to
spare.

    pip install pillow
    python tools/build-icon.py
"""

import struct
from io import BytesIO
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
SET = Path(__file__).resolve().parent / "icon-set"
ASSETS = ROOT / "crates" / "zipmount" / "assets"

# Sizes drawn separately in the set.
EXACT = (16, 24, 32, 48)

# The drawing with resolution to spare — everything else is made from it.
MASTER = SET / "zipmount-48-4x.png"

# Layers of the .ico: from a menu row to a desktop tile.
ICO_SIZES = (256, 48, 32, 24, 16)

# Tiles the sparse package's manifest requires.
TILES = ((44, "Square44x44Logo.png"), (150, "Square150x150Logo.png"), (50, "StoreLogo.png"))

# Logo variants for the shell.
#
# The "ZipMount" row under which Windows groups our context menu items is drawn
# not by the handler but by the shell itself — and it takes the row's icon from
# the package. A plain `Square44x44Logo.png` will not do: the shell wants an
# unplated variant of a particular size and looks for it by file name with
# qualifiers. Without these files the row has no icon.
UNPLATED = (16, 24, 32, 48, 64, 256)

# A separate large icon with a transparent background — for the README, a
# website, anywhere a plain picture is needed rather than an .ico.
LARGE = ROOT / "docs" / "icon-256.png"


def ico(frames):
    """An ICO made of PNG frames: Windows understands those since Vista."""
    header = struct.pack("<HHH", 0, 1, len(frames))
    offset = len(header) + 16 * len(frames)
    entries, payloads = b"", b""
    for size, data in frames:
        entries += struct.pack(
            "<BBBBHHII", size % 256, size % 256, 0, 0, 1, 32, len(data), offset
        )
        payloads += data
        offset += len(data)
    return header + entries + payloads


def tile(size, master):
    """A picture of the given size: from the set, if the set has it."""
    if size in EXACT:
        exact = Image.open(SET / f"zipmount-{size}.png").convert("RGBA")
        if exact.size != (size, size):
            raise SystemExit(f"the set has {exact.size} instead of {size}x{size}")
        return exact
    return master.resize((size, size), Image.LANCZOS)


def main():
    if not MASTER.exists():
        raise SystemExit(f"no icon set: {SET}")
    master = Image.open(MASTER).convert("RGBA")

    ASSETS.mkdir(parents=True, exist_ok=True)
    for size, name in TILES:
        tile(size, master).save(ASSETS / name, format="PNG", optimize=True)
        print(f"{name}: {size}x{size}")

    for size in UNPLATED:
        name = f"Square44x44Logo.altform-unplated_targetsize-{size}.png"
        tile(size, master).save(ASSETS / name, format="PNG", optimize=True)
    print(
        "Square44x44Logo.altform-unplated_targetsize-*: "
        + ", ".join(str(size) for size in UNPLATED)
    )

    LARGE.parent.mkdir(parents=True, exist_ok=True)
    tile(256, master).save(LARGE, format="PNG", optimize=True)
    print(f"{LARGE.name}: 256x256")

    frames = []
    for size in ICO_SIZES:
        buffer = BytesIO()
        tile(size, master).save(buffer, format="PNG", optimize=True)
        frames.append((size, buffer.getvalue()))
    (ASSETS / "ZipMount.ico").write_bytes(ico(frames))

    print(
        "ZipMount.ico: "
        + ", ".join(str(size) for size in ICO_SIZES)
        + " (from the set: "
        + ", ".join(str(size) for size in ICO_SIZES if size in EXACT)
        + ")"
    )


if __name__ == "__main__":
    main()
