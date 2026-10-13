//! The persisted scene model: [`Scene`] / [`Element`] / [`ElementKind`] and their
//! geometry payloads, rich-text style spans, [`Camera`], and the local-thumbnail
//! spec — the JSON-(de)serialized layer, split from `lib.rs`.

use super::*;

/// The board document: everything persisted for a whiteboard. Owned and
/// (de)serialized here; the host stores [`Scene::to_json`] opaquely (for Zorite,
/// in the `content` column of a `kind = 'whiteboard'` page).
///
/// Every field is `#[serde(default)]` so older boards keep loading as the model
/// grows.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Scene {
    /// The viewport (pan + zoom). Persisted so a board reopens where you left it.
    #[serde(default)]
    pub camera: Camera,
    /// The board's content, painted in z-order.
    #[serde(default)]
    pub elements: Vec<Element>,
}

impl Scene {
    /// Parse a board from its stored JSON, falling back to an empty board on
    /// empty or malformed input — a corrupt row never blocks opening the tab.
    pub fn from_json(s: &str) -> Self {
        let mut scene: Self = if s.trim().is_empty() {
            Self::default()
        } else {
            serde_json::from_str(s).unwrap_or_else(|e| {
                log::warn!("whiteboard: ignoring bad scene JSON ({e}); starting empty");
                Self::default()
            })
        };
        if !scene.camera.zoom.is_finite() || scene.camera.zoom <= 0.0 {
            scene.camera.zoom = 1.0;
        }
        for element in &mut scene.elements {
            if let ElementKind::Arrow(seg) | ElementKind::Line(seg) = &mut element.kind
                && seg.connector_type == ConnectorType::Bent
                && seg.manual_polyline
            {
                let path = seg.elbow_points();
                seg.set_manual_path(&path);
            }
        }
        scene
    }

    /// Serialize for persistence.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// One board element: a stable id plus its geometry/kind.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Element {
    pub id: u64,
    pub kind: ElementKind,
    /// Stroke / ink color, packed `0xRRGGBBAA`. `None` follows the theme ink, so
    /// uncolored elements still adapt to light/dark. Absent in older boards → 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<u32>,
    /// Fill color for closed shapes (rect/ellipse), packed `0xRRGGBBAA`. `None`
    /// is an unfilled outline. Ignored by other kinds. Absent in older boards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<u32>,
    /// Centered text label for closed shapes (rect / ellipse / …). Rendered at a
    /// font size auto-shrunk so the wrapped text never crosses the shape's
    /// border (see the shape paint path). `None` / empty = no label; ignored by
    /// non-shape kinds. Absent in older boards → `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Color of the shape's [`label`](Self::label), packed `0xRRGGBBAA`. `None`
    /// inks it with the shape's stroke (theme ink if that's unset too). Ignored by
    /// non-shape kinds. Absent in older boards → `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_color: Option<u32>,
    /// Rich-text formatting runs over the element's text (a Text element's
    /// content, or a shape's label): bold / italic / underline / strikethrough /
    /// highlight, each over a byte range. Empty = unstyled. Kept sorted +
    /// non-overlapping and maintained across edits; absent in older boards.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub styles: Vec<StyleSpan>,
    /// Optional whiteboard-native mind-map metadata. Only mind-map nodes carry
    /// this; lines stay regular anchored arrows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mindmap: Option<MindMapNodeMeta>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MindMapSide {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MindMapRootDirection {
    #[default]
    Both,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MindMapConnectorStyle {
    Straight,
    #[default]
    Bezier,
    Orthogonal,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MindMapNodeMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<u64>,
    pub side: MindMapSide,
    #[serde(default)]
    pub order: usize,
    #[serde(default)]
    pub root_direction: MindMapRootDirection,
    #[serde(default)]
    pub connector_style: MindMapConnectorStyle,
}

/// Whether a [`RunStyle`] flag is unset — its default, kept out of the JSON.
fn is_false(b: &bool) -> bool {
    !*b
}

/// The formatting of a run of characters; `default()` is plain text.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct RunStyle {
    #[serde(default, skip_serializing_if = "is_false")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub underline: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub strike: bool,
    /// Highlight color behind the glyphs, packed `0xRRGGBBAA`; `None` = none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight: Option<u32>,
}

