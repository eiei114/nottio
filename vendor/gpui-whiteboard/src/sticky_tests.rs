//! Regression tests through GPUI's actual rendered element/input dispatch tree.
use super::*;
use gpui::{TestAppContext, VisualTestContext};

#[test]
fn old_connectors_keep_default_heads_and_elbows_transform_with_the_line() {
    let legacy = r#"{"x1":10,"y1":20,"x2":30,"y2":40,"width":2.5}"#;
    let mut segment: SegGeom = serde_json::from_str(legacy).unwrap();
    assert_eq!(segment.bend, None);
    assert_eq!(
        segment.arrow_heads(true),
        ArrowHeads {
            start: false,
            end: true
        }
    );
    assert_eq!(
        segment.arrow_heads(false),
        ArrowHeads {
            start: false,
            end: false
        }
    );
    segment.bend = Some([-10.0, 80.0]);
    let mut kind = ElementKind::Arrow(segment);
    assert_eq!(bbox(&kind), (-10.0, 20.0, 34.0, 80.0));
    translate(&mut kind, 5.0, -3.0);
    assert_eq!(bbox(&kind), (-5.0, 17.0, 39.0, 77.0));
    resize_about(&mut kind, 0.0, 0.0, 2.0, 3.0);
    rotate_element(&mut kind, 0.0, 0.0, std::f32::consts::FRAC_PI_2);
    let ElementKind::Arrow(segment) = kind else {
        unreachable!()
    };
    let bend = segment.bend.unwrap();
    assert!((bend[0] + 231.0).abs() < 0.001);
    assert!((bend[1] + 10.0).abs() < 0.001);
}

fn fixture(cx: &mut TestAppContext) -> (Entity<WhiteboardView>, VisualTestContext) {
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|cx| {
                WhiteboardView::new(
                    Scene::default(),
                    Rc::new(|| WhiteboardStyle {
                        bg: rgba(0xF7F7F5FF).into(),
                        grid: rgba(0xCFCFCBFF).into(),
                        text: rgba(0x6B6B66FF).into(),
                        ink: rgba(0x1F1F23FF).into(),
                        panel: rgba(0xFFFFFFFF).into(),
                        panel_strong: rgba(0xFFFFFFFF).into(),
                        accent: rgba(0xE8EEFDFF).into(),
                        selection: rgba(0x2E6BE6FF).into(),
                        swatches: vec![],
                    }),
                    cx,
                )
            })
        })
        .unwrap()
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let board = window.root(&mut cx).unwrap();
    cx.run_until_parked();
    (board, cx)
}

#[test]
fn connector_types_have_distinct_paths_and_a_direct_curve_handle() {
    let mut seg = SegGeom::between([100.0, 100.0], [400.0, 300.0], 2.5);
    seg.connector_type = ConnectorType::Straight;
    assert_eq!(seg.points(), vec![[100.0, 100.0], [400.0, 300.0]]);
    seg.connector_type = ConnectorType::Bent;
    let bent = seg.points();
    assert_eq!(bent.first(), Some(&[100.0, 100.0]));
    assert_eq!(bent.last(), Some(&[400.0, 300.0]));
    assert!(bent.len() > 4, "bent corners should be rounded");
    seg.connector_type = ConnectorType::Curved;
    let curved = seg.points();
    assert_eq!(curved.len(), 33);
    assert_ne!(curved, bent);
    assert_eq!(seg.bend_handle(), curved[16]);
    seg.bend = Some([270.0, 50.0]);
    assert_eq!(seg.points()[16], [270.0, 50.0]);
    assert!(hit_test(&ElementKind::Arrow(seg), 270.0, 50.0, 1.0));
    let restored: SegGeom = serde_json::from_str(&serde_json::to_string(&seg).unwrap()).unwrap();
    assert_eq!(restored.connector_type, ConnectorType::Curved);
    assert_eq!(restored.points(), seg.points());
}

