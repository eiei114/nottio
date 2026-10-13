//! Sections are background frames. Membership is derived from positive overlap,
//! captured at the start of a drag so crossing other objects cannot scoop them up.
use super::*;

fn segment_intersects(a: [f32; 2], b: [f32; 2], rect: [f32; 4]) -> bool {
    let mut low = 0.0_f32;
    let mut high = 1.0_f32;
    for axis in 0..2 {
        let delta = b[axis] - a[axis];
        if delta.abs() < 0.001 {
            if a[axis] <= rect[axis] || a[axis] >= rect[axis + 2] {
                return false;
            }
        } else {
            let t1 = (rect[axis] - a[axis]) / delta;
            let t2 = (rect[axis + 2] - a[axis]) / delta;
            low = low.max(t1.min(t2));
            high = high.min(t1.max(t2));
            if low >= high {
                return false;
            }
        }
    }
    low < high
}

fn overlaps(kind: &ElementKind, frame: BoxGeom) -> bool {
    let rect = [frame.x, frame.y, frame.x + frame.w, frame.y + frame.h];
    let stroke_hits = |points: Vec<[f32; 2]>, width: f32| {
        let r = [
            rect[0] - width / 2.0,
            rect[1] - width / 2.0,
            rect[2] + width / 2.0,
            rect[3] + width / 2.0,
        ];
        points.windows(2).any(|p| segment_intersects(p[0], p[1], r))
    };
    match kind {
        ElementKind::Frame(_) => false,
        ElementKind::Arrow(s) | ElementKind::Line(s) => stroke_hits(s.points(), s.width),
        ElementKind::Draw(s) => stroke_hits(s.points.clone(), s.width),
        _ => {
            let b = bbox(kind);
            b.0 < rect[2] && b.2 > rect[0] && b.1 < rect[3] && b.3 > rect[1]
        }
    }
}

/// An object belongs to one frame: the smallest overlapping frame, then the
/// most recent/frontmost frame on a tie. Frame nesting is deliberately excluded.
pub(crate) fn owner(scene: &Scene, element: &Element) -> Option<u64> {
    scene
        .elements
        .iter()
        .rev()
        .filter_map(|e| {
            if let ElementKind::Frame(b) = e.kind
                && overlaps(&element.kind, b)
            {
                Some((e.id, b.w * b.h))
            } else {
                None
            }
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|p| p.0)
}

pub(crate) fn expand_selection(scene: &Scene, selected: &[u64]) -> Vec<u64> {
    let frames: HashSet<_> = scene
        .elements
        .iter()
        .filter(|e| selected.contains(&e.id) && matches!(e.kind, ElementKind::Frame(_)))
        .map(|e| e.id)
        .collect();
    let mut result = selected.to_vec();
    for e in &scene.elements {
        if !result.contains(&e.id) && owner(scene, e).is_some_and(|id| frames.contains(&id)) {
            result.push(e.id);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn images_follow_overlapping_frames_once_and_leave_on_full_exit() {
        let section = Element::frame(
            1,
            BoxGeom {
                x: 100.0,
                y: 100.0,
                w: 300.0,
                h: 240.0,
                width: 1.0,
                rotation: 0.0,
            },
        );
        let mut image = Element::sticky(2, [0.0, 0.0]);
        image.kind = ElementKind::Image(ImageGeom {
            src: "data:image/png;base64,test".into(),
            x: 399.0,
            y: 200.0,
            w: 80.0,
            h: 40.0,
            rotation: 0.0,
        });
        let mut scene = Scene {
            elements: vec![section, image],
            ..Default::default()
        };
        assert_eq!(expand_selection(&scene, &[1, 2]), vec![1, 2]);
        for element in &mut scene.elements {
            translate(&mut element.kind, 20.0, 30.0);
        }
        assert_eq!(owner(&scene, &scene.elements[1]), Some(1));
        let ElementKind::Image(image) = &scene.elements[1].kind else {
            panic!()
        };
        assert_eq!((image.x, image.y), (419.0, 230.0));
        translate(&mut scene.elements[1].kind, 2.0, 0.0);
        assert_eq!(owner(&scene, &scene.elements[1]), None);
    }
    #[test]
    fn slight_overlap_enters_and_full_exit_leaves() {
        let section = Element::frame(
            1,
            BoxGeom {
                x: 100.0,
                y: 100.0,
                w: 300.0,
                h: 240.0,
                width: 1.0,
                rotation: 0.0,
            },
        );
        let mut note = Element::sticky(2, [489.0, 200.0]); // left edge 399: overlaps by 1.
        let mut scene = Scene {
            elements: vec![section, note.clone()],
            ..Default::default()
        };
        assert_eq!(owner(&scene, &note), Some(1));
        assert_eq!(expand_selection(&scene, &[1]), vec![1, 2]);
        translate(&mut note.kind, 2.0, 0.0);
        assert_eq!(owner(&scene, &note), None);
        scene.elements[1] = note;
        assert_eq!(expand_selection(&scene, &[1]), vec![1]);
    }
    #[test]
    fn line_membership_uses_actual_path_not_empty_bbox_area() {
        let section = Element::frame(
            1,
            BoxGeom {
                x: 100.0,
                y: 100.0,
                w: 100.0,
                h: 100.0,
                width: 1.0,
                rotation: 0.0,
            },
        );
        let mut line = Element::sticky(2, [0.0, 0.0]);
        let mut seg = SegGeom::between([50.0, 150.0], [250.0, 150.0], 2.5);
        seg.connector_type = ConnectorType::Straight;
        line.kind = ElementKind::Line(seg);
        let scene = Scene {
            elements: vec![section],
            ..Default::default()
        };
        assert_eq!(owner(&scene, &line), Some(1));
        seg.x1 = 50.0;
        seg.y1 = 190.0;
        seg.x2 = 190.0;
        seg.y2 = 250.0;
        line.kind = ElementKind::Line(seg);
        assert_eq!(
            owner(&scene, &line),
            None,
            "diagonal bbox overlaps but line does not"
        );
    }
    #[test]
    fn overlapping_frames_never_move_a_member_twice() {
        let a = Element::frame(
            1,
            BoxGeom {
                x: 100.0,
                y: 100.0,
                w: 400.0,
                h: 400.0,
                width: 1.0,
                rotation: 0.0,
            },
        );
        let b = Element::frame(
            2,
            BoxGeom {
                x: 200.0,
                y: 200.0,
                w: 200.0,
                h: 200.0,
                width: 1.0,
                rotation: 0.0,
            },
        );
        let note = Element::sticky(3, [300.0, 300.0]);
        let scene = Scene {
            elements: vec![a, b, note.clone()],
            ..Default::default()
        };
        assert_eq!(owner(&scene, &note), Some(2));
        assert_eq!(expand_selection(&scene, &[1, 2]), vec![1, 2, 3]);
        assert_eq!(expand_selection(&scene, &[1]), vec![1]);
    }
}
