"""Rebuild committed Windows and macOS icon assets (Pillow)."""

from pathlib import Path
import struct

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
SIZES = (16, 20, 24, 32, 40, 48, 64, 96, 128, 256)


def master_from_source(image):
    """Normalize this commissioned artwork; exclude noisy generated alpha edges."""
    if image.size != (1254, 1254):
        raise ValueError("Update the artwork crop when replacing the source dimensions")
    # This bounds the actual porcelain tile, not the scattered exterior pixels.
    tile = image.convert("RGBA").crop((72, 87, 1182, 1176))
    tile = tile.resize((896, 896), Image.Resampling.LANCZOS)
    clean = Image.new("RGBA", tile.size, (250, 249, 246, 255))
    clean.alpha_composite(tile)
    # Analytic, supersampled silhouette: no generator speckles or ragged rim.
    mask = Image.new("L", (896 * 4, 896 * 4), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, 896 * 4 - 1, 896 * 4 - 1), radius=140 * 4, fill=255)
    clean.putalpha(mask.resize(tile.size, Image.Resampling.LANCZOS))
    master = Image.new("RGBA", (1024, 1024), (0, 0, 0, 0))
    master.paste(clean, (64, 64))
    return master


def main():
    assets = ROOT / "assets"
    with Image.open(assets / "nottio-icon-source.png") as source:
        image = master_from_source(source)
    image.save(assets / "nottio.png")
    image.save(assets / "nottio.icns", format="ICNS")
    icon = assets / "nottio.ico"
    image.save(icon, sizes=[(size, size) for size in SIZES])
    data = icon.read_bytes()
    assert struct.unpack_from("<HHH", data) == (0, 1, len(SIZES))
    actual = set()
    for index in range(len(SIZES)):
        w, h, _, _, _, _, length, offset = struct.unpack_from("<BBBBHHII", data, 6 + index * 16)
        actual.add((w or 256, h or 256))
        assert length > 0 and offset + length <= len(data)
    assert actual == {(size, size) for size in SIZES}
    print(f"Generated {icon} with {len(SIZES)} resolutions")


if __name__ == "__main__":
    main()
