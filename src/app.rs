//! Application entry: window setup, native menu, root view, and the board's
//! autosave / clipboard / file hooks.
//!
//! The root view owns almost nothing: the board entity and the theme handle.
//! Everything about the board itself — selection, undo history, camera, elements
//! and the whole editing toolbar — lives inside gpui-whiteboard. The only things
//! crossing that boundary are the scene JSON and the host hooks.
//!
//! Persistence is split deliberately:
//!
//! - the board's `on_change` hook debounces a write of the scene to
//!   the platform's app-data folder, so ordinary editing needs no user action;
//! - **File ▸ Save a copy…** writes the same JSON wherever the user asks, via a
//!   native save dialog, and leaves the working board alone;
//! - **File ▸ Open board…** adopts another file as the working board.
//!
//! The working board is a cache that always reflects the last edit; a copy is a
//! document the user owns.

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    actions, prelude::*, px, App, Bounds, ClipboardItem, Context, Entity, Global, KeyBinding, Menu,
    MenuItem, PathPromptOptions, Render, Window, WindowBounds, WindowOptions,
};
use gpui_whiteboard::{Scene, WhiteboardView};

use crate::storage::{self, UiState};
use crate::theme::{self, ThemeKind};
use crate::ui::{self, Chrome, ThemeHandle};

/// How long an idle burst of edits is allowed to sit unsaved.
///
/// Long enough that a drag or a typing run collapses into a single write, short
/// enough that a clean quit right after an edit rarely loses anything.
const AUTOSAVE_DEBOUNCE_MS: u64 = 600;

actions!(
    nottio,
    [
        ToggleTheme,
        SaveCopy,
        OpenBoard,
        Undo,
        Redo,
        ZoomIn,
        ZoomOut,
        ResetView,
        Quit
    ]
);

/// State the menu actions and board hooks need but cannot reach through their
/// own arguments.
///
/// `App::on_action` hands its listener only `&mut App`, and gpui-whiteboard's
/// hooks run while the board entity is mutably borrowed. A global is the only
/// place both can read. The root view is reached through `window` rather than a
/// stored handle, because the window is created before the root view exists.
pub(crate) struct NearApp {
    board_path: PathBuf,
    ui_state_path: PathBuf,
    ui: UiState,
    status: Status,
    theme: ThemeHandle,
    window: Option<gpui::AnyWindowHandle>,
    // Snapshot without re-entering a window already handling a native action.
    board: Option<gpui::WeakEntity<WhiteboardView>>,
}

impl Global for NearApp {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    Empty,
    Saved(String),
    Wrote(PathBuf),
    Failed(String),
}

impl Status {
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Empty => String::new(),
            Self::Saved(clock) => format!("Saved {clock}"),
            Self::Wrote(path) => format!("Wrote {}", path.display()),
            Self::Failed(message) => format!("Error: {message}"),
        }
    }

    pub(crate) fn is_error(&self) -> bool {
        matches!(self, Self::Failed(_))
    }
}

pub(crate) struct RootView {
    board: Entity<WhiteboardView>,
    theme: ThemeHandle,
}

impl RootView {
    /// Swap in a freshly built board after **File ▸ Open board…**.
    fn adopt(&mut self, board: Entity<WhiteboardView>) {
        self.board = board;
    }
}

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let kind = self.theme.get();
        let chrome = Chrome {
            palette: theme::palette(kind),
            status: cx.global::<NearApp>().status.clone(),
        };
        ui::render(&chrome, self.board.clone())
    }
}

// ---------------------------------------------------------------------------
// Reaching the live view and board from a global-only context
// ---------------------------------------------------------------------------

/// Run `f` against the live root view.
///
/// The root view is looked up through the window rather than stored as a handle,
/// because the window is created before the root view exists — there is no
/// point at which a `WeakEntity<RootView>` could be captured without unsafe
/// self-reference.
fn with_root(cx: &mut App, f: impl FnOnce(&mut RootView, &mut Context<RootView>)) {
    let Some(window) = cx.global::<NearApp>().window else {
        return;
    };
    let _ = window.update(cx, |view, _window, cx| {
        if let Ok(root) = view.downcast::<RootView>() {
            root.update(cx, |root, cx| f(root, cx));
        }
    });
}

