//! Bounded, camera-independent orthogonal routing around board objects.
//! Routes are a render cache; only the user's auto-route preference is saved.
use super::*;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

const MARGIN: f32 = 18.0;
const EPS: f32 = 0.001;
const MAX_GRID: usize = 4096;
const MAX_OBSTACLES: usize = 64;

#[derive(Clone, Copy, Debug)]
struct Obstacle {
    id: u64,
    rect: [f32; 4],
    rotation: f32,
}

#[derive(Clone, Copy, Debug)]
struct QueueEntry {
    cost: f32,
    state: usize,
}
impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.cost.to_bits() == other.cost.to_bits() && self.state == other.state
    }
}
impl Eq for QueueEntry {}
impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| self.state.cmp(&other.state))
    }
}

fn inside(p: [f32; 2], rect: [f32; 4]) -> bool {
    p[0] > rect[0] + EPS && p[0] < rect[2] - EPS && p[1] > rect[1] + EPS && p[1] < rect[3] - EPS
}

fn hits_rect(a: [f32; 2], b: [f32; 2], rect: [f32; 4]) -> bool {
    let mut enter: f32 = 0.0;
    let mut exit: f32 = 1.0;
    for axis in 0..2 {
        let low = rect[axis] + EPS;
        let high = rect[axis + 2] - EPS;
        let delta = b[axis] - a[axis];
        if delta.abs() < EPS {
            if a[axis] <= low || a[axis] >= high {
                return false;
            }
        } else {
            let t1 = (low - a[axis]) / delta;
            let t2 = (high - a[axis]) / delta;
            enter = enter.max(t1.min(t2));
            exit = exit.min(t1.max(t2));
            if enter > exit {
                return false;
            }
        }
    }
    enter <= exit
}

fn clear(a: [f32; 2], b: [f32; 2], obstacles: &[Obstacle], skip: Option<u64>) -> bool {
    obstacles
        .iter()
        .filter(|o| Some(o.id) != skip)
        .all(|o| !hits_rect(a, b, o.rect))
}

/// Leave a terminal object's padded bounds through its actual connector side.
/// Terminal legs may enter their own object, but never an unrelated object.
fn escape(
    p: [f32; 2],
    toward: [f32; 2],
    anchor: Option<SegmentAnchor>,
    obstacles: &[Obstacle],
) -> ([f32; 2], Option<u64>) {
    let obstacle = anchor
        .and_then(|anchor| obstacles.iter().find(|o| o.id == anchor.element_id))
        .filter(|o| inside(p, o.rect))
        .or_else(|| obstacles.iter().find(|o| inside(p, o.rect)));
    let Some(obstacle) = obstacle else {
        return (p, None);
    };
    let dir =
        if let Some(anchor) = anchor.filter(|a| a.element_id == obstacle.id && a.connector < 4) {
            let local = [[0.0, -1.0], [1.0, 0.0], [0.0, 1.0], [-1.0, 0.0]][anchor.connector];
            let (s, c) = obstacle.rotation.sin_cos();
            [local[0] * c - local[1] * s, local[0] * s + local[1] * c]
        } else {
            let dx = toward[0] - p[0];
            let dy = toward[1] - p[1];
            if dx.abs() >= dy.abs() {
                [if dx >= 0.0 { 1.0 } else { -1.0 }, 0.0]
            } else {
                [0.0, if dy >= 0.0 { 1.0 } else { -1.0 }]
            }
        };
    // Bent connectors leave even rotated objects on a world cardinal axis.
    // A diagonal terminal stub would otherwise reintroduce a slanted portion.
    let dir = if dir[0].abs() >= dir[1].abs() {
        [dir[0].signum(), 0.0]
    } else {
        [0.0, dir[1].signum()]
    };
    let mut distance = f32::INFINITY;
    for axis in 0..2 {
        if dir[axis].abs() > EPS {
            let edge = if dir[axis] > 0.0 {
                obstacle.rect[axis + 2]
            } else {
                obstacle.rect[axis]
            };
            distance = distance.min((edge - p[axis]) / dir[axis]);
        }
    }
    (
        [
            p[0] + dir[0] * (distance + 1.0),
            p[1] + dir[1] * (distance + 1.0),
        ],
        Some(obstacle.id),
    )
}

fn simplify(points: Vec<[f32; 2]>) -> Vec<[f32; 2]> {
    let mut result: Vec<[f32; 2]> = Vec::new();
    for p in points {
        if result.last() == Some(&p) {
            continue;
        }
        while result.len() >= 2 {
            let a = result[result.len() - 2];
            let b = result[result.len() - 1];
            let cross = (b[0] - a[0]) * (p[1] - b[1]) - (b[1] - a[1]) * (p[0] - b[0]);
            let dot = (b[0] - a[0]) * (p[0] - b[0]) + (b[1] - a[1]) * (p[1] - b[1]);
            if cross.abs() > EPS || dot < 0.0 {
                break;
            }
            result.pop();
        }
        result.push(p);
    }
    result
}

