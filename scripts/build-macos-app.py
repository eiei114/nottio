"""Build a Finder-launchable nottio.app with the committed icon (Python 3.11+).

Local/ad-hoc signed build only: no Apple account, notarization or installer.
"""

import argparse
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
BUNDLE_ID = "com.eiei114.nottio"
MARKER = ".nottio-build"


def build_bundle(binary, output):
    binary, output = Path(binary).resolve(), Path(output).absolute()
    if output.suffix != ".app" or output.is_symlink():
        raise ValueError("Output must be a non-symlink .app directory")
    if output.exists() and not (output / MARKER).is_file():
        raise ValueError("Refusing to replace a bundle not created by this script")
    with binary.open("rb") as source:
        if source.read(4) not in (b"\xcf\xfa\xed\xfe", b"\xfe\xed\xfa\xcf", b"\xca\xfe\xba\xbe", b"\xbe\xba\xfe\xca"):
            raise ValueError("Expected a macOS Mach-O executable, not a Windows binary")
    with (ROOT / "Cargo.toml").open("rb") as manifest:
        version = tomllib.load(manifest)["package"]["version"]
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="nottio-bundle-", dir=output.parent) as staging:
        app = Path(staging) / "nottio.app"
        contents = app / "Contents"
        executable = contents / "MacOS/nottio"
        executable.parent.mkdir(parents=True)
        resources = contents / "Resources"
        resources.mkdir()
        shutil.copy2(binary, executable)
        executable.chmod(0o755)
        shutil.copy2(ROOT / "assets/nottio.icns", resources / "nottio.icns")
        notices = resources / "licenses"
        notices.mkdir()
        for source, name in (
            ("vendor/gpui-whiteboard/LICENSE", "gpui-whiteboard.txt"),
            ("vendor/gpui-whiteboard/assets/icons/LICENSE", "icons.txt"),
            ("vendor/gpui-whiteboard/assets/JetBrainsMono-OFL.txt", "jetbrains-mono.txt"),
        ):
            shutil.copy2(ROOT / source, notices / name)
        info = {
            "CFBundleName": "nottio",
            "CFBundleDisplayName": "nottio",
            "CFBundleIdentifier": BUNDLE_ID,
            "CFBundleExecutable": "nottio",
            "CFBundlePackageType": "APPL",
            "CFBundleIconFile": "nottio.icns",
            "CFBundleShortVersionString": version,
            "CFBundleVersion": version,
            "LSMinimumSystemVersion": "14.0",
            "NSHighResolutionCapable": True,
        }
        with (contents / "Info.plist").open("wb") as target:
            plistlib.dump(info, target)
        (contents / "PkgInfo").write_bytes(b"APPL????")
        (app / MARKER).write_text(BUNDLE_ID, encoding="utf-8")
        if output.exists():
            if (output / MARKER).read_text(encoding="utf-8") != BUNDLE_ID:
                raise ValueError("Existing bundle has an unexpected ownership marker")
            shutil.rmtree(output)
        shutil.move(str(app), str(output))
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--no-build", action="store_true", help="Package an already-built binary")
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/nottio")
    parser.add_argument("--output", type=Path, default=ROOT / "target/macos/nottio.app")
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("Run this command on a Mac with the Apple SDK installed")
    if not args.no_build:
        subprocess.run(["cargo", "build", "--locked", "--release"], cwd=ROOT, check=True)
    app = build_bundle(args.binary, args.output)
    subprocess.run(["codesign", "--force", "--sign", "-", str(app)], check=True)
    subprocess.run(["codesign", "--verify", "--strict", str(app)], check=True)
    print(f"Built {app}\nLaunch: open '{app}'")


if __name__ == "__main__":
    main()