#[test]
fn multiple_bends_round_trip_transform_and_enforce_capacity() {
    let mut seg = SegGeom::between([100.0, 200.0], [600.0, 200.0], 2.5);
    let bends = [[180.0, 150.0], [350.0, 80.0], [490.0, 250.0]];
    assert!(seg.set_manual_bends(&bends));
    assert_eq!(seg.bend_handles(), bends);
    let json = serde_json::to_string(&seg).unwrap();
    assert!(json.contains("\"extra_bends\":[[350.0,80.0],[490.0,250.0]]"));
    let mut back: SegGeom = serde_json::from_str(&json).unwrap();
    assert_eq!(back.manual_bends(), bends);
    assert!(!back.set_manual_bends(&[[0.0, 0.0]; 34]));
    assert_eq!(back.manual_bends(), bends);
    let mut kind = ElementKind::Arrow(back);
    translate(&mut kind, 10.0, 10.0);
    resize_about(&mut kind, 0.0, 0.0, 2.0, 3.0);
    rotate_element(&mut kind, 0.0, 0.0, std::f32::consts::FRAC_PI_2);
    let ElementKind::Arrow(back) = kind else {
        unreachable!()
    };
    for (point, original) in back.manual_bends().iter().zip(bends) {
        assert!((point[0] + (original[1] + 10.0) * 3.0).abs() < 0.001);
        assert!((point[1] - (original[0] + 10.0) * 2.0).abs() < 0.001);
    }
    assert!(back.points().len() > 10);
}

#[gpui::test]
fn multiple_folds_add_drag_remove_cancel_and_save_through_ui(cx: &mut TestAppContext) {
    let (board, mut cx) = fixture(cx);
    board.update_in(&mut cx, |board, window, cx| {
        let left = Element::sticky(1, [200.0, 280.0]);
        let right = Element::sticky(2, [660.0, 280.0]);
        let mut arrow = Element::sticky(3, [0.0, 0.0]);
        arrow.kind = ElementKind::Arrow(
            serde_json::from_str(
                r#"{
            "x1":290,"y1":280,"x2":570,"y2":280,"width":2.5,
            "start_anchor":{"element_id":1,"connector":1},
            "end_anchor":{"element_id":2,"connector":3},
            "heads":{"start":true,"end":true}
        }"#,
            )
            .unwrap(),
        );
        board.scene.elements = vec![left, right, arrow];
        board.selected = vec![3];
        board.tool = Tool::Select;
        board.focus.focus(window, cx);
        cx.notify();
    });
    for _ in 0..2 {
        let add = cx.debug_bounds("arrow-add-bend").unwrap().center();
        cx.simulate_click(add, Default::default());
    }
    let before = board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert_eq!(seg.manual_bends().len(), 3);
        assert!(!seg.auto_route);
        seg.manual_bends()
    });
    let handle = cx.debug_bounds("arrow-bend-handle-1").unwrap().center();
    let moved = point(px(440.0), px(160.0));
    cx.simulate_mouse_down(handle, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(moved, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(moved, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert_eq!(seg.manual_bends()[0], before[0]);
        assert_eq!(seg.manual_bends()[1], [440.0, 160.0]);
        assert_eq!(seg.manual_bends()[2], before[2]);
        assert_eq!(seg.start_anchor.unwrap().element_id, 1);
        assert_eq!(seg.end_anchor.unwrap().element_id, 2);
        assert_eq!(
            seg.arrow_heads(true),
            ArrowHeads {
                start: true,
                end: true
            }
        );
        let restored = Scene::from_json(&board.scene.to_json());
        let ElementKind::Arrow(back) = restored.elements[2].kind else {
            unreachable!()
        };
        assert_eq!(back.manual_bends(), seg.manual_bends());
    });
    // Escape restores every handle, not just the original legacy bend.
    let handle = cx.debug_bounds("arrow-bend-handle-2").unwrap().center();
    cx.simulate_mouse_down(handle, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(
        point(px(530.0), px(200.0)),
        Some(MouseButton::Left),
        Default::default(),
    );
    cx.simulate_keystrokes("escape");
    cx.simulate_mouse_up(
        point(px(530.0), px(200.0)),
        MouseButton::Left,
        Default::default(),
    );
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert_eq!(seg.manual_bends()[2], before[2]);
    });
    // Double-click removes the chosen middle fold, not all folds.
    let handle = cx.debug_bounds("arrow-bend-handle-1").unwrap().center();
    cx.simulate_event(MouseDownEvent {
        position: handle,
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 2,
        first_mouse: false,
    });
    cx.simulate_mouse_up(handle, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert_eq!(seg.manual_bends(), vec![before[0], before[2]]);
    });
    cx.simulate_keystrokes("ctrl-z");
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert_eq!(seg.manual_bends().len(), 3);
    });
    cx.simulate_keystrokes("ctrl-shift-z");
    cx.simulate_click(
        point(px(before[0][0]), px(before[0][1])),
        Default::default(),
    );
    let remove = cx.debug_bounds("arrow-remove-bend").unwrap().center();
    cx.simulate_click(remove, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert_eq!(seg.manual_bends(), vec![before[0]]);
    });
    let route = cx.debug_bounds("arrow-auto-route").unwrap().center();
    cx.simulate_click(route, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert!(seg.manual_bends().is_empty());
        assert!(seg.auto_route);
        assert_eq!(seg.start_anchor.unwrap().element_id, 1);
        assert_eq!(seg.end_anchor.unwrap().element_id, 2);
    });
}