impl RunStyle {
    /// No formatting — needn't be stored (plain runs are implicit gaps).
    fn is_plain(&self) -> bool {
        *self == RunStyle::default()
    }
}

/// A toggleable boolean format. (Highlight is a color, toggled on its own.)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    Bold,
    Italic,
    Underline,
    Strike,
}

impl Format {
    pub(crate) fn get(self, s: &RunStyle) -> bool {
        match self {
            Format::Bold => s.bold,
            Format::Italic => s.italic,
            Format::Underline => s.underline,
            Format::Strike => s.strike,
        }
    }
    pub(crate) fn set(self, s: &mut RunStyle, on: bool) {
        match self {
            Format::Bold => s.bold = on,
            Format::Italic => s.italic = on,
            Format::Underline => s.underline = on,
            Format::Strike => s.strike = on,
        }
    }
}

/// A [`RunStyle`] over the byte range `[start, end)` of an element's text. Runs
/// are kept sorted, non-overlapping, and non-plain. See [`Element::styles`].
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct StyleSpan {
    pub start: usize,
    pub end: usize,
    pub style: RunStyle,
}

/// `[s, e)` as contiguous `(start, end, style)` segments — existing runs plus
/// plain gaps. Assumes `spans` is sorted + non-overlapping.
fn style_segments(spans: &[StyleSpan], s: usize, e: usize) -> Vec<(usize, usize, RunStyle)> {
    let mut out = Vec::new();
    let mut pos = s;
    for sp in spans.iter().filter(|sp| sp.start < e && sp.end > s) {
        let a = sp.start.max(s);
        if a > pos {
            out.push((pos, a, RunStyle::default()));
        }
        out.push((a, sp.end.min(e), sp.style));
        pos = sp.end.min(e);
    }
    if pos < e {
        out.push((pos, e, RunStyle::default()));
    }
    out
}

/// Re-sort, merge touching equal runs, and drop plain runs.
fn normalize_styles(mut segs: Vec<(usize, usize, RunStyle)>) -> Vec<StyleSpan> {
    segs.retain(|(a, b, _)| a < b);
    segs.sort_by_key(|(a, _, _)| *a);
    let mut out: Vec<StyleSpan> = Vec::new();
    for (a, b, st) in segs {
        if st.is_plain() {
            continue;
        }
        if let Some(last) = out.last_mut()
            && last.end == a
            && last.style == st
        {
            last.end = b;
            continue;
        }
        out.push(StyleSpan {
            start: a,
            end: b,
            style: st,
        });
    }
    out
}

/// The style covering byte `offset` (the char starting there), else plain.
pub(crate) fn style_at(spans: &[StyleSpan], offset: usize) -> RunStyle {
    spans
        .iter()
        .find(|sp| sp.start <= offset && offset < sp.end)
        .map_or(RunStyle::default(), |sp| sp.style)
}

/// The formatting common to *every* char in `[s, e)` — for menu checkmarks. A
/// collapsed range reports the style just left of the caret (what typing
/// inherits).
pub(crate) fn active_style(spans: &[StyleSpan], s: usize, e: usize) -> RunStyle {
    if s >= e {
        return style_at(spans, s.saturating_sub(1));
    }
    let segs = style_segments(spans, s, e);
    let mut it = segs.iter().map(|&(_, _, st)| st);
    let Some(mut acc) = it.next() else {
        return RunStyle::default();
    };
    for st in it {
        acc.bold &= st.bold;
        acc.italic &= st.italic;
        acc.underline &= st.underline;
        acc.strike &= st.strike;
        acc.highlight = match (acc.highlight, st.highlight) {
            (Some(a), Some(b)) if a == b => Some(a),
            _ => None,
        };
    }
    acc
}

