//! Where a board lives on disk, and the autosave/export plumbing around it.
//!
//! Two different stores, deliberately:
//!
//! - the **working board** (`board.json` under `%APPDATA%/nottio`) is opened on
//!   launch and rewritten on every change. It is a cache, not a document, so it
//!   is written directly rather than atomically.
//! - an **export** is a real file the user chose and may copy, sync, or hand to
//!   another app. That one is written temp-then-rename so a crash mid-write can
//!   never leave a truncated document where a valid one used to be.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Directory holding the working board and the small UI state alongside it.
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("nottio")
}

pub fn board_path() -> PathBuf {
    data_dir().join("board.json")
}

pub fn ui_state_path() -> PathBuf {
    data_dir().join("ui-state.json")
}

pub fn read_board(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

/// Overwrite the working board. Errors are returned rather than logged so the
/// caller can surface them in the status line.
pub fn write_board(path: &Path, json: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, json)
}

/// Write `contents` to `path` without ever leaving a half-written file behind.
pub fn write_atomically(path: &Path, contents: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    fs::write(&tmp, contents)?;
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            // Windows refuses a rename onto an existing file. Fall back to a
            // direct write, and clean up the temp file either way.
            let _ = fs::remove_file(&tmp);
            if err.kind() == io::ErrorKind::AlreadyExists
                || err.kind() == io::ErrorKind::PermissionDenied
            {
                fs::write(path, contents)
            } else {
                Err(err)
            }
        }
    }
}

/// Timestamp for the status line: local wall clock as `HH:MM:SS`.
///
/// Deliberately hand-rolled instead of pulling a date crate in for one label.
pub fn clock_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut h = (seconds / 3600) % 24;
    let m = (seconds / 60) % 60;
    let s = seconds % 60;
    // UTC -> JST so the label matches the user's clock without a tz database.
    h = (h + 9) % 24;
    format!("{h:02}:{m:02}:{s:02}")
}

/// The small slice of UI state worth remembering between launches.
///
/// Kept separate from the board so a corrupt or hand-edited board never takes
/// the theme down with it. Stored as a flat `key=value` file rather than JSON —
/// there are three keys, and a parse failure here has no interesting recovery.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct UiState {
    pub dark: bool,
    pub toolbar_vertical: bool,
    /// Board-relative toolbar top-left; `None` means the default top-center.
    pub toolbar_pos: Option<(f32, f32)>,
    pub saved_colors: Vec<u32>,
}

impl UiState {
    pub fn load(path: &Path) -> Self {
        let mut state = Self::default();
        let Ok(text) = fs::read_to_string(path) else {
            return state;
        };
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "dark" => state.dark = value.trim() == "1",
                "toolbar_vertical" => state.toolbar_vertical = value.trim() == "1",
                "toolbar_pos" => {
                    let mut parts = value.trim().split(',');
                    match (parts.next(), parts.next()) {
                        (Some(x), Some(y)) => match (x.trim().parse(), y.trim().parse()) {
                            (Ok(x), Ok(y)) => state.toolbar_pos = Some((x, y)),
                            _ => state.toolbar_pos = None,
                        },
                        _ => state.toolbar_pos = None,
                    }
                }
                "saved_colors" => {
                    // Written as 8-digit uppercase hex, so parse hex: a decimal
                    // parse would silently drop every swatch.
                    state.saved_colors = value
                        .trim()
                        .split(',')
                        .filter_map(|c| u32::from_str_radix(c.trim(), 16).ok())
                        .collect();
                }
                _ => {}
            }
        }
        state
    }

    pub fn save(&self, path: &Path) {
        let pos = match self.toolbar_pos {
            Some((x, y)) => format!("{x},{y}"),
            None => String::new(),
        };
        let colors = self
            .saved_colors
            .iter()
            .map(|c| format!("{c:08X}"))
            .collect::<Vec<_>>()
            .join(",");
        let text = format!(
            "dark={}\ntoolbar_vertical={}\ntoolbar_pos={}\nsaved_colors={}\n",
            u8::from(self.dark),
            u8::from(self.toolbar_vertical),
            pos,
            colors,
        );
        let _ = write_atomically(path, &text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_state_round_trips_through_disk() {
        let dir = std::env::temp_dir().join(format!("nottio-test-{}", std::process::id()));
        let path = dir.join("ui-state.json");
        let state = UiState {
            dark: true,
            toolbar_vertical: true,
            toolbar_pos: Some((12.5, -4.0)),
            saved_colors: vec![0xFF0000FF, 0x00FF00FF],
        };
        state.save(&path);
        assert_eq!(UiState::load(&path), state);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_ui_state_falls_back_to_defaults() {
        let dir = std::env::temp_dir().join(format!("nottio-bad-{}", std::process::id()));
        let path = dir.join("ui-state.json");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, "dark=maybe\ntoolbar_pos=nope\nsaved_colors=zzz\n").unwrap();
        let state = UiState::load(&path);
        assert!(!state.dark);
        assert_eq!(state.toolbar_pos, None);
        assert!(state.saved_colors.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_write_replaces_existing_file() {
        let dir = std::env::temp_dir().join(format!("nottio-atomic-{}", std::process::id()));
        let path = dir.join("board.json");
        write_atomically(&path, "first").unwrap();
        write_atomically(&path, "second").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn clock_is_zero_padded_hh_mm_ss() {
        let clock = clock_now();
        assert_eq!(clock.len(), 8);
        assert_eq!(clock.chars().filter(|c| *c == ':').count(), 2);
    }
}
