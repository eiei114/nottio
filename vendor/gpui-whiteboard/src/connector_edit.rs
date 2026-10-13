//! FigJam-style orthogonal portion editing. Document endpoints never move when
//! a path portion is dragged; terminal portions grow a dogleg instead.
pub(crate) struct PortionDrag {
    pub id: u64,
    pub index: usize,
    pub path: Vec<[f32; 2]>,
    pub from: [f32; 2],
    pub started: bool,
}

pub(crate) fn clean_path(points: Vec<[f32; 2]>) -> Vec<[f32; 2]> {
    let mut result: Vec<[f32; 2]> = Vec::new();
    for p in points {
        if result
            .last()
            .is_some_and(|last| (last[0] - p[0]).hypot(last[1] - p[1]) < 0.01)
        {
            continue;
        }
        while result.len() >= 2 {
            let a = result[result.len() - 2];
            let b = *result.last().unwrap();
            let u = [b[0] - a[0], b[1] - a[1]];
            let v = [p[0] - b[0], p[1] - b[1]];
            if (u[0] * v[1] - u[1] * v[0]).abs() > 0.01 || u[0] * v[0] + u[1] * v[1] < 0.0 {
                break;
            }
            result.pop();
        }
        result.push(p);
    }
    result
}

pub(crate) fn horizontal(a: [f32; 2], b: [f32; 2]) -> bool {
    (b[0] - a[0]).abs() >= (b[1] - a[1]).abs()
}

/// Repair imported/transformed diagonal legs without moving their endpoints or
/// losing the user's waypoints. Bent geometry is always an orthogonal polyline.
pub(crate) fn orthogonal_path(
    points: Vec<[f32; 2]>,
    start_horizontal: Option<bool>,
    end_horizontal: Option<bool>,
) -> Vec<[f32; 2]> {
    if points.len() < 2 {
        return points;
    }
    let mut result = vec![points[0]];
    for (index, pair) in points.windows(2).enumerate() {
        let a = pair[0];
        let b = pair[1];
        if (a[0] - b[0]).abs() > 0.001 && (a[1] - b[1]).abs() > 0.001 {
            let first = if index == 0 {
                start_horizontal.unwrap_or(horizontal(a, b))
            } else {
                !horizontal(result[result.len() - 2], *result.last().unwrap())
            };
            let last = if index + 2 == points.len() {
                end_horizontal.unwrap_or(!first)
            } else {
                let next = points[index + 2];
                if (next[0] - b[0]).abs() < 0.001 || (next[1] - b[1]).abs() < 0.001 {
                    !horizontal(b, next)
                } else {
                    !first
                }
            };
            if first == last {
                if first {
                    let x = (a[0] + b[0]) / 2.0;
                    result.extend([[x, a[1]], [x, b[1]]]);
                } else {
                    let y = (a[1] + b[1]) / 2.0;
                    result.extend([[a[0], y], [b[0], y]]);
                }
            } else {
                result.push(if first { [b[0], a[1]] } else { [a[0], b[1]] });
            }
        }
        result.push(b);
    }
    clean_path(result)
}

