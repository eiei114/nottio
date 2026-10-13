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

#[test]
fn manual_portion_paths_follow_endpoints_and_survive_legacy_json() {
    let mut seg = SegGeom::between([100.0, 100.0], [400.0, 300.0], 2.5);
    assert!(seg.set_manual_path(&[
        [100.0, 100.0],
        [250.0, 100.0],
        [250.0, 300.0],
        [400.0, 300.0]
    ]));
    seg.move_endpoint(0, [120.0, 150.0]);
    seg.move_endpoint(1, [430.0, 330.0]);
    assert_eq!(
        seg.elbow_points(),
        vec![
            [120.0, 150.0],
            [250.0, 150.0],
            [250.0, 330.0],
            [430.0, 330.0]
        ]
    );
    let back: SegGeom = serde_json::from_str(&serde_json::to_string(&seg).unwrap()).unwrap();
    assert!(back.manual_polyline);
    assert_eq!(back.elbow_points(), seg.elbow_points());
    let legacy: SegGeom = serde_json::from_str(
        r#"{"x1":100,"y1":100,"x2":400,"y2":300,"width":2.5,"bend":[250,200]}"#,
    )
    .unwrap();
    assert!(!legacy.manual_polyline);
    assert_eq!(legacy.bend, Some([250.0, 200.0]));
    // Coincident imported endpoints must not crash during reconnection.
    let mut zero = SegGeom::between([100.0, 100.0], [100.0, 100.0], 2.5);
    zero.manual_polyline = true;
    zero.move_endpoint(1, [200.0, 200.0]);
    assert_eq!([zero.x2, zero.y2], [200.0, 200.0]);
}

#[test]
fn saved_diagonal_bent_path_is_canonicalized_and_cannot_be_rotated_directly() {
    let json = r#"{"elements":[{"id":7,"kind":{"arrow":{"x1":419,"y1":566,"x2":798,"y2":548,"width":2.5,"connector_type":"bent","bend":[443,566],"extra_bends":[[443,449],[659,516],[798,430]],"manual_polyline":true,"start_anchor":{"element_id":5,"connector":1},"end_anchor":{"element_id":9,"connector":0}}}}]}"#;
    let scene = Scene::from_json(json);
    let ElementKind::Arrow(seg) = scene.elements[0].kind else {
        unreachable!()
    };
    assert!(!rotatable(&scene.elements[0].kind));
    let path = seg.elbow_points();
    for pair in path.windows(2) {
        assert!(pair[0][0] == pair[1][0] || pair[0][1] == pair[1][1]);
    }
    assert_eq!(path.first(), Some(&[419.0, 566.0]));
    assert_eq!(path.last(), Some(&[798.0, 548.0]));
    // Persist the repair, not just a different paint path.
    let stored: Scene = serde_json::from_str(&scene.to_json()).unwrap();
    let ElementKind::Arrow(back) = stored.elements[0].kind else {
        unreachable!()
    };
    let stored_vertices: Vec<_> = std::iter::once([back.x1, back.y1])
        .chain(back.manual_bends())
        .chain(std::iter::once([back.x2, back.y2]))
        .collect();
    for pair in stored_vertices.windows(2) {
        assert!(pair[0][0] == pair[1][0] || pair[0][1] == pair[1][1]);
    }
    let mut kind = ElementKind::Arrow(seg);
    rotate_element(&mut kind, 0.0, 0.0, 0.3);
    let ElementKind::Arrow(rotated) = kind else {
        unreachable!()
    };
    for pair in rotated.elbow_points().windows(2) {
        assert!(pair[0][0] == pair[1][0] || pair[0][1] == pair[1][1]);
    }
}