/// Notify the live root view so the status strip repaints.
///
/// Called by `ui::repaint` from hooks that only have an `App`.
pub(crate) fn notify_root(cx: &mut App) {
    with_root(cx, |_root, cx| cx.notify());
}

fn board_of(cx: &App) -> Option<Entity<WhiteboardView>> {
    cx.global::<NearApp>().board.as_ref()?.upgrade()
}

/// Run a board method that needs `&mut Window` (undo, redo, paste).
fn with_board_window(
    cx: &mut App,
    f: impl FnOnce(&mut WhiteboardView, &mut Window, &mut Context<WhiteboardView>),
) {
    let Some(window) = cx.global::<NearApp>().window else {
        return;
    };
    let _ = window.update(cx, |view, window, cx| {
        let Ok(root) = view.downcast::<RootView>() else {
            return;
        };
        let board = root.read(cx).board.clone();
        board.update(cx, |board, cx| f(board, window, cx));
    });
}

// ---------------------------------------------------------------------------
// Board construction
// ---------------------------------------------------------------------------

/// Build a board entity with every host hook installed.
///
/// Hooks are installed here rather than in `RootView::new` because the setters
/// take `&mut Context<WhiteboardView>`, which only exists inside `cx.new`.
fn build_board(
    scene: Scene,
    theme: ThemeHandle,
    ui: &UiState,
    cx: &mut Context<WhiteboardView>,
) -> WhiteboardView {
    let (board_path, ui_state_path) = {
        let host = cx.global::<NearApp>();
        (host.board_path.clone(), host.ui_state_path.clone())
    };

    let mut view = WhiteboardView::new(scene, theme.style_fn(), cx);

    // --- restore persisted layout and swatches ---------------------------
    view.set_toolbar_pos(ui.toolbar_pos, cx);
    view.set_toolbar_vertical(ui.toolbar_vertical, cx);
    view.set_saved_colors(ui.saved_colors.clone(), cx);

    // --- autosave --------------------------------------------------------
    // One timer per burst of edits: every change bumps a generation counter,
    // and a timer that wakes up stale exits without writing, so the last edit
    // of a burst is the one that lands.
    let generation = Rc::new(std::cell::Cell::new(0u64));
    view.set_on_change(Rc::new(move |scene_json, _window, cx| {
        let expected = generation.get().wrapping_add(1);
        generation.set(expected);
        let generation = generation.clone();
        let path = board_path.clone();
        cx.spawn(async move |cx| {
            cx.background_executor()
                .timer(Duration::from_millis(AUTOSAVE_DEBOUNCE_MS))
                .await;
            if generation.get() != expected {
                return;
            }
            let json = scene_json.clone();
            let result = cx
                .background_executor()
                .spawn(async move { storage::write_board(&path, &json) })
                .await;
            cx.update(|cx| {
                cx.global_mut::<NearApp>().status = match result {
                    Ok(()) => Status::Saved(storage::clock_now()),
                    Err(err) => Status::Failed(err.to_string()),
                };
                ui::repaint(cx);
            });
        })
        .detach();
    }));

    // --- clipboard -------------------------------------------------------
    // The crate never touches the system clipboard itself: it hands the host
    // the serialized selection and asks the host to read it back, so a copy can
    // be pasted onto a different board, or into another app's board.
    view.set_on_copy(Rc::new(|json, _window, cx| {
        cx.write_to_clipboard(ClipboardItem::new_string(json));
    }));
    view.set_on_paste(Rc::new(|_window, cx| {
        cx.read_from_clipboard()
            .and_then(|item| item.text())
            .filter(|text| ui::looks_like_elements(text))
    }));
    view.set_on_paste_image(Rc::new(|_window, cx| {
        let item = cx.read_from_clipboard()?;
        match crate::images::from_clipboard(&item) {
            Ok(image) => image,
            Err(error) => {
                cx.global_mut::<NearApp>().status = Status::Failed(error);
                ui::repaint(cx);
                None
            }
        }
    }));
    let image_cache = Rc::new(std::cell::RefCell::new(crate::images::ImageCache::default()));
    view.set_on_image(Rc::new(move |src, rotation, _window, _cx| {
        image_cache.borrow_mut().source(src, rotation)
    }));

    // --- toolbar layout and saved swatches -------------------------------
    let layout_path = ui_state_path.clone();
    view.set_on_move_toolbar(Rc::new(move |pos, vertical, _window, cx| {
        let snapshot = {
            let ui = &mut cx.global_mut::<NearApp>().ui;
            ui.toolbar_pos = pos;
            ui.toolbar_vertical = vertical;
            ui.clone()
        };
        snapshot.save(&layout_path);
    }));

    let colors_path = ui_state_path;
    view.set_on_save_colors(Rc::new(move |colors, _window, cx| {
        let snapshot = {
            let ui = &mut cx.global_mut::<NearApp>().ui;
            ui.saved_colors = colors;
            ui.clone()
        };
        snapshot.save(&colors_path);
    }));

    view
}