pub(crate) fn drag_portion(path: &[[f32; 2]], index: usize, delta: f32) -> Vec<[f32; 2]> {
    if path.len() < 2 || index + 1 >= path.len() || delta.abs() < 0.01 {
        return path.to_vec();
    }
    if index == path.len() - 2 && index != 0 {
        let reversed: Vec<_> = path.iter().rev().copied().collect();
        let mut moved = drag_portion(&reversed, 0, delta);
        moved.reverse();
        return moved;
    }
    let a = path[index];
    let b = path[index + 1];
    let axis = if horizontal(a, b) { 1 } else { 0 };
    if index != 0 {
        let mut moved = path.to_vec();
        moved[index][axis] += delta;
        moved[index + 1][axis] += delta;
        return moved;
    }
    let along = 1 - axis;
    let offset = (b[along] - a[along]).abs().min(72.0) / 3.0;
    let sign = if b[along] >= a[along] { 1.0 } else { -1.0 };
    let mut stub = a;
    stub[along] += sign * offset;
    let mut moved_stub = stub;
    moved_stub[axis] += delta;
    let mut moved_b = b;
    moved_b[axis] += delta;
    let mut result = vec![a, stub, moved_stub];
    if path.len() == 2 {
        // A one-portion path has TWO fixed endpoints. Add exits at both ends.
        let mut end_stub = b;
        end_stub[along] -= sign * offset;
        let mut moved_end_stub = end_stub;
        moved_end_stub[axis] += delta;
        result.extend([moved_end_stub, end_stub, b]);
    } else {
        result.push(moved_b);
        result.extend_from_slice(&path[2..]);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn orthogonal(path: &[[f32; 2]]) {
        for p in path.windows(2) {
            assert!(p[0][0] == p[1][0] || p[0][1] == p[1][1]);
        }
    }

    #[test]
    fn observed_diagonal_connector_is_repaired_to_right_angles() {
        let path = vec![
            [419.0, 566.0],
            [443.0, 566.0],
            [443.0, 449.0],
            [659.0, 516.0],
            [798.0, 430.0],
            [798.0, 548.0],
        ];
        let repaired = orthogonal_path(path.clone(), Some(true), Some(false));
        orthogonal(&repaired);
        assert_eq!(repaired.first(), path.first());
        assert_eq!(repaired.last(), path.last());
        for vertex in path {
            assert!(repaired.contains(&vertex));
        }
    }

    #[test]
    fn diagonal_terminals_respect_both_exit_axes() {
        for first in [true, false] {
            for last in [true, false] {
                let path = orthogonal_path(
                    vec![[100.0, 100.0], [400.0, 300.0]],
                    Some(first),
                    Some(last),
                );
                orthogonal(&path);
                assert_eq!(horizontal(path[0], path[1]), first);
                assert_eq!(horizontal(path[path.len() - 2], path[path.len() - 1]), last);
            }
        }
    }

    #[test]
    fn terminal_and_single_portions_add_folds_without_moving_endpoints() {
        for path in [
            vec![[100.0, 100.0], [400.0, 100.0]],
            vec![
                [100.0, 100.0],
                [250.0, 100.0],
                [250.0, 300.0],
                [400.0, 300.0],
            ],
        ] {
            for index in [0, path.len() - 2] {
                let moved = drag_portion(&path, index, 60.0);
                assert_eq!(moved.first(), path.first());
                assert_eq!(moved.last(), path.last());
                assert!(moved.len() > path.len());
                orthogonal(&moved);
            }
        }
    }

    #[test]
    fn interior_portions_move_on_one_axis_and_can_collapse_folds() {
        let path = [
            [100.0, 100.0],
            [250.0, 100.0],
            [250.0, 300.0],
            [400.0, 300.0],
        ];
        let moved = drag_portion(&path, 1, 50.0);
        assert_eq!(
            moved,
            vec![
                [100.0, 100.0],
                [300.0, 100.0],
                [300.0, 300.0],
                [400.0, 300.0]
            ]
        );
        orthogonal(&moved);
        let simple = clean_path(drag_portion(&path, 1, -150.0));
        assert_eq!(simple, vec![[100.0, 100.0], [100.0, 300.0], [400.0, 300.0]]);
    }

    #[test]
    fn cleanup_preserves_endpoints_and_reversals() {
        assert_eq!(
            clean_path(vec![[0.0, 0.0], [10.0, 0.0], [20.0, 0.0], [20.0, 0.0]]),
            vec![[0.0, 0.0], [20.0, 0.0]]
        );
        assert_eq!(
            clean_path(vec![[0.0, 0.0], [20.0, 0.0], [10.0, 0.0]]),
            vec![[0.0, 0.0], [20.0, 0.0], [10.0, 0.0]]
        );
    }
}
