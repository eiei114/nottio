//! Host chrome, and the small bridge between the window and gpui-whiteboard.
//!
//! gpui-whiteboard owns its entire editing surface — toolbar, flyouts, colour
//! picker, right-click menu, canvas, keyboard map — so the host contributes
//! almost nothing: a status strip, a theme handle, and the hooks the board asks
//! the host to install. Keeping this file thin is the point; if chrome starts
//! growing here, it probably belongs to the board instead.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{div, prelude::*, px, rgba, Entity, Hsla, SharedString};
use gpui_whiteboard::{WhiteboardStyleFn, WhiteboardView};

use crate::app::Status;
use crate::theme::{self, Palette, ThemeKind};

/// A theme the board can see change after it captured its style closure.
///
/// `WhiteboardView::new` takes the style as a `Fn() -> WhiteboardStyle` and
/// reads it on every paint, but it never hands that closure back — there is no
/// `set_style`. So the closure captures this handle rather than a value, and
/// flipping the handle re-themes the live board without rebuilding it (which
/// would discard the undo history, the camera, and the selection).
#[derive(Clone)]
pub struct ThemeHandle(Rc<Cell<ThemeKind>>);

impl ThemeHandle {
    pub fn new(kind: ThemeKind) -> Self {
        Self(Rc::new(Cell::new(kind)))
    }

    pub fn get(&self) -> ThemeKind {
        self.0.get()
    }

    /// Flip the theme and return the new one, so the caller can persist it.
    pub fn toggle(&self) -> ThemeKind {
        let next = self.0.get().toggled();
        self.0.set(next);
        next
    }

    pub fn style_fn(&self) -> WhiteboardStyleFn {
        let handle = self.clone();
        Rc::new(move || theme::style(handle.get()))
    }
}

/// Status text plus the resolved palette, computed once per render.
pub struct Chrome {
    pub palette: Palette,
    pub status: Status,
}

/// Lay out the status strip and board. Tools live in the board's own toolbar.
pub fn render(chrome: &Chrome, board: Entity<WhiteboardView>) -> gpui::Stateful<gpui::Div> {
    let palette = chrome.palette;
    let label = chrome.status.label();
    let color: Hsla = if chrome.status.is_error() {
        rgba(palette.danger).into()
    } else {
        rgba(palette.text).into()
    };
    // An always-present strip, empty when there is nothing to report, so the
    // canvas never shifts up and down as saves land.
    let strip = div()
        .flex()
        .items_center()
        .gap_3()
        .h(px(30.))
        .flex_none()
        .px_3()
        .bg(rgba(palette.panel_strong))
        .border_b_1()
        .border_color(rgba(palette.border))
        .text_xs()
        .whitespace_nowrap()
        .child(
            div()
                .flex_none()
                .text_color(color)
                .child(SharedString::from(label)),
        )
        .child(div().flex_1());

    // The id makes the root a stateful element so the caller can attach
    // `on_key_down` to it — keys the board does not consume bubble up to here.
    div()
        .id("nottio-root")
        .flex()
        .flex_col()
        .size_full()
        .bg(rgba(palette.bg))
        .child(strip)
        .child(
            div()
                .flex_1()
                .flex_basis(px(0.))
                .overflow_hidden()
                .relative()
                .child(board),
        )
}

/// Does this clipboard payload look like a serialized board selection?
///
/// The paste hook returns `None` for anything else, which lets the keystroke
/// fall through to the board's own text editing. A bare `[` is not enough —
/// plain JSON arrays are common — so the element discriminator has to be there
/// too.
pub fn looks_like_elements(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with('[') && trimmed.contains("\"kind\"")
}

/// Loose sanity check before an opened file is allowed to replace the board.
///
/// `Scene` gives every field a serde default, so *any* JSON object parses into
/// a valid (possibly empty) board. There is therefore no strict validation to
/// do; the goal here is only to stop a text file or an unrelated JSON document
/// from silently blanking the working board.
pub fn looks_like_board(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with('{') && (trimmed.contains("\"elements\"") || trimmed.contains("\"camera\""))
}

/// Repaint the host chrome from a hook that only has an `App`.
///
/// The board repaints itself; this is for the status strip, which the root view
/// owns. The root view is reached through the window because there is no point
/// at which its handle could be captured — see `app::with_root`.
pub fn repaint(cx: &mut gpui::App) {
    crate::app::notify_root(cx);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_shape_check_rejects_plain_text_and_arrays() {
        assert!(!looks_like_elements("just a note"));
        assert!(!looks_like_elements("[1, 2, 3]"));
        // The real shape: `Element::kind` is a field whose value is an
        // externally-tagged enum, e.g. {"rect": {..}}.
        assert!(looks_like_elements(r#"[{"id":0,"kind":{"rect":{}}}]"#));
        assert!(looks_like_elements(r#"[{"id":3,"kind":{"arrow":{}}}]"#));
    }

    #[test]
    fn board_shape_check_rejects_non_boards() {
        assert!(!looks_like_board("hello"));
        assert!(!looks_like_board("[1,2,3]"));
        assert!(looks_like_board(r#"{"camera":{"x":0.0},"elements":[]}"#));
    }

    #[test]
    fn theme_handle_toggles_in_place() {
        let handle = ThemeHandle::new(ThemeKind::Light);
        assert_eq!(handle.get(), ThemeKind::Light);
        assert_eq!(handle.toggle(), ThemeKind::Dark);
        assert_eq!(handle.get(), ThemeKind::Dark);
    }

    #[test]
    fn style_closure_follows_the_handle() {
        let handle = ThemeHandle::new(ThemeKind::Light);
        let style = handle.style_fn();
        let light = style().bg;
        handle.toggle();
        assert_ne!(style().bg, light);
    }
}