/// Replace the styling of `[s, e)` with `mid` (which covers `[s, e)`), keeping
/// runs outside (splitting any that straddle), then normalize.
fn replace_segment(
    spans: &[StyleSpan],
    s: usize,
    e: usize,
    mid: Vec<(usize, usize, RunStyle)>,
) -> Vec<StyleSpan> {
    let mut segs: Vec<(usize, usize, RunStyle)> = Vec::new();
    for sp in spans {
        if sp.start < s {
            segs.push((sp.start, sp.end.min(s), sp.style));
        }
        if sp.end > e {
            segs.push((sp.start.max(e), sp.end, sp.style));
        }
    }
    segs.extend(mid);
    normalize_styles(segs)
}

/// Toggle `format` over `[s, e)`: removed if every char already has it, else
/// added.
pub(crate) fn toggle_format(
    spans: &[StyleSpan],
    s: usize,
    e: usize,
    format: Format,
) -> Vec<StyleSpan> {
    if s >= e {
        return spans.to_vec();
    }
    let segs = style_segments(spans, s, e);
    let add = !segs.iter().all(|(_, _, st)| format.get(st));
    let mid = segs
        .into_iter()
        .map(|(a, b, mut st)| {
            format.set(&mut st, add);
            (a, b, st)
        })
        .collect();
    replace_segment(spans, s, e, mid)
}

/// Set/clear the highlight color over `[s, e)`: cleared if every char already
/// has `color`, else set to it.
pub(crate) fn toggle_highlight(
    spans: &[StyleSpan],
    s: usize,
    e: usize,
    color: u32,
) -> Vec<StyleSpan> {
    if s >= e {
        return spans.to_vec();
    }
    let segs = style_segments(spans, s, e);
    let on = !segs.iter().all(|(_, _, st)| st.highlight == Some(color));
    let mid = segs
        .into_iter()
        .map(|(a, b, mut st)| {
            st.highlight = on.then_some(color);
            (a, b, st)
        })
        .collect();
    replace_segment(spans, s, e, mid)
}

/// Shift/clip runs when the text `[s, e)` becomes `ins_len` bytes; the inserted
/// bytes take `insert_style`. Keeps runs aligned to the edited text.
pub(crate) fn splice_styles(
    spans: &[StyleSpan],
    s: usize,
    e: usize,
    ins_len: usize,
    insert_style: RunStyle,
) -> Vec<StyleSpan> {
    let delta = ins_len as isize - (e - s) as isize;
    let after = |p: usize| (p as isize + delta).max(0) as usize;
    let mut segs: Vec<(usize, usize, RunStyle)> = Vec::new();
    for sp in spans {
        if sp.start < s {
            segs.push((sp.start, sp.end.min(s), sp.style));
        }
        if sp.end > e {
            segs.push((after(sp.start.max(e)), after(sp.end), sp.style));
        }
    }
    if ins_len > 0 && !insert_style.is_plain() {
        segs.push((s, s + ins_len, insert_style));
    }
    normalize_styles(segs)
}

/// The kinds of thing a board can hold.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementKind {
    Draw(Stroke),
    Rect(BoxGeom),
    Ellipse(BoxGeom),
    Diamond(BoxGeom),
    Triangle(BoxGeom),
    RoundRect(BoxGeom),
    /// A sticky note, distinct from shapes in the persisted scene. Its text
    /// lives in Element::label and moves with the note rather than a text node.
    Sticky(BoxGeom),
    /// Background section. Objects with positive overlap move with this frame.
    Frame(BoxGeom),
    Star(BoxGeom),
    Hexagon(BoxGeom),
    Line(SegGeom),
    Arrow(SegGeom),
    Text(TextGeom),
    Embed(EmbedGeom),
    Image(ImageGeom),
}

/// A page-card: a titled box anchored at `(x, y)` that links to a host page
/// (`page_id`). The crate is page-agnostic — the host supplies the id + title
/// and handles opening it; this just stores and draws the card.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbedGeom {
    pub page_id: i64,
    pub title: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// An image: a box anchored at `(x, y)` referencing a host-managed file (`src`,
