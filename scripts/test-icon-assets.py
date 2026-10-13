"""Asset regression checks; no source or installed app files are modified."""
import importlib.util
from pathlib import Path
import unittest

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("generate_icon", ROOT / "scripts/generate-icon.py")
generator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(generator)


class IconAssetsTest(unittest.TestCase):
    def test_master_matches_source_and_has_clean_transparent_exterior(self):
        with Image.open(ROOT / "assets/nottio-icon-source.png") as source:
            expected = generator.master_from_source(source)
        with Image.open(ROOT / "assets/nottio.png") as actual:
            self.assertEqual(actual.size, (1024, 1024))
            self.assertEqual(actual.tobytes(), expected.tobytes())
            alpha = actual.getchannel("A")
            self.assertEqual(alpha.crop((0, 0, 1024, 64)).getbbox(), None)
            self.assertEqual(alpha.crop((0, 960, 1024, 1024)).getbbox(), None)
            self.assertEqual(alpha.getpixel((512, 512)), 255)

    def test_ico_has_all_resolutions_and_transparent_corners(self):
        with Image.open(ROOT / "assets/nottio.ico") as icon:
            self.assertEqual(icon.ico.sizes(), {(size, size) for size in generator.SIZES})
            for size in generator.SIZES:
                frame = icon.ico.getimage((size, size)).convert("RGBA")
                self.assertEqual(frame.size, (size, size))
                # Lanczos can leave a <=2/255 ringing value at the smallest size.
                self.assertLessEqual(frame.getpixel((0, 0))[3], 2)
                self.assertGreater(frame.getpixel((size // 2, size // 2))[3], 240)

    def test_unexpected_source_size_requires_explicit_layout_update(self):
        with self.assertRaises(ValueError):
            generator.master_from_source(Image.new("RGBA", (100, 100)))

    def test_mac_icon_retains_the_same_master_and_transparency(self):
        with Image.open(ROOT / "assets/nottio.icns") as icon:
            self.assertEqual(icon.size, (1024, 1024))
            icon.load()
            with Image.open(ROOT / "assets/nottio.png") as master:
                self.assertEqual(icon.convert("RGBA").tobytes(), master.tobytes())


if __name__ == "__main__":
    unittest.main()
