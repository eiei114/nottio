"""Launch the native Mac app in disposable storage; no real boards are touched."""

import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    if sys.platform != "darwin":
        raise SystemExit("Native smoke test requires macOS")
    binary = ROOT / "target/macos/nottio.app/Contents/MacOS/nottio"
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