// ---------------------------------------------------------------------------
// Menu actions
// ---------------------------------------------------------------------------

fn set_menus(cx: &mut App) {
    let mut menus = Vec::new();
    if cfg!(target_os = "macos") {
        menus.push(Menu::new("nottio").items([MenuItem::action("Quit nottio", Quit)]));
    }
    menus.extend([
        Menu::new("File").items([
            MenuItem::action("Open board…", OpenBoard),
            MenuItem::action("Save a copy…", SaveCopy),
        ]),
        Menu::new("Edit").items([
            MenuItem::action("Undo", Undo),
            MenuItem::action("Redo", Redo),
        ]),
        Menu::new("View").items([
            MenuItem::action("Zoom in", ZoomIn),
            MenuItem::action("Zoom out", ZoomOut),
            MenuItem::action("Reset view", ResetView),
            MenuItem::separator(),
            MenuItem::action("Toggle theme", ToggleTheme),
        ]),
    ]);
    cx.set_menus(menus);
    let modifier = if cfg!(target_os = "macos") {
        "cmd"
    } else {
        "ctrl"
    };
    cx.bind_keys([
        KeyBinding::new(&format!("{modifier}-o"), OpenBoard, None),
        KeyBinding::new(&format!("{modifier}-shift-s"), SaveCopy, None),
        KeyBinding::new(&format!("{modifier}-z"), Undo, None),
        KeyBinding::new(&format!("{modifier}-shift-z"), Redo, None),
    ]);
    if cfg!(target_os = "macos") {
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
    }
}

fn flush_working_board(cx: &App) -> std::io::Result<()> {
    if let Some(board) = board_of(cx) {
        storage::write_board(
            &cx.global::<NearApp>().board_path,
            &board.read(cx).scene().to_json(),
        )?;
    }
    Ok(())
}

fn quit(_: &Quit, cx: &mut App) {
    // Cmd+Q does not necessarily close each NSWindow first. Never rely on the
    // window-close hook or debounce to persist the last edit on app-level quit.
    match flush_working_board(cx) {
        Ok(()) => cx.quit(),
        Err(error) => {
            cx.global_mut::<NearApp>().status = Status::Failed(error.to_string());
            notify_root(cx);
        }
    }
}

fn toggle_theme(_: &ToggleTheme, cx: &mut App) {
    let kind = cx.global::<NearApp>().theme.toggle();
    let path = cx.global::<NearApp>().ui_state_path.clone();
    let snapshot = {
        let ui = &mut cx.global_mut::<NearApp>().ui;
        ui.dark = kind.is_dark();
        ui.clone()
    };
    snapshot.save(&path);
    ui::repaint(cx);
}

/// Write the current scene wherever the user asks.
fn save_copy(_: &SaveCopy, cx: &mut App) {
    let Some(board) = board_of(cx) else {
        return;
    };
    let json = board.read(cx).scene().to_json();
    let receiver = cx.prompt_for_new_path(&storage::data_dir(), Some("board.json"));
    cx.spawn(async move |cx| {
        let Ok(Ok(Some(path))) = receiver.await else {
            return;
        };
        let json = json.clone();
        let target = path.clone();
        let result = cx
            .background_executor()
            .spawn(async move { storage::write_atomically(&target, &json) })
            .await;
        cx.update(|cx| {
            cx.global_mut::<NearApp>().status = match result {
                Ok(()) => Status::Wrote(path.clone()),
                Err(err) => Status::Failed(err.to_string()),
            };
            ui::repaint(cx);
        });
    })
    .detach();
}