#[gpui::test]
fn figjam_connector_shortcuts_and_control_bypass_work(cx: &mut TestAppContext) {
    let (board, mut cx) = fixture(cx);
    board.update_in(&mut cx, |board, window, cx| {
        board.scene.elements = vec![
            Element::sticky(1, [200.0, 280.0]),
            Element::sticky(2, [660.0, 280.0]),
        ];
        board.next_id = 3;
        board.active_connector_type = ConnectorType::Curved;
        board.focus.focus(window, cx);
    });
    cx.simulate_keystrokes("x");
    board.read_with(&cx, |board, _| {
        assert_eq!(board.tool, Tool::Arrow);
        assert_eq!(board.active_connector_type, ConnectorType::Bent);
    });
    let start = point(px(200.0), px(280.0));
    let end = point(px(660.0), px(280.0));
    cx.simulate_mouse_down(start, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(
        end,
        Some(MouseButton::Left),
        gpui::Modifiers {
            control: true,
            ..Default::default()
        },
    );
    cx.simulate_mouse_up(end, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert!(seg.start_anchor.is_some());
        assert!(
            seg.end_anchor.is_none(),
            "Ctrl bypasses target side snapping"
        );
        assert_eq!([seg.x2, seg.y2], [660.0, 280.0]);
    });
    cx.simulate_keystrokes("escape");
    board.read_with(&cx, |board, _| assert_eq!(board.tool, Tool::Select));
    cx.simulate_keystrokes("shift-l");
    board.read_with(&cx, |board, _| assert_eq!(board.tool, Tool::Arrow));
    cx.simulate_keystrokes("l");
    board.read_with(&cx, |board, _| assert_eq!(board.tool, Tool::Line));
    cx.simulate_keystrokes("j");
    board.read_with(&cx, |board, _| assert_eq!(board.tool, Tool::Hexagon));
    cx.simulate_keystrokes("shift-s");
    board.read_with(&cx, |board, _| assert_eq!(board.tool, Tool::Frame));
}