#[gpui::test]
fn connector_tool_links_bodies_changes_type_and_reconnects_endpoints(cx: &mut TestAppContext) {
    let (board, mut cx) = fixture(cx);
    board.update_in(&mut cx, |board, window, cx| {
        let left = Element::sticky(1, [200.0, 280.0]);
        let right = Element::sticky(2, [660.0, 280.0]);
        let lower = Element::sticky(3, [660.0, 490.0]);
        board.scene.elements = vec![left, right, lower];
        board.next_id = 4;
        board.focus.focus(window, cx);
        cx.notify();
    });
    cx.simulate_keystrokes("a");
    let start = point(px(200.0), px(280.0));
    let end = point(px(660.0), px(280.0));
    cx.simulate_mouse_down(start, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(end, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(end, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        assert_eq!(board.scene.elements.len(), 4);
        let ElementKind::Arrow(seg) = board.scene.elements[3].kind else {
            panic!("connector tool must create an arrow, not move the note")
        };
        assert_eq!(seg.start_anchor.unwrap().element_id, 1);
        assert_eq!(seg.start_anchor.unwrap().connector, 1);
        assert_eq!(seg.end_anchor.unwrap().element_id, 2);
        assert_eq!(seg.end_anchor.unwrap().connector, 3);
        assert_eq!(seg.connector_type, ConnectorType::Bent);
    });
    let curve = cx.debug_bounds("arrow-curved").unwrap().center();
    cx.simulate_click(curve, Default::default());
    let handle = cx.debug_bounds("arrow-bend-handle").unwrap().center();
    let bent = point(px(430.0), px(180.0));
    cx.simulate_mouse_down(handle, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(bent, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(bent, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[3].kind else {
            unreachable!()
        };
        assert_eq!(seg.connector_type, ConnectorType::Curved);
        assert!(!seg.auto_route);
        assert_eq!(seg.bend, Some([430.0, 180.0]));
        assert_eq!(seg.points()[16], [430.0, 180.0]);
    });
    // Editing the selected endpoint must not start another arrow even though
    // the connector tool remains active after creation.
    let port = point(px(570.0), px(280.0));
    let lower = point(px(660.0), px(490.0));
    cx.simulate_mouse_down(port, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(lower, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(lower, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        assert_eq!(board.scene.elements.len(), 4);
        let ElementKind::Arrow(seg) = board.scene.elements[3].kind else {
            unreachable!()
        };
        assert_eq!(seg.end_anchor.unwrap().element_id, 3);
        assert_eq!(seg.start_anchor.unwrap().element_id, 1);
    });
    cx.simulate_keystrokes("ctrl-z");
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[3].kind else {
            unreachable!()
        };
        assert_eq!(seg.end_anchor.unwrap().element_id, 2);
        assert_eq!(seg.connector_type, ConnectorType::Curved);
    });
    cx.simulate_keystrokes("ctrl-shift-z");
    cx.simulate_click(bent, Default::default());
    let straight = cx.debug_bounds("arrow-straighten").unwrap().center();
    cx.simulate_click(straight, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[3].kind else {
            unreachable!()
        };
        assert_eq!(seg.connector_type, ConnectorType::Straight);
        assert_eq!(seg.points().len(), 2);
        assert!(seg.bend.is_none());
    });
    let elbow = cx.debug_bounds("arrow-bent").unwrap().center();
    cx.simulate_click(elbow, Default::default());
    board.read_with(&cx, |board, _| {
        let restored = Scene::from_json(&board.scene.to_json());
        let ElementKind::Arrow(seg) = restored.elements[3].kind else {
            unreachable!()
        };
        assert_eq!(seg.connector_type, ConnectorType::Bent);
        assert!(seg.auto_route);
        assert_eq!(seg.end_anchor.unwrap().element_id, 3);
    });
}

