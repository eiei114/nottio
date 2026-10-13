//! The colour palette.
//!
//! gpui-whiteboard reads its palette through a closure on every paint, so the
//! theme lives in one `Copy` struct that `app.rs` can snapshot for the closure
//! and `ui.rs` can use for its own chrome.

use gpui::{rgba, Hsla};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeKind {
    Light,
    Dark,
}

impl ThemeKind {
    pub fn toggled(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }

    pub fn from_dark(dark: bool) -> Self {
        if dark {
            Self::Dark
        } else {
            Self::Light
        }
    }

    pub fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }
}

/// One palette, as opaque 8-bit channels.
///
/// Held as hex rather than `Hsla` so it stays `Copy + Eq` and independent of the
/// gpui colour types; `into_style` does the conversion at the boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub bg: u32,
    pub grid: u32,
    pub text: u32,
    pub ink: u32,
    pub panel: u32,
    pub panel_strong: u32,
    pub accent: u32,
    pub selection: u32,
    pub border: u32,
    pub danger: u32,
    pub swatches: [u32; 6],
}

const LIGHT: Palette = Palette {
    bg: 0xF7F7F5FF,
    grid: 0xCFCFCBFF,
    text: 0x6B6B66FF,
    ink: 0x1F1F23FF,
    panel: 0xFFFFFFF2,
    panel_strong: 0xFFFFFFFF,
    accent: 0xE8EEFDFF,
    selection: 0x2E6BE6FF,
    border: 0xD8D8D4FF,
    danger: 0xC2403AFF,
    swatches: [
        0x3563E9FF, 0xE24B4BFF, 0xF2A93BFF, 0x3FA45BFF, 0x8A5CF6FF, 0x1F1F23FF,
    ],
};

const DARK: Palette = Palette {
    bg: 0x17171AFF,
    grid: 0x35353AFF,
    text: 0x9A9AA2FF,
    ink: 0xE8E8ECFF,
    panel: 0x26262BF2,
    panel_strong: 0x2C2C33FF,
    accent: 0x24304CFF,
    selection: 0x6E9BFFFF,
    border: 0x3C3C43FF,
    danger: 0xE06C64FF,
    swatches: [
        0x6E9BFFFF, 0xE06C64FF, 0xF2C14EFF, 0x63C77EFF, 0xB18CFFFF, 0xE8E8ECFF,
    ],
};

pub fn palette(kind: ThemeKind) -> Palette {
    match kind {
        ThemeKind::Light => LIGHT,
        ThemeKind::Dark => DARK,
    }
}

/// `WhiteboardStyle` takes `Hsla`, so the palette crosses over here.
pub fn style(kind: ThemeKind) -> gpui_whiteboard::WhiteboardStyle {
    let p = palette(kind);
    gpui_whiteboard::WhiteboardStyle {
        bg: rgba(p.bg).into(),
        grid: rgba(p.grid).into(),
        text: rgba(p.text).into(),
        ink: rgba(p.ink).into(),
        panel: rgba(p.panel).into(),
        panel_strong: rgba(p.panel_strong).into(),
        accent: rgba(p.accent).into(),
        selection: rgba(p.selection).into(),
        swatches: p
            .swatches
            .iter()
            .map(|hex| -> Hsla { rgba(*hex).into() })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_palette_keeps_rgb_channels_and_alpha() {
        let color = rgba(LIGHT.bg);
        assert_eq!(color.r, 247.0 / 255.0);
        assert_eq!(color.g, 247.0 / 255.0);
        assert_eq!(color.b, 245.0 / 255.0);
        assert_eq!(color.a, 1.0);
        assert_eq!(rgba(LIGHT.panel).a, 242.0 / 255.0);
    }

    #[test]
    fn toggling_a_theme_returns_the_other_one() {
        assert_eq!(ThemeKind::Light.toggled(), ThemeKind::Dark);
        assert_eq!(ThemeKind::Dark.toggled(), ThemeKind::Light);
        // Toggling twice returns to where it started, from either side.
        assert_eq!(ThemeKind::Light.toggled().toggled(), ThemeKind::Light);
        assert_eq!(ThemeKind::Dark.toggled().toggled(), ThemeKind::Dark);
    }

    #[test]
    fn from_dark_matches_is_dark() {
        assert!(ThemeKind::from_dark(true).is_dark());
        assert!(!ThemeKind::from_dark(false).is_dark());
    }

    #[test]
    fn style_surfaces_stay_distinct_so_the_picker_reads_over_the_canvas() {
        for kind in [ThemeKind::Light, ThemeKind::Dark] {
            let s = style(kind);
            assert_ne!(s.bg, s.panel_strong);
            assert_ne!(s.bg, s.selection);
            assert_eq!(s.swatches.len(), 6);
        }
    }
}