#[gpui::test]
fn frame_tool_groups_overlaps_moves_followers_and_restores_undo(cx: &mut TestAppContext) {
    let (board, mut cx) = fixture(cx);
    board.update_in(&mut cx, |board, window, cx| {
        let left = Element::sticky(1, [220.0, 270.0]);
        let mut triangle = Element::sticky(2, [440.0, 270.0]);
        let ElementKind::Sticky(bounds) = triangle.kind else {
            unreachable!()
        };
        triangle.kind = ElementKind::Triangle(bounds);
        let partial = Element::sticky(3, [689.0, 320.0]); // x=599, one unit overlaps.
        let outside = Element::sticky(5, [820.0, 270.0]);
        let mut arrow = Element::sticky(4, [0.0, 0.0]);
        let mut seg = SegGeom::between([310.0, 270.0], [350.0, 270.0], 2.5);
        seg.start_anchor = Some(SegmentAnchor {
            element_id: 1,
            connector: 1,
        });
        seg.end_anchor = Some(SegmentAnchor {
            element_id: 2,
            connector: 3,
        });
        seg.heads = Some(ArrowHeads {
            start: true,
            end: true,
        });
        arrow.kind = ElementKind::Arrow(seg);
        let mut crossing = Element::sticky(6, [0.0, 0.0]);
        let mut line = SegGeom::between([580.0, 420.0], [720.0, 420.0], 2.5);
        line.connector_type = ConnectorType::Straight;
        line.auto_route = false;
        crossing.kind = ElementKind::Line(line);
        board.scene.elements = vec![left, triangle, partial, arrow, outside, crossing];
        board.next_id = 7;
        board.focus.focus(window, cx);
        cx.notify();
    });
    let tool = cx.debug_bounds("frame-tool").unwrap().center();
    cx.simulate_click(tool, Default::default());
    let from = point(px(100.0), px(150.0));
    let to = point(px(600.0), px(470.0));
    cx.simulate_mouse_down(from, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(to, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(to, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        assert!(
            matches!(board.scene.elements[0].kind, ElementKind::Frame(_)),
            "frame stays behind existing content"
        );
        assert_eq!(board.selected, vec![7]);
        assert_eq!(
            frame::expand_selection(&board.scene, &[7]),
            vec![7, 1, 2, 3, 4, 6]
        );
        assert!(!connector_capable(&board.scene.elements[0].kind));
    });
    // Rename the frame's header through the shared native text editor.
    let header = point(px(160.0), px(135.0));
    cx.simulate_event(MouseDownEvent {
        position: header,
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 2,
        first_mouse: false,
    });
    cx.simulate_mouse_up(header, MouseButton::Left, Default::default());
    cx.simulate_keystrokes("ctrl-a");
    cx.simulate_input("整理フレーム");
    cx.simulate_keystrokes("escape");
    let start_scene = board.read_with(&cx, |board, _| board.scene.clone());
    assert_eq!(
        start_scene.elements[0].label.as_deref(),
        Some("整理フレーム")
    );
    let moved = point(px(340.0), px(245.0));
    cx.simulate_mouse_down(header, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(moved, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(moved, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        for id in [7, 1, 2, 3, 6] {
            let before = bbox(
                &start_scene
                    .elements
                    .iter()
                    .find(|e| e.id == id)
                    .unwrap()
                    .kind,
            );
            let after = bbox(
                &board
                    .scene
                    .elements
                    .iter()
                    .find(|e| e.id == id)
                    .unwrap()
                    .kind,
            );
            assert_eq!(
                after,
                (
                    before.0 + 180.0,
                    before.1 + 110.0,
                    before.2 + 180.0,
                    before.3 + 110.0
                )
            );
        }
        let outside = board.scene.elements.iter().find(|e| e.id == 5).unwrap();
        assert_eq!(
            bbox(&outside.kind),
            bbox(
                &start_scene
                    .elements
                    .iter()
                    .find(|e| e.id == 5)
                    .unwrap()
                    .kind
            ),
            "do not scoop up objects during a frame drag"
        );
        let ElementKind::Arrow(seg) = board
            .scene
            .elements
            .iter()
            .find(|e| e.id == 4)
            .unwrap()
            .kind
        else {
            unreachable!()
        };
        assert_eq!(
            [seg.x1, seg.y1, seg.x2, seg.y2],
            [490.0, 380.0, 530.0, 380.0]
        );
        assert_eq!(seg.start_anchor.unwrap().element_id, 1);
        assert_eq!(seg.end_anchor.unwrap().element_id, 2);
        let restored = Scene::from_json(&board.scene.to_json());
        assert!(matches!(restored.elements[0].kind, ElementKind::Frame(_)));
        assert!(frame::expand_selection(&restored, &[7]).contains(&3));
    });
    cx.simulate_keystrokes("ctrl-z");
    board.read_with(&cx, |board, _| {
        assert_eq!(board.scene.to_json(), start_scene.to_json())
    });
    cx.simulate_keystrokes("ctrl-shift-z");
    let before_cancel = board.read_with(&cx, |board, _| board.scene.to_json());
    cx.simulate_mouse_down(moved, MouseButton::Left, Default::default());
    let cancelled = point(px(340.0), px(290.0));
    cx.simulate_mouse_move(cancelled, Some(MouseButton::Left), Default::default());
    cx.simulate_keystrokes("escape");
    cx.simulate_mouse_up(cancelled, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        assert_eq!(board.scene.to_json(), before_cancel)
    });
    // A member can leave the section independently. Its external connector
    // remains bound when the section later moves without that member.
    let inside = point(px(620.0), px(380.0));
    let left_out = point(px(60.0), px(650.0));
    cx.simulate_mouse_down(inside, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(left_out, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(left_out, MouseButton::Left, Default::default());
    let outside_bounds = board.read_with(&cx, |board, _| {
        let triangle = board.scene.elements.iter().find(|e| e.id == 2).unwrap();
        assert_eq!(frame::owner(&board.scene, triangle), None);
        bbox(&triangle.kind)
    });
    cx.simulate_mouse_down(moved, MouseButton::Left, Default::default());
    let again = point(px(440.0), px(245.0));
    cx.simulate_mouse_move(again, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(again, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        assert_eq!(
            bbox(
                &board
                    .scene
                    .elements
                    .iter()
                    .find(|e| e.id == 2)
                    .unwrap()
                    .kind
            ),
            outside_bounds
        );
        let ElementKind::Arrow(seg) = board
            .scene
            .elements
            .iter()
            .find(|e| e.id == 4)
            .unwrap()
            .kind
        else {
            unreachable!()
        };
        assert_eq!(seg.end_anchor.unwrap().element_id, 2);
        assert_eq!(
            [seg.x2, seg.y2],
            connector_points(
                &board
                    .scene
                    .elements
                    .iter()
                    .find(|e| e.id == 2)
                    .unwrap()
                    .kind
            )[3]
        );
    });
}

#[gpui::test]
fn frame_click_places_default_square_and_enter_finishes_rename(cx: &mut TestAppContext) {
    let (board, mut cx) = fixture(cx);
    board.update_in(&mut cx, |board, window, cx| board.focus.focus(window, cx));
    cx.simulate_keystrokes("shift-s");
    cx.simulate_click(point(px(200.0), px(250.0)), Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Frame(b) = board.scene.elements[0].kind else {
            unreachable!()
        };
        assert_eq!([b.x, b.y, b.w, b.h], [200.0, 250.0, 400.0, 400.0]);
    });
    let header = point(px(240.0), px(234.0));
    cx.simulate_event(MouseDownEvent {
        position: header,
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 2,
        first_mouse: false,
    });
    cx.simulate_mouse_up(header, MouseButton::Left, Default::default());
    cx.simulate_keystrokes("ctrl-a");
    cx.simulate_input("キャンバス");
    cx.simulate_keystrokes("enter");
    board.read_with(&cx, |board, _| {
        assert!(board.editing.is_none());
        assert_eq!(board.scene.elements[0].label.as_deref(), Some("キャンバス"));
    });
}

#[gpui::test]
fn portions_add_folds_drag_cancel_and_save_through_ui(cx: &mut TestAppContext) {
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
    assert!(cx.debug_bounds("arrow-add-bend").is_none());
    assert!(cx.debug_bounds("arrow-remove-bend").is_none());
    let handle = cx.debug_bounds("connector-portion-0").unwrap().center();
    // Merely clicking a path handle is not a document edit or an undo entry.
    cx.simulate_click(handle, Default::default());
    board.read_with(&cx, |board, _| assert!(board.history.is_empty()));
    let top = point(px(430.0), px(160.0));
    cx.simulate_mouse_down(handle, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(top, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(top, MouseButton::Left, Default::default());
    let before = board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert_eq!(seg.elbow_points().len(), 6);
        assert!(seg.manual_polyline);
        assert!(!seg.auto_route);
        seg.elbow_points()
    });
    let handle = cx.debug_bounds("connector-portion-2").unwrap().center();
    let moved = point(px(490.0), px(200.0));
    cx.simulate_mouse_down(handle, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(moved, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(moved, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        let path = seg.elbow_points();
        assert_eq!(path.first(), before.first());
        assert_eq!(path.last(), before.last());
        assert_eq!(path[2], [before[2][0], 200.0]);
        assert_eq!(path[3], [before[3][0], 200.0]);
        assert_eq!(path.len(), before.len());
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
        assert!(back.manual_polyline);
    });
    // Dragging the end portion grows more folds automatically, keeping its
    // attached endpoint in place. No add-fold button or corner selection.
    let handle = cx.debug_bounds("connector-portion-4").unwrap().center();
    let lower = point(handle.x, handle.y + px(40.0));
    cx.simulate_mouse_down(handle, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(lower, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(lower, MouseButton::Left, Default::default());
    let expanded = board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert_eq!(seg.elbow_points().len(), 8);
        assert_eq!([seg.x2, seg.y2], [570.0, 280.0]);
        seg.elbow_points()
    });
    let handle = cx.debug_bounds("connector-portion-2").unwrap().center();
    let changed = point(handle.x, handle.y - px(30.0));
    cx.simulate_mouse_down(handle, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(changed, Some(MouseButton::Left), Default::default());
    cx.simulate_keystrokes("escape");
    cx.simulate_mouse_up(changed, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert_eq!(seg.elbow_points(), expanded);
    });
    cx.simulate_keystrokes("ctrl-z");
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert_eq!(seg.elbow_points().len(), 6);
    });
    cx.simulate_keystrokes("ctrl-shift-z");
    cx.simulate_click(point(px(430.0), px(200.0)), Default::default());
    // Moving the middle portion back onto the original baseline collapses its
    // adjacent folds on release, instead of requiring a deletion button.
    let handle = cx.debug_bounds("connector-portion-2").unwrap().center();
    let collapsed = point(handle.x, px(280.0));
    cx.simulate_mouse_down(handle, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(collapsed, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(collapsed, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert!(seg.elbow_points().len() < expanded.len());
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
    let handle = cx.debug_bounds("connector-portion-0").unwrap().center();
    let elbow = point(px(440.0), px(160.0));
    cx.simulate_mouse_down(handle, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(elbow, Some(MouseButton::Left), Default::default());
    cx.simulate_mouse_up(elbow, MouseButton::Left, Default::default());
    board.read_with(&cx, |board, _| {
        let ElementKind::Arrow(seg) = board.scene.elements[2].kind else {
            unreachable!()
        };
        assert!(seg.manual_polyline);
        assert!(seg.elbow_points().iter().any(|p| p[1] == 160.0));
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
        assert!(seg.manual_polyline);
        assert!(seg.elbow_points().iter().any(|p| p[1] == 160.0));
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
    assert!(cx.debug_bounds("connector-portion-0").is_none());
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