#[gpui::test]
fn toolbar_drag_drops_one_sticky_with_text_and_undo(cx: &mut TestAppContext) {
    let (board, mut cx) = fixture(cx);
    let button = cx
        .debug_bounds("sticky-tool")
        .expect("sticky toolbar button");
    let destination = point(px(300.0), px(280.0));
    cx.simulate_mouse_down(button.center(), MouseButton::Left, Default::default());
    cx.simulate_mouse_move(destination, Some(MouseButton::Left), Default::default());
    board.read_with(&cx, |board, _| {
        assert!(
            board.scene.elements.is_empty(),
            "preview must not autosave as an object"
        );
        assert!(board.sticky_preview.is_some());
    });
    cx.simulate_mouse_up(destination, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        assert_eq!(board.scene.elements.len(), 1);
        assert!(matches!(
            board.scene.elements[0].kind,
            ElementKind::Sticky(_)
        ));
        assert_eq!(board.editing, Some(board.scene.elements[0].id));
        assert_eq!(board.tool, Tool::Select);
        assert!(board.sticky_preview.is_none());
    });
    cx.simulate_input("思考を整理");
    cx.simulate_keystrokes("enter");
    cx.simulate_input("次のアイデア");
    cx.simulate_keystrokes("escape");
    board.read_with(&cx, |board, _| {
        assert_eq!(
            board.scene.elements[0].label.as_deref(),
            Some("思考を整理\n次のアイデア")
        );
        assert!(board.editing.is_none());
    });
    // Drag the note itself, not its text. Then exercise the real undo/redo path.
    let moved = point(px(430.0), px(390.0));
    cx.simulate_mouse_down(destination, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(moved, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(moved, MouseButton::Left, Default::default());
    let after = board.read_with(&cx, |board, _| bbox(&board.scene.elements[0].kind));
    cx.simulate_keystrokes("ctrl-z");
    let before = board.read_with(&cx, |board, _| bbox(&board.scene.elements[0].kind));
    assert_ne!(before, after);
    cx.simulate_keystrokes("ctrl-shift-z");
    board.read_with(&cx, |board, _| {
        assert_eq!(bbox(&board.scene.elements[0].kind), after);
        assert_eq!(
            board.scene.elements[0].label.as_deref(),
            Some("思考を整理\n次のアイデア")
        );
        let restored = Scene::from_json(&board.scene.to_json());
        assert!(matches!(restored.elements[0].kind, ElementKind::Sticky(_)));
        assert_eq!(restored.elements[0].label, board.scene.elements[0].label);
    });
}

#[gpui::test]
fn sticky_button_click_arms_without_placing_and_escape_cancels_drag(cx: &mut TestAppContext) {
    let (board, mut cx) = fixture(cx);
    let button = cx.debug_bounds("sticky-tool").unwrap().center();
    cx.simulate_click(button, Default::default());
    board.read_with(&cx, |board, _| {
        assert_eq!(board.tool, Tool::Sticky);
        assert!(board.scene.elements.is_empty());
    });
    cx.simulate_mouse_down(button, MouseButton::Left, Default::default());
    let dest = point(px(300.0), px(280.0));
    cx.simulate_mouse_move(dest, Some(MouseButton::Left), Default::default());
    cx.simulate_keystrokes("escape");
    cx.simulate_mouse_up(dest, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        assert!(board.scene.elements.is_empty());
        assert!(board.history.is_empty());
        assert!(board.sticky_preview.is_none());
    });
}