/// Adopt another board file as the working board.
///
/// The board entity is rebuilt rather than mutated: the crate has no
/// `set_scene`, and rebuilding is also the right thing to do to the undo
/// history when a different document is loaded.
fn open_board(_: &OpenBoard, cx: &mut App) {
    let receiver = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some("Open a nottio board".into()),
    });
    cx.spawn(async move |cx| {
        let Ok(Ok(Some(paths))) = receiver.await else {
            return;
        };
        let Some(path) = paths.into_iter().next() else {
            return;
        };

        let loaded = std::fs::read_to_string(&path);
        let (json, error) = match loaded {
            Ok(json) if ui::looks_like_board(&json) => (json, None),
            Ok(_) => (
                String::new(),
                Some(format!("{} is not a board file", path.display())),
            ),
            Err(err) => (
                String::new(),
                Some(format!("Could not read {}: {err}", path.display())),
            ),
        };
        if let Some(message) = error {
            cx.update(|cx| {
                cx.global_mut::<NearApp>().status = Status::Failed(message);
                ui::repaint(cx);
            });
            return;
        }

        let scene = Scene::from_json(&json);
        cx.update(|cx| {
            // Also make it the working board, so autosave and the next launch
            // continue from the file the user just opened.
            let board_path = cx.global::<NearApp>().board_path.clone();
            let _ = storage::write_board(&board_path, &json);

            let theme = cx.global::<NearApp>().theme.clone();
            let ui = cx.global::<NearApp>().ui.clone();
            let board = cx.new(|cx| build_board(scene, theme, &ui, cx));
            cx.global_mut::<NearApp>().board = Some(board.downgrade());
            cx.global_mut::<NearApp>().status = Status::Wrote(path.clone());

            with_root(cx, |root, cx| {
                root.adopt(board.clone());
                cx.notify();
            });
        });
        cx.update(ui::repaint);
    })
    .detach();
}

fn undo(_: &Undo, cx: &mut App) {
    with_board_window(cx, |board, window, cx| board.undo(window, cx));
}

fn redo(_: &Redo, cx: &mut App) {
    with_board_window(cx, |board, window, cx| board.redo(window, cx));
}

fn zoom_in(_: &ZoomIn, cx: &mut App) {
    if let Some(board) = board_of(cx) {
        board.update(cx, |board, cx| board.zoom_in(cx));
    }
}

fn zoom_out(_: &ZoomOut, cx: &mut App) {
    if let Some(board) = board_of(cx) {
        board.update(cx, |board, cx| board.zoom_out(cx));
    }
}

fn reset_view(_: &ResetView, cx: &mut App) {
    if let Some(board) = board_of(cx) {
        board.update(cx, |board, cx| board.reset_view(cx));
    }
}

// ---------------------------------------------------------------------------