/// e.g. `images/<name>`). The crate is storage-agnostic — the host imports the
/// file and supplies the decoded bitmap (see [`ImageFn`]); this stores the
/// reference + geometry and draws it as an overlay.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageGeom {
    pub src: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Rotation about the image's center, radians. Absent in older boards → 0.
    /// The host re-rotates the bitmap to match (see [`ImageFn`]).
    #[serde(default)]
    pub rotation: f32,
}

/// A text label: a top-left anchor, its content, and a world-space font size.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextGeom {
    pub x: f32,
    pub y: f32,
    pub content: String,
    pub size: f32,
    /// Rotation about the text block's center, radians. Absent in older boards → 0.
    #[serde(default)]
    pub rotation: f32,
    /// Cached world-space extent, set each render from the font layout so the
    /// selection box and hit-test fit the real glyphs. Not persisted; a zero
    /// height means unmeasured (a fallback estimate is used until then).
    #[serde(skip)]
    pub measured_w: f32,
    #[serde(skip)]
    pub measured_h: f32,
}

/// A freehand pen stroke: world-space points and a world-space width.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stroke {
    pub points: Vec<[f32; 2]>,
    pub width: f32,
}

/// A box (rectangle / ellipse), world-space. `x,y,w,h` describe the *unrotated*
/// box; `rotation` (radians, clockwise) spins it about its center at paint time.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct BoxGeom {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub width: f32,
    /// Rotation about the box center, radians. Absent in older boards → 0.
    #[serde(default)]
    pub rotation: f32,
}

impl Element {
    pub fn frame(id: u64, mut bounds: BoxGeom) -> Self {
        bounds.rotation = 0.0;
        bounds.width = 1.5;
        Self {
            id,
            kind: ElementKind::Frame(bounds),
            stroke: Some(0x7789ABFF),
            fill: Some(0x9BB2E01A),
            label: Some("フレーム".into()),
            label_color: None,
            styles: Vec::new(),
            mindmap: None,
        }
    }
    /// Default-sized sticky, centred at a world-space point. Size is independent
    /// of zoom; colours are explicit so notes stay readable in either theme.
    pub fn sticky(id: u64, center: [f32; 2]) -> Self {
        Self {
            id,
            kind: ElementKind::Sticky(BoxGeom {
                x: center[0] - 90.0,
                y: center[1] - 70.0,
                w: 180.0,
                h: 140.0,
                width: 1.0,
                rotation: 0.0,
            }),
            stroke: Some(0xC9A96BFF),
            fill: Some(0xFFE9A0FF),
            label: None,
            label_color: Some(0x2B2B2FFF),
            styles: Vec::new(),
            mindmap: None,
        }
    }
}

/// A directed segment (line / arrow), world-space.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SegmentAnchor {
    /// The connected shape/text/image element.
    pub element_id: u64,
    /// Which connector on that element: 0/1/2/3 = top/right/bottom/left.
    pub connector: usize,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SegGeom {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub width: f32,
    #[serde(default = "default_segment_style")]
    pub style: SegmentStyle,
    /// Attachment for the first endpoint. Absent in older boards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_anchor: Option<SegmentAnchor>,
    /// Attachment for the second endpoint. Absent in older boards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_anchor: Option<SegmentAnchor>,
    /// Optional world-space elbow, draggable independently of the endpoints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bend: Option<[f32; 2]>,
    /// Additional ordered manual handles. `bend` remains the first for old JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_bends: Option<RoutePath>,
    /// Manual handles represent exact polyline vertices after portion editing.
    #[serde(default, skip_serializing_if = "is_false")]
    pub manual_polyline: bool,
    /// None preserves legacy behaviour: arrows have an end head, lines no heads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heads: Option<ArrowHeads>,
    #[serde(default)]
    pub connector_type: ConnectorType,
    #[serde(default = "default_auto_route")]
    pub auto_route: bool,
    /// Derived from the board's obstacles, not persisted routing coordinates.
    #[serde(skip)]
    pub route: Option<RoutePath>,
    #[serde(skip)]
    pub route_failed: bool,
}