fn route_segment(seg: &SegGeom, obstacles: &[Obstacle]) -> Option<Vec<[f32; 2]>> {
    if obstacles.len() > MAX_OBSTACLES {
        return None;
    }
    let start = [seg.x1, seg.y1];
    let end = [seg.x2, seg.y2];
    if !start.iter().chain(end.iter()).all(|v| v.is_finite()) {
        return None;
    }
    let pad = MARGIN + seg.width.max(0.0) / 2.0;
    let obstacles: Vec<_> = obstacles
        .iter()
        .map(|o| Obstacle {
            rect: [
                o.rect[0] - pad,
                o.rect[1] - pad,
                o.rect[2] + pad,
                o.rect[3] + pad,
            ],
            ..*o
        })
        .collect();
    let (a, source) = escape(start, end, seg.start_anchor, &obstacles);
    let (b, target) = escape(end, start, seg.end_anchor, &obstacles);
    if !clear(start, a, &obstacles, source) || !clear(b, end, &obstacles, target) {
        return None;
    }
    let mut xs = vec![a[0], b[0]];
    let mut ys = vec![a[1], b[1]];
    for obstacle in &obstacles {
        xs.extend([obstacle.rect[0], obstacle.rect[2]]);
        ys.extend([obstacle.rect[1], obstacle.rect[3]]);
    }
    xs.sort_by(f32::total_cmp);
    ys.sort_by(f32::total_cmp);
    xs.dedup();
    ys.dedup();
    let cols = xs.len();
    let rows = ys.len();
    if cols * rows > MAX_GRID {
        return None;
    }
    let points: Vec<_> = ys
        .iter()
        .flat_map(|y| xs.iter().map(move |x| [*x, *y]))
        .collect();
    let blocked: Vec<_> = points
        .iter()
        .map(|p| obstacles.iter().any(|o| inside(*p, o.rect)))
        .collect();
    let source_node = points.iter().position(|p| *p == a)?;
    let target_node = points.iter().position(|p| *p == b)?;
    if blocked[source_node] || blocked[target_node] {
        return None;
    }
    // Dijkstra over (grid point, incoming axis); turning has a small penalty so
    // equal-length routes prefer fewer corners. The heap makes this bounded grid
    // substantially cheaper than scanning all unvisited points every step.
    let mut costs = vec![f32::INFINITY; points.len() * 3];
    let mut previous = vec![usize::MAX; costs.len()];
    let mut heap = BinaryHeap::new();
    costs[source_node * 3] = 0.0;
    heap.push(QueueEntry {
        cost: 0.0,
        state: source_node * 3,
    });
    let mut finish = None;
    while let Some(QueueEntry { cost, state }) = heap.pop() {
        if cost > costs[state] {
            continue;
        }
        let node = state / 3;
        let axis = state % 3;
        if node == target_node {
            finish = Some(state);
            break;
        }
        let x = node % cols;
        let y = node / cols;
        let neighbours = [
            x.checked_sub(1).map(|_| (node - 1, 1)),
            (x + 1 < cols).then_some((node + 1, 1)),
            y.checked_sub(1).map(|_| (node - cols, 2)),
            (y + 1 < rows).then_some((node + cols, 2)),
        ];
        for (next, next_axis) in neighbours.into_iter().flatten() {
            if blocked[next] || !clear(points[node], points[next], &obstacles, None) {
                continue;
            }
            let length = (points[node][0] - points[next][0]).abs()
                + (points[node][1] - points[next][1]).abs();
            let next_cost = cost
                + length
                + if axis != 0 && axis != next_axis {
                    24.0
                } else {
                    0.0
                };
            let next_state = next * 3 + next_axis;
            if next_cost < costs[next_state] {
                costs[next_state] = next_cost;
                previous[next_state] = state;
                heap.push(QueueEntry {
                    cost: next_cost,
                    state: next_state,
                });
            }
        }
    }
    let mut state = finish?;
    let mut path = vec![end];
    loop {
        path.push(points[state / 3]);
        if previous[state] == usize::MAX {
            break;
        }
        state = previous[state];
    }
    path.push(start);
    path.reverse();
    Some(simplify(path))
}

