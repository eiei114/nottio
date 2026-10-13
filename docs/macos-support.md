# macOS support

[English (default)](../README.md) · [日本語](readme-ja.md) · [简体中文](readme-zh-cn.md)

## Platform contract

- macOS 14+; native Apple Silicon and Intel builds, not a universal binary.
- Same pinned GPUI 0.3.9 graph as Windows, native Cocoa/Metal backend.
- macOS-only `runtime_shaders` embeds Metal source and compiles it on launch;
  Command Line Tools suffice without the offline Metal compiler.
- Working board and UI state: `~/Library/Application Support/nottio/`.
  `NOTTIO_DATA_DIR` remains an explicit override. Windows' APPDATA is unchanged.
- Standard edit shortcuts accept Command. Control-based connector gestures stay
  Control-based. File actions: Command+O / Command+Shift+S. Command+Q saves first.
- GPUI reads native PNG/TIFF clipboard entries; the host normalizes them to the
  same self-contained PNG board format. Board JSON is shared across platforms.
- Finder bundle has Info.plist, executable, ICNS, and third-party licenses.
  Local ad-hoc signing is not Developer ID signing or notarization.

## Automated verification

The `Native builds` workflow builds/tests/clippy-checks on Windows and native
macOS runners (`macos-15` / `macos-15-intel`), then generates and verifies each
Mac app bundle. The job reports `rustc -vV` and `file` output, so architecture is
evidence, not an assumption. CI bundles are zipped with `ditto` to retain modes.
The smoke step launches the native bundle executable in isolated storage and
checks that it stays alive. This does not exercise every mouse or IME path.

Windows can run the platform-path policies, Command-key test-platform image
paste/copy/undo/redo, pending-write flush, TIFF decoding, ICNS asset checks, and
bundle layout/safe-rebuild regressions. Windows cannot link Cocoa/Metal or
certify native Mac UI behavior; that is why native CI is included.

## Physical Mac acceptance checklist

Use `NOTTIO_DATA_DIR` with a disposable folder; never clear a real working board.

1. Build and open `target/macos/nottio.app` from Finder. Check Dock icon and menu.
2. Drag a sticky from the toolbar, enter English/Japanese/Chinese with the native
   IME, Escape, move it and reopen its text without duplicating composition.
3. Paste a screenshot and a TIFF-backed image with Command+V. Resize, undo,
   copy/paste, save/reopen. Text editing must still paste text instead.
4. Connect notes, drag portions, move overlapping frames. Control bypasses snap.
5. Open/save-copy dialogs and Command shortcuts work; trackpad pan/zoom works.
6. Quit immediately after editing via Command+Q and Dock Quit, relaunch and check
   the last change. Test an unwritable disposable path for a visible save error.
7. Check both Apple Silicon and Intel on the stated minimum OS; CI's newer OS is
   not proof of all Sonoma/driver combinations.

Unverified until performed: physical Mac IME/accessibility/trackpad behavior,
minimum-OS GUI acceptance, signed distribution, notarization and installers.