fn default_auto_route() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorType {
    Straight,
    #[default]
    Bent,
    Curved,
}

#[derive(Clone, Copy, Debug)]
pub struct RoutePath {
    pub points: [[f32; 2]; 32],
    pub len: usize,
}

impl RoutePath {
    pub fn new(points: &[[f32; 2]]) -> Option<Self> {
        if points.len() > 32 {
            return None;
        }
        let mut route = Self {
            points: [[0.0; 2]; 32],
            len: points.len(),
        };
        route.points[..points.len()].copy_from_slice(points);
        Some(route)
    }

    pub fn as_slice(&self) -> &[[f32; 2]] {
        &self.points[..self.len.min(32)]
    }
}

// Persist only used handles, not the fixed-capacity backing array. Derived
// automatic routes remain serde-skipped on SegGeom.
impl Serialize for RoutePath {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.as_slice().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RoutePath {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let points = Vec::<[f32; 2]>::deserialize(deserializer)?;
        Self::new(&points).ok_or_else(|| serde::de::Error::custom("at most 32 additional bends"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrowHeads {
    pub start: bool,
    pub end: bool,
}

impl SegGeom {
    pub fn between(start: [f32; 2], end: [f32; 2], width: f32) -> Self {
        Self {
            x1: start[0],
            y1: start[1],
            x2: end[0],
            y2: end[1],
            width,
            style: SegmentStyle::Solid,
            start_anchor: None,
            end_anchor: None,
            bend: None,
            extra_bends: None,
            manual_polyline: false,
            heads: None,
            connector_type: ConnectorType::Bent,
            auto_route: true,
            route: None,
            route_failed: false,
        }
    }

    pub fn arrow_heads(&self, is_arrow: bool) -> ArrowHeads {
        self.heads.unwrap_or(ArrowHeads {
            start: false,
            end: is_arrow,
        })
    }

    pub fn manual_bends(&self) -> Vec<[f32; 2]> {
        self.bend
            .into_iter()
            .chain(
                self.extra_bends
                    .iter()
                    .flat_map(|p| p.as_slice().iter().copied()),
            )
            .collect()
    }

    pub fn has_manual_bends(&self) -> bool {
        self.manual_polyline
            || self.bend.is_some()
            || self.extra_bends.is_some_and(|p| !p.as_slice().is_empty())
    }

    pub fn set_manual_bends(&mut self, bends: &[[f32; 2]]) -> bool {
        if bends.len() > 33 {
            return false;
        }
        self.bend = bends.first().copied();
        self.manual_polyline = false;
        self.extra_bends = if bends.len() > 1 {
            RoutePath::new(&bends[1..])
        } else {
            None
        };
        self.route = None;
        self.route_failed = false;
        true
    }

    pub fn set_manual_path(&mut self, path: &[[f32; 2]]) -> bool {
        let path = connector_edit::orthogonal_path(
            path.to_vec(),
            self.start_anchor.map(|a| a.connector % 2 == 1),
            self.end_anchor.map(|a| a.connector % 2 == 1),
        );
        if path.len() < 2 || !self.set_manual_bends(&path[1..path.len() - 1]) {
            return false;
        }
        self.manual_polyline = true;
        self.auto_route = false;
        self.connector_type = ConnectorType::Bent;
        true
    }

    pub fn move_endpoint(&mut self, which: usize, pos: [f32; 2]) {
        let mut path = self
            .manual_polyline
            .then(|| self.elbow_points())
            .filter(|p| p.len() >= 2);
        if self.manual_polyline && path.is_none() {
            self.manual_polyline = false;
        }
        if let Some(path) = path.as_mut() {
            let (endpoint, adjacent) = if which == 0 {
                (0, 1)
            } else {
                (path.len() - 1, path.len() - 2)
            };
            let old = path[endpoint];
            if (old[1] - path[adjacent][1]).abs() < 0.01 {
                path[adjacent][1] = pos[1];
            } else if (old[0] - path[adjacent][0]).abs() < 0.01 {
                path[adjacent][0] = pos[0];
            }
            path[endpoint] = pos;
        }
        if which == 0 {
            [self.x1, self.y1] = pos;
        } else {
            [self.x2, self.y2] = pos;
        }
        if let Some(path) = path {
            if path.len() > 2 {
                self.set_manual_path(&path);
            } else {
                self.manual_polyline = false;
            }
        }
    }

    pub fn bend_handles(&self) -> Vec<[f32; 2]> {
        let bends = self.manual_bends();
        if self.connector_type == ConnectorType::Bent && !bends.is_empty() {
            bends
        } else {
            vec![self.bend_handle()]
        }
    }

    pub fn bend_handle(&self) -> [f32; 2] {
        if self.bend.is_none() && self.connector_type == ConnectorType::Curved {
            let points = self.points();
            return points[points.len() / 2];
        }
        if self.bend.is_none() && self.route.is_some() {
            let points = self.points();
            let length: f32 = points
                .windows(2)
                .map(|pair| (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]))
                .sum();
            let mut remaining = length / 2.0;
            for pair in points.windows(2) {
                let distance = (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]);
                if distance > 0.0 && remaining <= distance {
                    let t = remaining / distance;
                    return [
                        pair[0][0] + t * (pair[1][0] - pair[0][0]),
                        pair[0][1] + t * (pair[1][1] - pair[0][1]),
                    ];
                }
                remaining -= distance;
            }
        }
        self.bend
            .unwrap_or([(self.x1 + self.x2) / 2.0, (self.y1 + self.y2) / 2.0])
    }

    pub fn points(&self) -> Vec<[f32; 2]> {
        let start = [self.x1, self.y1];
        let end = [self.x2, self.y2];
        if self.connector_type == ConnectorType::Straight {
            return vec![start, end];
        }
        if self.connector_type == ConnectorType::Curved {
            // The handle is ON the quadratic curve at t=1/2, not an invisible
            // off-path control point. This makes drag-to-reshape direct.
            let middle = self.bend.unwrap_or([
                (self.x1 + self.x2) / 2.0,
                (self.y1 + self.y2) / 2.0 - (self.x2 - self.x1).abs() * 0.08,
            ]);
            let control = [
                2.0 * middle[0] - (self.x1 + self.x2) / 2.0,
                2.0 * middle[1] - (self.y1 + self.y2) / 2.0,
            ];
            return (0..=32)
                .map(|i| {
                    let t = i as f32 / 32.0;
                    let u = 1.0 - t;
                    [
                        u * u * start[0] + 2.0 * u * t * control[0] + t * t * end[0],
                        u * u * start[1] + 2.0 * u * t * control[1] + t * t * end[1],
                    ]
                })
                .collect();
        }
        rounded_corners(self.elbow_points())
    }

    /// Sharp geometry for editing portions, independent of corner tessellation.
    pub fn elbow_points(&self) -> Vec<[f32; 2]> {
        let start = [self.x1, self.y1];
        let end = [self.x2, self.y2];
        let mut points = vec![start];
        if self.manual_polyline {
            points.extend(self.manual_bends());
            points.push(end);
            return connector_edit::orthogonal_path(
                points,
                self.start_anchor.map(|a| a.connector % 2 == 1),
                self.end_anchor.map(|a| a.connector % 2 == 1),
            );
        }
        if self.has_manual_bends() {
            let stub = |p: [f32; 2], other: [f32; 2], anchor: Option<SegmentAnchor>| {
                let direction = anchor
                    .filter(|a| a.connector < 4)
                    .map(|a| [[0.0, -1.0], [1.0, 0.0], [0.0, 1.0], [-1.0, 0.0]][a.connector])
                    .unwrap_or(if (other[0] - p[0]).abs() >= (other[1] - p[1]).abs() {
                        [if other[0] >= p[0] { 1.0 } else { -1.0 }, 0.0]
                    } else {
                        [0.0, if other[1] >= p[1] { 1.0 } else { -1.0 }]
                    });
                [p[0] + direction[0] * 24.0, p[1] + direction[1] * 24.0]
            };
            let a = stub(start, end, self.start_anchor);
            let b = stub(end, start, self.end_anchor);
            points.push(a);
            for bend in self.manual_bends() {
                let last = *points.last().unwrap();
                let previous = points[points.len() - 2];
                points.push(if last[0] != previous[0] {
                    [last[0], bend[1]]
                } else {
                    [bend[0], last[1]]
                });
                points.push(bend);
                points.dedup();
                // Keep a preceding point even for zero-length terminal stubs.
                if points.len() == 1 {
                    points.push(bend);
                }
            }
            let bend = *points.last().unwrap();
            points.push(if b[0] != end[0] {
                [b[0], bend[1]]
            } else {
                [bend[0], b[1]]
            });
            points.push(b);
        } else if let Some(route) = self.route {
            points.extend_from_slice(route.as_slice());
        } else {
            let middle_x = (self.x1 + self.x2) / 2.0;
            points.extend([[middle_x, self.y1], [middle_x, self.y2]]);
        }
        points.push([self.x2, self.y2]);
        connector_edit::orthogonal_path(
            points,
            self.start_anchor.map(|a| a.connector % 2 == 1),
            self.end_anchor.map(|a| a.connector % 2 == 1),
        )
    }
}

fn rounded_corners(points: Vec<[f32; 2]>) -> Vec<[f32; 2]> {
    let mut points = points;
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let mut result = vec![points[0]];
    for corner in points.windows(3) {
        let a = corner[0];
        let b = corner[1];
        let c = corner[2];
        let l1 = (b[0] - a[0]).hypot(b[1] - a[1]);
        let l2 = (c[0] - b[0]).hypot(c[1] - b[1]);
        let cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
        if l1 == 0.0 || l2 == 0.0 || cross.abs() < 0.001 {
            result.push(b);
            continue;
        }
        let radius = 8.0_f32.min(l1 / 2.0).min(l2 / 2.0);
        let entry = [
            b[0] - (b[0] - a[0]) * radius / l1,
            b[1] - (b[1] - a[1]) * radius / l1,
        ];
        let exit = [
            b[0] + (c[0] - b[0]) * radius / l2,
            b[1] + (c[1] - b[1]) * radius / l2,
        ];
        result.push(entry);
        for i in 1..=8 {
            let t = i as f32 / 8.0;
            let u = 1.0 - t;
            result.push([
                u * u * entry[0] + 2.0 * u * t * b[0] + t * t * exit[0],
                u * u * entry[1] + 2.0 * u * t * b[1] + t * t * exit[1],
            ]);
        }
    }
    result.push(*points.last().unwrap());
    result
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SegmentStyle {
    Solid,
    Dashed,
}

fn default_segment_style() -> SegmentStyle {
    SegmentStyle::Solid
}

/// The viewport: a world-space pan offset and a zoom factor. The offset is the
/// world point that maps to the canvas's top-left corner, so a screen point
/// `s` (relative to the canvas) is the world point `offset + s / zoom`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Camera {
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default = "one")]
    pub zoom: f32,
}