impl WhiteboardView {
    pub(super) fn update_routes(&mut self) {
        let obstacles: Vec<_> = self
            .scene
            .elements
            .iter()
            .filter_map(|e| {
                if matches!(
                    e.kind,
                    ElementKind::Line(_)
                        | ElementKind::Arrow(_)
                        | ElementKind::Draw(_)
                        | ElementKind::Frame(_)
                ) {
                    return None;
                }
                let (x0, y0, x1, y1) = bbox(&e.kind);
                if ![x0, y0, x1, y1].iter().all(|v| v.is_finite()) {
                    return None;
                }
                Some(Obstacle {
                    id: e.id,
                    rect: [x0, y0, x1, y1],
                    rotation: box_like(&e.kind).map_or(0.0, |b| b.4),
                })
            })
            .collect();
        let mut hash = DefaultHasher::new();
        for o in &obstacles {
            o.id.hash(&mut hash);
            for v in o.rect {
                v.to_bits().hash(&mut hash);
            }
            o.rotation.to_bits().hash(&mut hash);
        }
        let segments: Vec<_> = self
            .scene
            .elements
            .iter()
            .filter_map(|e| {
                let (ElementKind::Line(seg) | ElementKind::Arrow(seg)) = e.kind else {
                    return None;
                };
                e.id.hash(&mut hash);
                seg.auto_route.hash(&mut hash);
                seg.manual_polyline.hash(&mut hash);
                seg.connector_type.hash(&mut hash);
                for v in [seg.x1, seg.y1, seg.x2, seg.y2, seg.width] {
                    v.to_bits().hash(&mut hash);
                }
                seg.bend.map(|p| p.map(f32::to_bits)).hash(&mut hash);
                for bend in seg.extra_bends.iter().flat_map(|p| p.as_slice()) {
                    bend.map(f32::to_bits).hash(&mut hash);
                }
                for anchor in [seg.start_anchor, seg.end_anchor] {
                    anchor.map(|a| (a.element_id, a.connector)).hash(&mut hash);
                }
                let style = self.mindmap_connector_style_for_element(&e.kind);
                style.map(|s| std::mem::discriminant(&s)).hash(&mut hash);
                Some((e.id, seg, style))
            })
            .collect();
        let signature = hash.finish();
        if self.route_signature == Some(signature) {
            return;
        }
        self.route_signature = Some(signature);
        for (id, seg, style) in segments {
            let enabled = seg.auto_route
                && seg.connector_type == ConnectorType::Bent
                && !seg.has_manual_bends()
                && style.is_none_or(|s| s == MindMapConnectorStyle::Straight);
            let path = enabled.then(|| route_segment(&seg, &obstacles)).flatten();
            let cached = path.and_then(|p| {
                if p.len() >= 2 {
                    RoutePath::new(&p[1..p.len() - 1])
                } else {
                    RoutePath::new(&[])
                }
            });
            if let Some(element) = self.scene.elements.iter_mut().find(|e| e.id == id)
                && let ElementKind::Arrow(s) | ElementKind::Line(s) = &mut element.kind
            {
                s.route = cached;
                s.route_failed = enabled && cached.is_none();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn segment() -> SegGeom {
        serde_json::from_str(r#"{"x1":0,"y1":50,"x2":300,"y2":50,"width":2.5}"#).unwrap()
    }
    #[test]
    fn route_detours_around_an_obstacle_with_clearance() {
        let seg = segment();
        let obstacle = Obstacle {
            id: 1,
            rect: [100.0, 0.0, 200.0, 100.0],
            rotation: 0.0,
        };
        let path = route_segment(&seg, &[obstacle]).unwrap();
        assert_eq!(path.first(), Some(&[0.0, 50.0]));
        assert_eq!(path.last(), Some(&[300.0, 50.0]));
        assert!(path.len() > 2);
        for pair in path.windows(2) {
            assert!(!hits_rect(pair[0], pair[1], obstacle.rect));
        }
        assert!(path.iter().any(|p| p[1] < -18.0 || p[1] > 118.0));
        let mut moved = obstacle;
        moved.rect = [100.0, 120.0, 200.0, 220.0];
        assert_eq!(
            route_segment(&seg, &[moved]).unwrap(),
            vec![[0.0, 50.0], [300.0, 50.0]]
        );
    }
    #[test]
    fn unobstructed_routes_stay_straight_and_capacity_is_bounded() {
        let seg = segment();
        assert_eq!(
            route_segment(&seg, &[]).unwrap(),
            vec![[0.0, 50.0], [300.0, 50.0]]
        );
        let obstacles: Vec<_> = (0..65)
            .map(|id| Obstacle {
                id,
                rect: [0.0, 0.0, 10.0, 10.0],
                rotation: 0.0,
            })
            .collect();
        assert!(route_segment(&seg, &obstacles).is_none());
    }
}