pub(crate) fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let ui_state_path = storage::ui_state_path();
        let ui = UiState::load(&ui_state_path);
        let board_path = storage::board_path();
        let scene = storage::read_board(&board_path)
            .map(|json| Scene::from_json(&json))
            .unwrap_or_default();
        let theme = ThemeHandle::new(ThemeKind::from_dark(ui.dark));

        cx.on_action(toggle_theme);
        cx.on_action(save_copy);
        cx.on_action(open_board);
        cx.on_action(undo);
        cx.on_action(redo);
        cx.on_action(zoom_in);
        cx.on_action(zoom_out);
        cx.on_action(reset_view);
        cx.on_action(quit);
        cx.on_app_quit(|cx| {
            // Also covers native Dock/system quit. This late hook cannot veto
            // shutdown, whereas our Quit action above can report a write error.
            if let Err(error) = flush_working_board(cx) {
                eprintln!("Could not save nottio board on quit: {error}");
            }
            async {}
        })
        .detach();
        set_menus(cx);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        // The global has to exist before the board is built, because the hooks
        // read the paths and UI state out of it; `window` is filled in as soon
        // as the window exists.
        cx.set_global(NearApp {
            board_path,
            ui_state_path,
            ui: ui.clone(),
            status: Status::Empty,
            theme: theme.clone(),
            window: None,
            board: None,
        });

        let bounds = Bounds::centered(None, gpui::size(px(1280.), px(840.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                window.set_window_title("nottio");
                let handle = window.window_handle();
                cx.global_mut::<NearApp>().window = Some(handle);
                let root = cx.new(|cx| {
                    let board = cx.new(|cx| build_board(scene, theme.clone(), &ui, cx));
                    cx.global_mut::<NearApp>().board = Some(board.downgrade());
                    RootView {
                        board,
                        theme: theme.clone(),
                    }
                });
                let weak = root.downgrade();
                // Closing immediately after an edit must not race the debounce.
                window.on_window_should_close(cx, move |_window, cx| {
                    let Some(root) = weak.upgrade() else {
                        return true;
                    };
                    let json = root.read(cx).board.read(cx).scene().to_json();
                    let path = cx.global::<NearApp>().board_path.clone();
                    match storage::write_board(&path, &json) {
                        Ok(()) => true,
                        Err(error) => {
                            cx.global_mut::<NearApp>().status = Status::Failed(error.to_string());
                            root.update(cx, |_, cx| cx.notify());
                            false
                        }
                    }
                });
                root
            },
        )
        .expect("could not open the nottio window");
        cx.activate(true);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, Image, ImageFormat, MouseButton, TestAppContext, VisualTestContext};
    use gpui_whiteboard::ElementKind;
    use std::io::Cursor;

    #[gpui::test]
    fn ctrl_v_pastes_a_real_clipboard_image_moves_copies_undoes_and_reopens(
        cx: &mut TestAppContext,
    ) {
        clipboard_round_trip(cx, "ctrl");
    }

    #[gpui::test]
    fn command_v_pastes_images_and_command_shortcuts_preserve_the_board(cx: &mut TestAppContext) {
        clipboard_round_trip(cx, "cmd");
    }

    fn clipboard_round_trip(cx: &mut TestAppContext, modifier: &str) {
        let dir =
            std::env::temp_dir().join(format!("nottio-image-ui-{modifier}-{}", std::process::id()));
        let path = dir.join("board.json");
        let theme = ThemeHandle::new(ThemeKind::Light);
        let window = cx.update(|cx| {
            cx.set_global(NearApp {
                board_path: path.clone(),
                ui_state_path: dir.join("ui-state.json"),
                ui: UiState::default(),
                status: Status::Empty,
                theme: theme.clone(),
                window: None,
                board: None,
            });
            let window = cx
                .open_window(Default::default(), |_, cx| {
                    let board = cx.new(|cx| {
                        build_board(Scene::default(), theme.clone(), &UiState::default(), cx)
                    });
                    cx.global_mut::<NearApp>().board = Some(board.downgrade());
                    cx.new(|_| RootView {
                        board,
                        theme: theme.clone(),
                    })
                })
                .unwrap();
            cx.global_mut::<NearApp>().window = Some(window.into());
            window
        });
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        let root = window.root(&mut cx).unwrap();
        let board = root.read_with(&cx, |root, _| root.board.clone());
        cx.run_until_parked();
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(120, 60)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        cx.update(|_, cx| {
            cx.write_to_clipboard(ClipboardItem::new_image(&Image::from_bytes(
                ImageFormat::Png,
                bytes.into_inner(),
            )))
        });
        let focus = point(px(400.0), px(350.0));
        cx.simulate_mouse_down(focus, MouseButton::Left, Default::default());
        cx.simulate_mouse_up(focus, MouseButton::Left, Default::default());
        cx.simulate_keystrokes(&format!("{modifier}-v"));
        cx.run_until_parked();
        let original = board.read_with(&cx, |board, _| {
            assert_eq!(board.scene().elements.len(), 1);
            let ElementKind::Image(image) = &board.scene().elements[0].kind else {
                panic!()
            };
            assert!(image.src.starts_with("data:image/png;base64,"));
            assert_eq!((image.w, image.h), (280.0, 140.0));
            image.clone()
        });
        let center = board.read_with(&cx, |board, _| board.viewport_center());
        // The host has a 30px status strip; the board's origin is just below it.
        let from = point(px(center[0]), px(center[1] + 30.0));
        let to = point(from.x + px(70.0), from.y + px(40.0));
        cx.simulate_mouse_down(from, MouseButton::Left, Default::default());
        cx.simulate_mouse_move(to, Some(MouseButton::Left), Default::default());
        cx.simulate_mouse_up(to, MouseButton::Left, Default::default());
        board.read_with(&cx, |board, _| {
            let ElementKind::Image(image) = &board.scene().elements[0].kind else {
                panic!()
            };
            assert!((image.x - original.x - 70.0).abs() < 0.01);
            assert!((image.y - original.y - 40.0).abs() < 0.01);
        });
        cx.simulate_keystrokes(&format!("{modifier}-z"));
        board.read_with(&cx, |board, _| {
            let ElementKind::Image(image) = &board.scene().elements[0].kind else {
                panic!()
            };
            assert_eq!((image.x, image.y), (original.x, original.y));
        });
        // Undo intentionally clears selection. Select before grabbing a handle.
        cx.simulate_mouse_down(from, MouseButton::Left, Default::default());
        cx.simulate_mouse_up(from, MouseButton::Left, Default::default());
        let edge = point(
            px(original.x + original.w),
            px(original.y + original.h / 2.0 + 30.0),
        );
        let resized = point(edge.x + px(80.0), edge.y);
        cx.simulate_mouse_down(edge, MouseButton::Left, Default::default());
        cx.simulate_mouse_move(resized, Some(MouseButton::Left), Default::default());
        cx.simulate_mouse_up(resized, MouseButton::Left, Default::default());
        board.read_with(&cx, |board, _| {
            let ElementKind::Image(image) = &board.scene().elements[0].kind else {
                panic!()
            };
            assert_eq!((image.w, image.h), (360.0, 140.0));
            assert_eq!(image.src, original.src);
        });
        cx.simulate_keystrokes(&format!("{modifier}-z"));
        cx.simulate_keystrokes(&format!("{modifier}-z"));
        board.read_with(&cx, |board, _| assert!(board.scene().elements.is_empty()));
        cx.simulate_keystrokes(&format!("{modifier}-shift-z"));
        cx.simulate_mouse_down(from, MouseButton::Left, Default::default());
        cx.simulate_mouse_up(from, MouseButton::Left, Default::default());
        cx.simulate_keystrokes(&format!("{modifier}-c"));
        // Include an invalid bitmap beside the valid copied selection. JSON
        // wins, so the image fallback must not emit an error or add another item.
        cx.update(|_, cx| {
            let mut item = cx.read_from_clipboard().unwrap();
            assert!(ui::looks_like_elements(&item.text().unwrap()));
            item.entries
                .push(gpui::ClipboardEntry::Image(Image::empty()));
            cx.write_to_clipboard(item);
        });
        cx.simulate_keystrokes(&format!("{modifier}-v"));
        let saved = board.read_with(&cx, |board, _| {
            assert_eq!(board.scene().elements.len(), 2);
            for element in &board.scene().elements {
                let ElementKind::Image(image) = &element.kind else {
                    panic!()
                };
                assert_eq!(image.src, original.src);
            }
            board.scene().to_json()
        });
        cx.update(|_, cx| assert!(!cx.global::<NearApp>().status.is_error()));
        storage::write_board(&path, &saved).unwrap();
        // Flush before advancing the autosave timer: app-level quit must keep
        // the very last clipboard edit, not only a previously debounced board.
        std::fs::remove_file(&path).unwrap();
        cx.update(|_, cx| flush_working_board(cx).unwrap());
        assert_eq!(storage::read_board(&path).unwrap(), saved);
        let reopened = Scene::from_json(&storage::read_board(&path).unwrap());
        assert_eq!(reopened.elements.len(), 2);
        for element in reopened.elements {
            let ElementKind::Image(image) = element.kind else {
                panic!()
            };
            assert!(crate::images::ImageCache::default()
                .source(&image.src, image.rotation)
                .is_some());
        }
        cx.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_image(&Image::empty())));
        cx.simulate_keystrokes(&format!("{modifier}-v"));
        board.read_with(&cx, |board, _| assert_eq!(board.scene().elements.len(), 2));
        cx.update(|_, cx| assert!(cx.global::<NearApp>().status.is_error()));
        let _ = std::fs::remove_dir_all(dir);
    }
}