#[gpui::test]
fn connected_arrow_bends_heads_toggle_and_follows_a_dragged_note(cx: &mut TestAppContext) {
    let (board, mut cx) = fixture(cx);
    board.update_in(&mut cx, |board, window, cx| {
        let note = Element::sticky(1, [200.0, 280.0]);
        let mut shape = Element::sticky(2, [660.0, 280.0]);
        let ElementKind::Sticky(geom) = shape.kind else {
            unreachable!()
        };
        shape.kind = ElementKind::Rect(geom);
        board.scene.elements = vec![note, shape];
        board.next_id = 3;
        board.selected = vec![1];
        board.tool = Tool::Select;
        board.focus.focus(window, cx);
        cx.notify();
    });
    // Create a genuine sticky-to-shape attachment through the existing connector
    // drag path, not by seeding an already-linked arrow into the model.
    let connector = board.read_with(&cx, |board, _| {
        connector_button_centers(
            &board.scene.elements[0].kind,
            board.scene.camera,
            board.bounds.get().origin,
        )[1]
    });
    let endpoint = point(px(570.0), px(280.0));
    cx.simulate_mouse_down(connector, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(endpoint, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(endpoint, MouseButton::Left, Default::default());
    let arrow = board.read_with(&cx, |board, _| {
        assert_eq!(board.scene.elements.len(), 3);
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            panic!("expected attached arrow")
        };
        assert_eq!(seg.start_anchor.unwrap().element_id, 1);
        assert_eq!(seg.end_anchor.unwrap().element_id, 2);
        assert_eq!(
            seg.arrow_heads(true),
            ArrowHeads {
                start: false,
                end: true
            }
        );
        board.scene.elements[2].id
    });
    let handle = cx.debug_bounds("arrow-bend-handle").unwrap().center();
    let elbow = point(px(440.0), px(160.0));
    cx.simulate_mouse_down(handle, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(elbow, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(elbow, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert_eq!(seg.bend, Some([440.0, 160.0]));
        assert!(hit_test(&board.scene.elements[2].kind, 440.0, 160.0, 5.0));
        assert!(!hit_test(&board.scene.elements[2].kind, 440.0, 280.0, 5.0));
    });
    cx.simulate_keystrokes("ctrl-z");
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert_eq!(seg.bend, None);
    });
    cx.simulate_keystrokes("ctrl-shift-z");
    // Undo clears selection; reselect by clicking the actual bent line.
    cx.simulate_click(elbow, Default::default());
    let start = cx.debug_bounds("arrow-start-head").unwrap().center();
    cx.simulate_click(start, Default::default());
    let end = cx.debug_bounds("arrow-end-head").unwrap().center();
    cx.simulate_click(end, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert_eq!(
            seg.arrow_heads(true),
            ArrowHeads {
                start: true,
                end: false
            }
        );
    });
    // Both ends can be disabled, then enabled independently.
    cx.simulate_click(start, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert_eq!(
            seg.arrow_heads(true),
            ArrowHeads {
                start: false,
                end: false
            }
        );
    });
    cx.simulate_click(start, Default::default());
    cx.simulate_click(end, Default::default());
    let old_note = point(px(200.0), px(280.0));
    let moved_note = point(px(200.0), px(410.0));
    cx.simulate_mouse_down(old_note, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(moved_note, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(moved_note, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert_eq!(seg.x1, 290.0);
        assert_eq!(seg.y1, 410.0);
        assert_eq!(seg.bend, Some([440.0, 160.0]));
        assert_eq!(
            seg.arrow_heads(true),
            ArrowHeads {
                start: true,
                end: true
            }
        );
        let restored = Scene::from_json(&board.scene.to_json());
        let ElementKind::Arrow(back) = restored
            .elements
            .iter()
            .find(|e| e.id == arrow)
            .unwrap()
            .kind
        else {
            unreachable!()
        };
        assert_eq!(back.bend, seg.bend);
        assert_eq!(back.heads, seg.heads);
        assert_eq!(back.start_anchor, seg.start_anchor);
        assert_eq!(back.end_anchor, seg.end_anchor);
    });
    // The straighten action removes the bend without detaching either endpoint.
    cx.simulate_click(elbow, Default::default());
    let straighten = cx.debug_bounds("arrow-straighten").unwrap().center();
    cx.simulate_click(straighten, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert!(seg.bend.is_none());
        assert!(seg.start_anchor.is_some());
        assert!(seg.end_anchor.is_some());
    });
    let handle = cx.debug_bounds("arrow-bend-handle").unwrap().center();
    cx.simulate_mouse_down(handle, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(elbow, Some(MouseButton::Left), Default::default());
    cx.simulate_keystrokes("escape");
    cx.simulate_mouse_up(elbow, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert!(seg.bend.is_none(), "Escape must restore the previous route");
        assert!(seg.start_anchor.is_some());
        assert!(seg.end_anchor.is_some());
    });
}

