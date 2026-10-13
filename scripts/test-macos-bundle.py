"""Cross-platform packaging regressions; no Cargo build or user data access."""

import importlib.util
from pathlib import Path
import plistlib
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("macos_bundle", Path(__file__).with_name("build-macos-app.py"))
bundle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bundle)


class MacBundleTest(unittest.TestCase):
    def test_bundle_layout_identity_icon_and_rebuild(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / "nottio"
            binary.write_bytes(b"\xcf\xfa\xed\xfe" + b"test fixture")
            output = root / "nottio.app"
            for _ in range(2):
                bundle.build_bundle(binary, output)
                with (output / "Contents/Info.plist").open("rb") as source:
                    info = plistlib.load(source)
                self.assertEqual(info["CFBundleIdentifier"], bundle.BUNDLE_ID)
                self.assertEqual(info["LSMinimumSystemVersion"], "14.0")
                self.assertEqual(info["CFBundleExecutable"], "nottio")
                self.assertEqual((output / "Contents/MacOS/nottio").read_bytes(), binary.read_bytes())
                self.assertEqual((output / "Contents/Resources/nottio.icns").read_bytes(),
                                 (bundle.ROOT / "assets/nottio.icns").read_bytes())
                self.assertEqual(len(list((output / "Contents/Resources/licenses").iterdir())), 3)

    def test_foreign_bundle_and_non_mac_binary_are_not_overwritten(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / "nottio.exe"
            binary.write_bytes(b"MZwindows")
            output = root / "nottio.app"
            with self.assertRaises(ValueError):
                bundle.build_bundle(binary, output)
            self.assertFalse(output.exists())
            output.mkdir()
            sentinel = output / "personal-file"
            sentinel.write_text("keep", encoding="utf-8")
            with self.assertRaises(ValueError):
                bundle.build_bundle(binary, output)
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "keep")


if __name__ == "__main__":
    unittest.main()
