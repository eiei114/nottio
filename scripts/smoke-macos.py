"""Launch the native Mac app in disposable storage; no real boards are touched."""

import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/macos/nottio.app/Contents/MacOS/nottio")
    args = parser.parse_args()
    if sys.platform != "darwin":
        raise SystemExit("Native smoke test requires macOS")
    binary = args.binary.resolve()
    with tempfile.TemporaryDirectory(prefix="nottio-native-smoke-") as directory:
        env = dict(os.environ, NOTTIO_DATA_DIR=str(Path(directory) / "data"))
        with (Path(directory) / "launch.log").open("w+b") as log:
            process = subprocess.Popen([str(binary)], env=env, stdout=log, stderr=log)
            try:
                try:
                    code = process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    print("Native Cocoa/Metal launch stayed alive for 10 seconds")
                else:
                    log.seek(0)
                    raise SystemExit(f"App exited during launch ({code}):\n" + log.read().decode(errors="replace"))
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()


if __name__ == "__main__":
    main()