#[gpui::test]
fn auto_route_avoids_notes_recomputes_on_move_and_survives_restore(cx: &mut TestAppContext) {
    let (board, mut cx) = fixture(cx);
    board.update_in(&mut cx, |board, window, cx| {
        let left = Element::sticky(1, [200.0, 280.0]);
        let right = Element::sticky(2, [660.0, 280.0]);
        let blocker = Element::sticky(3, [430.0, 280.0]);
        let mut arrow = Element::sticky(4, [0.0, 0.0]);
        arrow.kind = ElementKind::Arrow(
            serde_json::from_str(
                r#"{
            "x1":290,"y1":280,"x2":570,"y2":280,"width":2.5,
            "start_anchor":{"element_id":1,"connector":1},
            "end_anchor":{"element_id":2,"connector":3},
            "heads":{"start":true,"end":true}
        }"#,
            )
            .unwrap(),
        );
        board.scene.elements = vec![left, right, blocker, arrow];
        board.selected = vec![4];
        board.tool = Tool::Select;
        board.focus.focus(window, cx);
        cx.notify();
    });
    let detour = board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[3].kind else {
            unreachable!()
        };
        assert!(!seg.route_failed);
        assert!(seg.auto_route);
        assert!(seg.points().len() > 2);
        let blocker = bbox(&board.scene.elements[2].kind);
        for pair in seg.points().windows(2) {
            // Here all legs must avoid the unrelated blocking note.
            let steps = 100;
            for step in 0..=steps {
                let t = step as f32 / steps as f32;
                let p = [
                    pair[0][0] + t * (pair[1][0] - pair[0][0]),
                    pair[0][1] + t * (pair[1][1] - pair[0][1]),
                ];
                assert!(
                    !(p[0] > blocker.0 && p[0] < blocker.2 && p[1] > blocker.1 && p[1] < blocker.3)
                );
            }
        }
        let json = board.scene.to_json();
        assert!(
            !json.contains("\"route\":"),
            "derived route coordinates must not persist"
        );
        let restored = Scene::from_json(&json);
        let ElementKind::Arrow(restored_seg) = restored.elements[3].kind else {
            unreachable!()
        };
        assert!(restored_seg.auto_route);
        assert!(restored_seg.route.is_none());
        assert_eq!(restored_seg.heads, seg.heads);
        seg.points()
    });
    let center = point(px(430.0), px(280.0));
    let moved = point(px(430.0), px(500.0));
    cx.simulate_mouse_down(center, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(moved, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(moved, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[3].kind else {
            unreachable!()
        };
        assert_eq!(seg.points(), vec![[290.0, 280.0], [570.0, 280.0]]);
    });
    cx.simulate_keystrokes("ctrl-z");
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[3].kind else {
            unreachable!()
        };
        assert_eq!(seg.points(), detour);
    });
    cx.simulate_keystrokes("ctrl-shift-z");
    cx.simulate_click(center, Default::default());
    let auto = cx.debug_bounds("arrow-auto-route").unwrap().center();
    cx.simulate_click(auto, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[3].kind else {
            unreachable!()
        };
        assert!(!seg.auto_route);
        assert!(seg.route.is_none());
    });
    cx.simulate_click(auto, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[3].kind else {
            unreachable!()
        };
        assert!(seg.auto_route);
        assert_eq!(
            seg.arrow_heads(true),
            ArrowHeads {
                start: true,
                end: true
            }
        );
    });
}