fn one() -> f32 {
    1.0
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}

impl Camera {
    /// The world point under a canvas-relative screen point.
    pub(crate) fn screen_to_world(&self, sx: f32, sy: f32) -> (f32, f32) {
        let z = self.zoom.max(MIN_ZOOM);
        (self.x + sx / z, self.y + sy / z)
    }

    /// Pan by a screen-space delta (px): the content follows the gesture.
    pub(crate) fn pan_by(&mut self, dx: f32, dy: f32) {
        let z = self.zoom.max(MIN_ZOOM);
        self.x -= dx / z;
        self.y -= dy / z;
    }

    /// Multiply the zoom by `factor`, keeping the world point under the
    /// canvas-relative screen point `(rx, ry)` fixed (zoom-about-cursor).
    pub(crate) fn zoom_about(&mut self, rx: f32, ry: f32, factor: f32) {
        let z = self.zoom;
        let z2 = (z * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        if (z2 - z).abs() < f32::EPSILON {
            return;
        }
        self.x += rx * (1.0 / z - 1.0 / z2);
        self.y += ry * (1.0 / z - 1.0 / z2);
        self.zoom = z2;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalThumbnailMode {
    Auto,
    Selection,
    Viewport,
    AllContent,
    Element(u64),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct LocalThumbnailSpec {
    pub anchor_element_id: Option<u64>,
    /// Focus bounds in world coordinates: `[x0, y0, x1, y1]`.
    pub focus_bounds: [f32; 4],
    /// Whole-scene bounds in world coordinates, when available.
    pub scene_bounds: Option<[f32; 4]>,
    /// Recommended camera to render this thumbnail into the requested size.
    pub camera: Camera,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalThumbnailSnapshot {
    pub scene: Scene,
    pub spec: LocalThumbnailSpec,
}

impl LocalThumbnailSnapshot {
    /// Build a chrome-free all-content thumbnail directly from a persisted scene.
    /// Hosts can use this without mounting a full [`WhiteboardView`] entity first.
    pub fn for_scene_all_content(scene: Scene, width_px: f32, height_px: f32) -> Self {
        let width_px = width_px.max(1.0);
        let height_px = height_px.max(1.0);
        let scene_bounds = scene_bbox_for_local_thumbnail(&scene);
        let spec = if let Some(focus) = scene_bounds {
            local_thumbnail_spec_from_bbox(None, focus, scene_bounds, width_px, height_px)
        } else {
            let zoom = scene.camera.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
            LocalThumbnailSpec {
                anchor_element_id: None,
                focus_bounds: [
                    scene.camera.x,
                    scene.camera.y,
                    scene.camera.x + width_px / zoom,
                    scene.camera.y + height_px / zoom,
                ],
                scene_bounds: None,
                camera: Camera {
                    x: scene.camera.x,
                    y: scene.camera.y,
                    zoom,
                },
            }
        };
        Self { scene, spec }
    }
}

pub(crate) fn scene_bbox_for_local_thumbnail(scene: &Scene) -> Option<(f32, f32, f32, f32)> {
    let mut bounds = scene.elements.iter().map(|element| bbox(&element.kind));
    let first = bounds.next()?;
    Some(bounds.fold(first, |current, next| {
        (
            current.0.min(next.0),
            current.1.min(next.1),
            current.2.max(next.2),
            current.3.max(next.3),
        )
    }))
}

pub(crate) fn local_thumbnail_spec_from_bbox(
    anchor_element_id: Option<u64>,
    focus: (f32, f32, f32, f32),
    scene_bounds: Option<(f32, f32, f32, f32)>,
    width_px: f32,
    height_px: f32,
) -> LocalThumbnailSpec {
    let width_px = width_px.max(1.0);
    let height_px = height_px.max(1.0);
    let focus_w = (focus.2 - focus.0).abs().max(1.0);
    let focus_h = (focus.3 - focus.1).abs().max(1.0);
    let pad_x = (focus_w * 0.18).max(48.0);
    let pad_y = (focus_h * 0.18).max(36.0);
    let padded_w = (focus_w + pad_x * 2.0).max(1.0);
    let padded_h = (focus_h + pad_y * 2.0).max(1.0);
    let zoom = (width_px / padded_w)
        .min(height_px / padded_h)
        .clamp(MIN_ZOOM, MAX_ZOOM);
    let center_x = (focus.0 + focus.2) * 0.5;
    let center_y = (focus.1 + focus.3) * 0.5;
    LocalThumbnailSpec {
        anchor_element_id,
        focus_bounds: [focus.0, focus.1, focus.2, focus.3],
        scene_bounds: scene_bounds.map(|bounds| [bounds.0, bounds.1, bounds.2, bounds.3]),
        camera: Camera {
            x: center_x - width_px / (2.0 * zoom),
            y: center_y - height_px / (2.0 * zoom),
            zoom,
        },
    }
}
