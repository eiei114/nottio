"""Rebuild committed Windows icon assets from the generated source (Pillow)."""

from pathlib import Path
import struct

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SIZES = (16, 20, 24, 32, 40, 48, 64, 96, 128, 256)


def main():
    assets = ROOT / "assets"
    image = Image.open(assets / "nottio-icon-source.png").convert("RGBA")
    # Remove near-transparent generation noise outside the silhouette.
    alpha = image.getchannel("A").point(lambda value: 0 if value < 8 else value)
    image.putalpha(alpha)
    image = image.resize((1024, 1024), Image.Resampling.LANCZOS)
    image.save(assets / "nottio.png")
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
