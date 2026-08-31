// Renders an expanded protocol::Node tree to a PNG.
//
// The polyfill lays the scene out *itself* and rasterizes the result with
// resvg — it does not drive a general-purpose CSS/text-shaping engine. This is
// possible because every TWP node has a grid-determined (mono, box, flex) or
// explicitly-sized (svg, img) footprint, so no recursive text measurement is
// needed:
//
//   * `mono` / `text` — each glyph occupies a fixed cell block, so positions
//     are pure arithmetic (`col * cell_w`), with no font-advance measurement.
//   * `flex` / `box` / `stack` — sized by explicit width/height/gap/padding/
//     flex-grow against a known parent (taffy flexbox).
//   * `svg` / `img` — explicit width/height.
//
// Layout is computed with taffy (all-definite leaf sizes, no measure
// callbacks), the tree is emitted as an SVG with absolutely-positioned
// elements, and resvg rasterizes it to PNG. This is the same picture the proxy
// transmits via the Kitty graphics protocol.
//
// Because the polyfill paints borders as SVG edge strokes *after* layout, a
// border never displaces a node's content or siblings — the grid-stability
// invariant the spec (§7) describes.
//
// Proportional `text` (wrapping prose in a sans-serif face) is deferred: a
// `text` node renders as a monospace run in the polyfill. See §13 of the spec.

use std::collections::HashMap;
use std::sync::OnceLock;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use taffy::geometry::Size;
use taffy::style::AvailableSpace;
use taffy::style::Dimension;
use taffy::style::LengthPercentage;
use taffy::style::LengthPercentageAuto;
use taffy::style::Position;
use taffy::{AlignItems, Display, FlexDirection, JustifyContent, NodeId, TaffyTree};

use crate::protocol::{Border, Dimension as TDimension, FontWeight, Img, Node};

/// Fallback render resolution per cell when the terminal doesn't report
/// pixel dimensions via TIOCGWINSZ.
const DEFAULT_PX_PER_COL: u32 = 20;
const DEFAULT_PX_PER_ROW: u32 = 40;

/// Actual cell pixel dimensions, set once at startup from TIOCGWINSZ.
/// If the terminal doesn't populate ws_xpixel/ws_ypixel, falls back to
/// the defaults above.
static CELL_PX: OnceLock<(u32, u32)> = OnceLock::new();

pub fn set_cell_pixels(px_per_col: u32, px_per_row: u32) {
    let _ = CELL_PX.set((px_per_col, px_per_row));
}

pub(crate) fn px_per_col() -> u32 {
    CELL_PX.get().map(|c| c.0).unwrap_or(DEFAULT_PX_PER_COL)
}

pub(crate) fn px_per_row() -> u32 {
    CELL_PX.get().map(|c| c.1).unwrap_or(DEFAULT_PX_PER_ROW)
}

// ── Terminal color palette ─────────────────────────────────────────
//
// Widgets can reference the user's terminal theme with `term(fg)`,
// `term(bg)`, and `term(0..255)` — resolved against the palette below.
// The proxy queries the real terminal (OSC 4 / 10 / 11) at startup and
// installs it via `set_palette`; absent that (library use, tests), a
// standard xterm default is used.

/// The 16 ANSI colors + default fg/bg + the computed 256-color palette.
#[derive(Clone, Copy)]
pub struct Palette {
    pub fg: [u8; 3],
    pub bg: [u8; 3],
    pub ansi: [[u8; 3]; 256],
}

static PALETTE: OnceLock<Palette> = OnceLock::new();

pub fn set_palette(p: Palette) {
    let _ = PALETTE.set(p);
}

fn palette() -> Palette {
    PALETTE.get().copied().unwrap_or_else(default_palette)
}

/// Build a full 256-entry palette from the 16 base colors: indices 16–231
/// are the standard 6×6×6 cube and 232–255 the grayscale ramp — both fixed
/// formulas, so only 0–15 (+ fg/bg) are ever theme-dependent.
pub fn palette_from_base(base16: [[u8; 3]; 16], fg: [u8; 3], bg: [u8; 3]) -> Palette {
    let mut ansi = [[0u8; 3]; 256];
    ansi[..16].copy_from_slice(&base16);
    let cube = |c: usize| -> u8 { if c == 0 { 0 } else { (55 + c * 40) as u8 } };
    for (n, slot) in ansi[16..232].iter_mut().enumerate() {
        *slot = [cube((n / 36) % 6), cube((n / 6) % 6), cube(n % 6)];
    }
    for (n, slot) in ansi[232..256].iter_mut().enumerate() {
        let v = (8 + n * 10) as u8;
        *slot = [v, v, v];
    }
    Palette { fg, bg, ansi }
}

pub fn default_palette() -> Palette {
    // Standard xterm 16-color defaults.
    let base: [[u8; 3]; 16] = [
        [0, 0, 0],
        [205, 0, 0],
        [0, 205, 0],
        [205, 205, 0],
        [0, 0, 238],
        [205, 0, 205],
        [0, 205, 205],
        [229, 229, 229],
        [127, 127, 127],
        [255, 0, 0],
        [0, 255, 0],
        [255, 255, 0],
        [92, 92, 255],
        [255, 0, 255],
        [0, 255, 255],
        [255, 255, 255],
    ];
    palette_from_base(base, [229, 229, 229], [0, 0, 0])
}

/// Resolve a `term(...)` token to an RGB triple, or None if it isn't one.
pub(crate) fn resolve_term(s: &str) -> Option<[u8; 3]> {
    let inner = s.trim().strip_prefix("term(")?.strip_suffix(')')?.trim();
    let pal = palette();
    match inner {
        "fg" => Some(pal.fg),
        "bg" => Some(pal.bg),
        _ => inner
            .parse::<usize>()
            .ok()
            .filter(|&i| i < 256)
            .map(|i| pal.ansi[i]),
    }
}

/// Replace every `term(...)` token in a string with its `#rrggbb` equivalent.
/// The token is self-delimiting, so this is a safe substitution in any
/// color-bearing value without parsing the surrounding CSS/SVG.
pub(crate) fn substitute_term(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(pos) = rest.find("term(") {
        out.push_str(&rest[..pos]);
        let after = &rest[pos..];
        match after.find(')') {
            Some(end) => {
                let token = &after[..=end];
                match resolve_term(token) {
                    Some([r, g, b]) => out.push_str(&format!("#{r:02x}{g:02x}{b:02x}")),
                    None => out.push_str(token),
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(after);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// An RGBA color in 0..=255 space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    fn opaque(r: u8, g: u8, b: u8) -> Self {
        Rgba { r, g, b, a: 255 }
    }
    fn to_css(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!(
                "rgba({},{},{},{:.3})",
                self.r,
                self.g,
                self.b,
                self.a as f32 / 255.0
            )
        }
    }
}

/// Resolve a color value (hex, `term(...)`, `transparent`, `currentColor`,
/// `color-mix(...)`) to an RGBA. Returns None for anything unrecognized, so
/// the property is dropped rather than crashing.
fn resolve_color(s: &str) -> Option<Rgba> {
    let s = substitute_term(s.trim());
    if s.eq_ignore_ascii_case("transparent") {
        return Some(Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        });
    }
    if s.eq_ignore_ascii_case("currentcolor") {
        return None; // only meaningful inside SVG markup, handled there
    }
    if let Some(hex) = s.strip_prefix('#') {
        let (r, g, b, a) = match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                (r * 17, g * 17, b * 17, 255)
            }
            6 => (
                u8::from_str_radix(&hex[0..2], 16).ok()?,
                u8::from_str_radix(&hex[2..4], 16).ok()?,
                u8::from_str_radix(&hex[4..6], 16).ok()?,
                255,
            ),
            8 => (
                u8::from_str_radix(&hex[0..2], 16).ok()?,
                u8::from_str_radix(&hex[2..4], 16).ok()?,
                u8::from_str_radix(&hex[4..6], 16).ok()?,
                u8::from_str_radix(&hex[6..8], 16).ok()?,
            ),
            _ => return None,
        };
        return Some(Rgba { r, g, b, a });
    }
    resolve_color_mix(&s)
}

/// Parse and resolve `color-mix(in srgb, C1 p1%, C2 p2%)`. Percentages that
/// are omitted are filled so the pair sums to 100% (both omitted ⇒ 50/50).
fn resolve_color_mix(s: &str) -> Option<Rgba> {
    let inner = s
        .strip_prefix("color-mix(in srgb,")?
        .strip_suffix(')')?
        .trim();
    let mut parts = inner.split(',').map(|p| p.trim()).filter(|p| !p.is_empty());
    let a = parts.next()?.trim().to_string();
    let b = parts.next()?.trim().to_string();
    let (c1, p1) = split_pct(&a);
    let (c2, p2) = split_pct(&b);
    let c1 = resolve_color(&c1)?;
    let c2 = resolve_color(&c2)?;
    let (p1, p2) = match (p1, p2) {
        (Some(x), Some(y)) => (x, y),
        (Some(x), None) => (x, 100.0 - x),
        (None, Some(y)) => (100.0 - y, y),
        (None, None) => (50.0, 50.0),
    };
    let f1 = (p1 / 100.0).clamp(0.0, 1.0);
    let f2 = (p2 / 100.0).clamp(0.0, 1.0);
    let total = f1 + f2;
    if total <= 0.0 {
        return Some(c2);
    }
    // Interpolate in sRGB, weighting by the declared percentages (renormalized
    // to sum to 1).
    let w1 = f1 / total;
    let w2 = f2 / total;
    let lerp = |a: u8, b: u8| (a as f32 * w1 + b as f32 * w2).round() as u8;
    // Alpha: CSS color-mix composites over the result; a simple weighted
    // average is close enough for a polyfill.
    Some(Rgba {
        r: lerp(c1.r, c2.r),
        g: lerp(c1.g, c2.g),
        b: lerp(c1.b, c2.b),
        a: (c1.a as f32 * w1 + c2.a as f32 * w2).round() as u8,
    })
}

/// Split a `color-mix` argument into (color, optional percentage).
fn split_pct(s: &str) -> (String, Option<f32>) {
    let s = s.trim();
    if let Some(rest) = s.strip_suffix('%')
        && let Some(idx) = rest.rfind(' ')
    {
        let color = rest[..idx].trim().to_string();
        let pct = rest[idx + 1..].trim().parse().ok();
        return (color, pct);
    }
    (s.to_string(), None)
}

/// Final-fallback font paths if every other discovery mechanism fails.
const FONT_PROBE: &[&str] = &[
    "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
    "/usr/share/fonts/adobe-source-code-pro/SourceCodePro-Regular.otf",
    "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
    "/usr/share/fonts/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/System/Library/Fonts/Menlo.ttc",
    "/System/Library/Fonts/Monaco.ttf",
];

#[derive(Debug, Default)]
struct FontSet {
    regular: Option<std::path::PathBuf>,
}

/// Best-effort monospace font discovery (independent of any rendering engine):
///   1. `TWP_FONT_PATH` env override.
///   2. Kitty config `font_family`.
///   3. `fc-match monospace`.
///   4. `FONT_PROBE` paths.
fn discover_fonts() -> FontSet {
    if let Ok(p) = std::env::var("TWP_FONT_PATH") {
        return FontSet {
            regular: Some(std::path::PathBuf::from(p)),
        };
    }
    if let Some(reg) = std::env::var("TWP_FONT_FAMILY")
        .ok()
        .or_else(read_kitty_family)
    {
        return FontSet {
            regular: fc_match(&reg),
        };
    }
    if let Some(path) = fc_match("monospace") {
        return FontSet {
            regular: Some(path),
        };
    }
    for p in FONT_PROBE {
        if std::path::Path::new(p).exists() {
            return FontSet {
                regular: Some(std::path::PathBuf::from(p)),
            };
        }
    }
    FontSet::default()
}

fn read_kitty_family() -> Option<String> {
    let cfg_dir = std::env::var("XDG_CONFIG_HOME")
        .or_else(|_| std::env::var("HOME").map(|h| format!("{h}/.config")))
        .ok()?;
    let path = std::path::Path::new(&cfg_dir).join("kitty/kitty.conf");
    let contents = std::fs::read_to_string(path).ok()?;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("font_family") {
            return Some(extract_family(rest.trim()));
        }
    }
    None
}

/// Kitty supports both legacy (`font_family Inter`) and modern
/// (`font_family family="Inter" features=...`) syntax. Handle both.
fn extract_family(rest: &str) -> String {
    if let Some(start) = rest.find("family=\"") {
        let after = &rest[start + "family=\"".len()..];
        if let Some(end) = after.find('"') {
            return after[..end].to_string();
        }
    }
    rest.to_string()
}

fn fc_match(family: &str) -> Option<std::path::PathBuf> {
    let output = std::process::Command::new("fc-match")
        .args(["-f", "%{file}", family])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8(output.stdout).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(std::path::PathBuf::from(trimmed))
}

fn fonts() -> &'static FontSet {
    static FS: OnceLock<FontSet> = OnceLock::new();
    FS.get_or_init(discover_fonts)
}

/// Build the resvg/usvg options: a font database populated with system fonts
/// plus the discovered monospace family (so `mono`/`text` glyphs render in a
/// real monospace face), with that family as the default.
fn usvg_options() -> usvg::Options<'static> {
    let mut opt = usvg::Options::default();
    let db = opt.fontdb_mut();
    db.load_system_fonts();
    let family = fonts()
        .regular
        .as_ref()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|bytes| {
            db.load_font_data(bytes);
            // The just-loaded font is the last face; ask fontdb for its family.
            db.faces()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .find(|f| !f.families.is_empty())
                .and_then(|f| f.families.first().map(|(n, _)| n.clone()))
        });
    if let Some(family) = family {
        opt.font_family = family;
    } else if fonts().regular.is_none() {
        eprintln!(
            "twp-proxy: no monospace font found via Kitty config, fc-match, or probe paths; \
             text will render with resvg's default face"
        );
    }
    opt
}

// ── Layout ─────────────────────────────────────────────────────────

/// Node kinds the layout/emit passes care about.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// `flex`, `box`, `stack` — layout containers / styled rects.
    Container,
    /// `mono`, `text` — grid text, emitted per-glyph (arithmetic positions).
    Text,
    /// `svg` — inline vector markup embedded as an SVG image.
    Svg,
    /// `img` — bitmap embedded as an image.
    Img,
}

/// Paint-time info for a laid-out node, keyed by its taffy NodeId.
#[derive(Clone)]
struct Info {
    kind: Kind,
    text: String,
    color: Rgba,
    bold: bool,
    font_size: f32,
    cell_w: f32,
    cell_h: f32,
    background: Option<Rgba>,
    radius: f32,
    opacity: f32,
    border: Option<Border>,
    img: Option<Img>,
    svg_markup: String,
    /// The node's untyped style keys (the `extra` map), for the small set of
    /// passthrough effects (e.g. `box-shadow`).
    extra: HashMap<String, serde_json::Value>,
}

impl Info {
    fn container() -> Self {
        Info {
            kind: Kind::Container,
            text: String::new(),
            color: Rgba::opaque(255, 255, 255),
            bold: false,
            font_size: 0.0,
            cell_w: 0.0,
            cell_h: 0.0,
            background: None,
            radius: 0.0,
            opacity: 1.0,
            border: None,
            img: None,
            svg_markup: String::new(),
            extra: HashMap::new(),
        }
    }
}

/// Convert a protocol Dimension to a taffy LengthPercentage (px or percent).
fn to_length_percentage(d: TDimension) -> LengthPercentage {
    match d {
        TDimension::Px(v) => LengthPercentage::length(v),
        TDimension::Percent(v) => LengthPercentage::percent(v / 100.0),
        TDimension::ColWidth(v) => LengthPercentage::length(v * px_per_col() as f32),
        TDimension::RowHeight(v) => LengthPercentage::length(v * px_per_row() as f32),
    }
}

/// Resolve a protocol Dimension to a plain pixel length (percent resolves
/// against `base`). Cell units resolve against the live cell size.
fn to_px(d: TDimension, base: f32) -> f32 {
    match d {
        TDimension::Px(v) => v,
        TDimension::Percent(v) => base * v / 100.0,
        TDimension::ColWidth(v) => v * px_per_col() as f32,
        TDimension::RowHeight(v) => v * px_per_row() as f32,
    }
}

/// Build the taffy layout tree for `scene`, sized to `canvas_w` × `canvas_h`,
/// returning the tree, the root id, and the paint-info map.
fn build_layout(
    scene: &Node,
    canvas_w: f32,
    canvas_h: f32,
) -> (TaffyTree, NodeId, HashMap<NodeId, Info>) {
    let mut tree = TaffyTree::new();
    let mut info = HashMap::new();
    let root = add(&mut tree, &mut info, scene, canvas_w, canvas_h, true, false);
    (tree, root, info)
}

fn kind_of(n: &str) -> Kind {
    match n {
        "mono" | "text" => Kind::Text,
        "svg" => Kind::Svg,
        "img" => Kind::Img,
        _ => Kind::Container,
    }
}

fn add(
    tree: &mut TaffyTree,
    info: &mut HashMap<NodeId, Info>,
    node: &Node,
    avail_w: f32,
    avail_h: f32,
    is_root: bool,
    absolute: bool,
) -> NodeId {
    let kind = kind_of(&node.n);
    let s = &node.s;

    let mut ts = taffy::style::Style::default();

    // Stack children are absolutely-positioned layers anchored top-left,
    // filling the stack box (their own width/height styles still apply).
    if absolute {
        ts.position = Position::Absolute;
        ts.inset.left = LengthPercentageAuto::length(0.0);
        ts.inset.top = LengthPercentageAuto::length(0.0);
    }

    // Flex layout for `flex` containers.
    if node.n == "flex" {
        ts.display = Display::Flex;
        ts.flex_direction = match s.flex_direction.as_deref() {
            Some("column") => FlexDirection::Column,
            _ => FlexDirection::Row,
        };
        ts.justify_content = match s.justify_content.as_deref() {
            Some("center") => Some(JustifyContent::Center),
            Some("space-between") => Some(JustifyContent::SpaceBetween),
            Some("space-around") => Some(JustifyContent::SpaceAround),
            Some("space-evenly") => Some(JustifyContent::SpaceEvenly),
            Some("flex-end") | Some("end") => Some(JustifyContent::FlexEnd),
            _ => Some(JustifyContent::FlexStart),
        };
        ts.align_items = match s.align_items.as_deref() {
            Some("center") => Some(AlignItems::Center),
            Some("flex-end") | Some("end") => Some(AlignItems::FlexEnd),
            Some("stretch") => Some(AlignItems::Stretch),
            _ => Some(AlignItems::FlexStart),
        };
    }

    // Stack: children are absolutely-positioned layers filling the stack box.
    let is_stack = node.n == "stack";
    if is_stack {
        ts.position = Position::Relative;
    } // Gap (flex only).
    if let Some(g) = s.gap {
        let v = to_length_percentage(g);
        ts.gap = Size {
            width: v,
            height: v,
        };
    }

    // Padding: shorthand and longhands.
    let pad = |d: Option<TDimension>| -> LengthPercentage {
        d.map(to_length_percentage)
            .unwrap_or(LengthPercentage::length(0.0))
    };
    match (
        s.padding,
        s.padding_top,
        s.padding_right,
        s.padding_bottom,
        s.padding_left,
    ) {
        (Some(p), None, None, None, None) => {
            let v = to_length_percentage(p);
            ts.padding = taffy::geometry::Rect {
                left: v,
                right: v,
                top: v,
                bottom: v,
            };
        }
        _ => {
            ts.padding = taffy::geometry::Rect {
                top: pad(s.padding_top),
                right: pad(s.padding_right),
                bottom: pad(s.padding_bottom),
                left: pad(s.padding_left),
            };
            // A plain shorthand is overridden by longhands where present.
            if let Some(p) = s.padding {
                let v = to_length_percentage(p);
                let cur = ts.padding;
                ts.padding = taffy::geometry::Rect {
                    top: if s.padding_top.is_some() { cur.top } else { v },
                    right: if s.padding_right.is_some() {
                        cur.right
                    } else {
                        v
                    },
                    bottom: if s.padding_bottom.is_some() {
                        cur.bottom
                    } else {
                        v
                    },
                    left: if s.padding_left.is_some() {
                        cur.left
                    } else {
                        v
                    },
                };
            }
        }
    }

    if let Some(g) = s.flex_grow {
        ts.flex_grow = g;
    }
    if let Some(mw) = s.max_width {
        ts.max_size.width = Dimension::from(to_length_percentage(mw));
    }

    // Sizing.
    let grow = s.flex_grow.map(|g| g > 0.0).unwrap_or(false);
    if is_stack {
        ts.size.width = s
            .width
            .map(|d| Dimension::from(to_length_percentage(d)))
            .unwrap_or_else(|| {
                if is_root {
                    Dimension::length(avail_w)
                } else {
                    Dimension::auto()
                }
            });
        ts.size.height = s
            .height
            .map(|d| Dimension::from(to_length_percentage(d)))
            .unwrap_or_else(|| {
                if is_root {
                    Dimension::length(avail_h)
                } else {
                    Dimension::auto()
                }
            });
    } else {
        match kind {
            Kind::Text => {
                let scale = s.scale.unwrap_or(1).max(1);
                let char_w = s.char_width.unwrap_or(scale).max(1);
                let n = node.t.as_deref().unwrap_or("").chars().count() as f32;
                ts.size.width = Dimension::length(n * px_per_col() as f32 * char_w as f32);
                ts.size.height = Dimension::length(px_per_row() as f32 * scale as f32);
            }
            _ => {
                ts.size.width = match &s.width {
                    Some(d) => Dimension::from(to_length_percentage(*d)),
                    None if is_root => Dimension::length(avail_w),
                    None if grow => Dimension::auto(),
                    None => Dimension::auto(),
                };
                ts.size.height = match &s.height {
                    Some(d) => Dimension::from(to_length_percentage(*d)),
                    None if is_root => Dimension::length(avail_h),
                    None => Dimension::auto(),
                };
            }
        }
    }

    let id = tree.new_leaf(ts).unwrap();

    // Children.
    if is_stack {
        for child in &node.c {
            let cid = add(tree, info, child, avail_w, avail_h, false, true);
            tree.add_child(id, cid).unwrap();
        }
    } else {
        for child in &node.c {
            let cid = add(tree, info, child, avail_w, avail_h, false, false);
            tree.add_child(id, cid).unwrap();
        }
    }

    // Paint info.
    let mut inf = Info::container();
    inf.kind = kind;
    inf.text = node.t.clone().unwrap_or_default();
    let scale = s.scale.unwrap_or(1).max(1);
    let char_w = s.char_width.unwrap_or(scale).max(1);
    inf.bold = matches!(&s.font_weight, Some(FontWeight::Name(n)) if n == "bold")
        || matches!(&s.font_weight, Some(FontWeight::Number(n)) if *n >= 700);
    inf.font_size = px_per_row() as f32 * scale as f32 * s.subscale_n.unwrap_or(1) as f32
        / s.subscale_d.filter(|&d| d > 0).unwrap_or(1) as f32
        * 0.75;
    inf.cell_w = px_per_col() as f32 * char_w as f32;
    inf.cell_h = px_per_row() as f32 * scale as f32;
    if let Some(rgba) = s.color.as_deref().and_then(resolve_color) {
        inf.color = rgba;
    }
    if let Some(bg) = &s.background {
        inf.background = resolve_color(bg);
    }
    inf.radius = s
        .border_radius
        .map(|d| to_px(d, avail_w))
        .unwrap_or(0.0)
        .max(0.0);
    inf.opacity = s.opacity.unwrap_or(1.0).clamp(0.0, 1.0);
    inf.border = s.border.clone();
    inf.img = node.img.clone();
    if kind == Kind::Svg {
        inf.svg_markup = substitute_term(node.t.as_deref().unwrap_or(""));
    }
    inf.extra = s.extra.clone();
    info.insert(id, inf);
    id
}

// ── SVG emission ───────────────────────────────────────────────────

struct EmitCtx {
    info: HashMap<NodeId, Info>,
    blur_ids: usize,
}

impl EmitCtx {
    fn new(info: HashMap<NodeId, Info>) -> Self {
        EmitCtx { info, blur_ids: 0 }
    }
    fn blur_id(&mut self) -> String {
        self.blur_ids += 1;
        format!("twp-blur-{}", self.blur_ids)
    }
}

fn f(x: f32) -> String {
    let r = (x * 100.0).round() / 100.0;
    if r.fract() == 0.0 {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

/// Render `scene` to an SVG document string sized `img_w` × `img_h`.
fn scene_to_svg(scene: &Node, img_w: u32, img_h: u32) -> String {
    let (tree, root, info) = build_layout(scene, img_w as f32, img_h as f32);
    let avail = Size {
        width: AvailableSpace::Definite(img_w as f32),
        height: AvailableSpace::Definite(img_h as f32),
    };
    let mut tree = tree;
    tree.compute_layout(root, avail)
        .expect("taffy layout should succeed");

    let mut out = String::with_capacity(8 * 1024);
    out.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{img_w}" height="{img_h}" viewBox="0 0 {img_w} {img_h}">"#
    ));
    let mut defs = String::new();
    let mut body = String::new();
    let mut ctx = EmitCtx::new(info);
    emit(&tree, &mut ctx, &mut defs, &mut body, root, 0.0, 0.0);
    if !defs.is_empty() {
        out.push_str(&format!("<defs>{defs}</defs>"));
    }
    out.push_str(&body);
    out.push_str("</svg>");
    out
}

fn emit(
    tree: &TaffyTree,
    ctx: &mut EmitCtx,
    defs: &mut String,
    body: &mut String,
    id: NodeId,
    ox: f32,
    oy: f32,
) {
    let layout = tree.layout(id).unwrap();
    let (x, y, w, h) = (
        ox + layout.location.x,
        oy + layout.location.y,
        layout.size.width,
        layout.size.height,
    );
    let inf = ctx.info[&id].clone();

    // box-shadow (passthrough effect) — a blurred rect behind the node.
    if let Some(shadow) = parse_box_shadow(&inf) {
        let (sx, sy, blur, color) = shadow;
        let bid = ctx.blur_id();
        defs.push_str(&format!(
            r#"<filter id="{bid}" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="{blur}"/></filter>"#
        ));
        body.push_str(&format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{}" filter="url(#{bid})"/>"#,
            f(x + sx),
            f(y + sy),
            f(w),
            f(h),
            f(inf.radius),
            color
        ));
    }

    match inf.kind {
        Kind::Text => {
            // A text node paints its declared background across its box (so a
            // highlighted run reads as a filled cell-span), then the glyphs.
            if let Some(bg) = inf.background {
                body.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{}" fill-opacity="{:.3}"/>"#,
                    f(x),
                    f(y),
                    f(w),
                    f(h),
                    f(inf.radius),
                    bg.to_css(),
                    inf.opacity
                ));
            }
            emit_text(body, x, y, &inf);
        }
        Kind::Svg => emit_svg(defs, body, x, y, w, h, &inf),
        Kind::Img => emit_img(defs, body, x, y, w, h, &inf),
        Kind::Container => {
            if let Some(bg) = inf.background {
                let fill = bg.to_css();
                body.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{}" fill-opacity="{:.3}"/>"#,
                    f(x),
                    f(y),
                    f(w),
                    f(h),
                    f(inf.radius),
                    fill,
                    inf.opacity
                ));
            }
            // Children paint on top of the background.
            for child in tree.children(id).unwrap() {
                emit(tree, ctx, defs, body, child, x, y);
            }
            // Border is painted *after* content — a non-displacing edge stroke.
            if let Some((color, width)) = inf
                .border
                .as_ref()
                .and_then(|b| resolve_color(&b.color).map(|c| (c, b.width)))
            {
                body.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="none" stroke="{}" stroke-width="{}"/>"#,
                    f(x),
                    f(y),
                    f(w),
                    f(h),
                    f(inf.radius),
                    color.to_css(),
                    f(width)
                ));
            }
        }
    }
}

/// Emit a mono/text node: one `<text>` per glyph, positioned arithmetically
/// (`x = col * cell_w`) so no font-advance measurement is needed and there is
/// no cumulative drift.
fn emit_text(body: &mut String, x: f32, y: f32, inf: &Info) {
    let bold = if inf.bold {
        " font-weight=\"bold\""
    } else {
        ""
    };
    let fill = inf.color.to_css();
    for (i, ch) in inf.text.chars().enumerate() {
        if ch == ' ' {
            continue; // spaces produce no ink
        }
        let cx = x + i as f32 * inf.cell_w;
        let cy = y + inf.font_size; // baseline
        let escaped = match ch {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            c => c.to_string(),
        };
        body.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="{}" fill="{}" fill-opacity="{:.3}"{}>{}</text>"#,
            f(cx),
            f(cy),
            f(inf.font_size),
            fill,
            inf.opacity,
            bold,
            escaped
        ));
    }
}

/// Emit an `svg` node: the inline markup, with `term(...)` already resolved and
/// `currentColor` bound to the node's `color`, embedded as an SVG image scaled
/// to the node's box (clipped to `border-radius`).
fn emit_svg(defs: &mut String, body: &mut String, x: f32, y: f32, w: f32, h: f32, inf: &Info) {
    let mut markup = inf.svg_markup.clone();
    // Bind `currentColor` to the node's `color` so vector art can inherit a
    // (possibly theme-derived) color.
    markup = markup.replace("currentColor", &inf.color.to_css());
    let data = STANDARD.encode(markup.as_bytes());
    let clip = clip_attr(defs, x, y, w, h, inf);
    body.push_str(&format!(
        r#"<image x="{}" y="{}" width="{}" height="{}" preserveAspectRatio="none"{} href="data:image/svg+xml;base64,{}"/>"#,
        f(x),
        f(y),
        f(w),
        f(h),
        clip,
        data
    ));
}

/// Emit an `img` node: decode the source and embed it as a raster image data
/// URI, clipped to `border-radius`.
fn emit_img(defs: &mut String, body: &mut String, x: f32, y: f32, w: f32, h: f32, inf: &Info) {
    let Some(img) = &inf.img else { return };
    let Some(data) = img_data_uri(img) else {
        eprintln!("twp-proxy: img node has no resolvable source");
        return;
    };
    let clip = clip_attr(defs, x, y, w, h, inf);
    body.push_str(&format!(
        r#"<image x="{}" y="{}" width="{}" height="{}" preserveAspectRatio="xMidYMid slice"{} href="{}"/>"#,
        f(x),
        f(y),
        f(w),
        f(h),
        clip,
        data
    ));
}

/// If the node has a `border-radius`, emit a `<clipPath>` rounded rect in
/// `defs` and return the `clip-path` attribute referencing it.
fn clip_attr(defs: &mut String, x: f32, y: f32, w: f32, h: f32, inf: &Info) -> String {
    if inf.radius <= 0.0 {
        return String::new();
    }
    // A stable id from the node's top-left corner.
    let id = format!("twp-clip-{}-{}", (x * 100.0) as u32, (y * 100.0) as u32);
    defs.push_str(&format!(
        r#"<clipPath id="{id}"><rect x="{}" y="{}" width="{}" height="{}" rx="{}"/></clipPath>"#,
        f(x),
        f(y),
        f(w),
        f(h),
        f(inf.radius)
    ));
    format!(r#" clip-path="url(#{id})""#)
}

/// Decode an `img` node's source into a `data:` URI string. Encoded formats
/// (PNG/JPEG) are passed through; raw RGBA/RGB are encoded to PNG first.
fn img_data_uri(img: &Img) -> Option<String> {
    let medium = img
        .transmission
        .as_deref()
        .unwrap_or(if img.data.is_some() { "d" } else { "f" });
    let bytes = match medium {
        "d" => STANDARD.decode(img.data.as_deref()?.trim()).ok()?,
        "f" => std::fs::read(img.path.as_deref()?).ok()?,
        _ => return None,
    };
    let format = img.format.unwrap_or(100);
    match format {
        32 => {
            let (w, h) = (img.data_width?, img.data_height?);
            let rgba = image::RgbaImage::from_raw(w, h, bytes)?;
            let mut buf = Vec::new();
            rgba.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                .ok()?;
            let b64 = STANDARD.encode(&buf);
            Some(format!("data:image/png;base64,{b64}"))
        }
        24 => {
            let (w, h) = (img.data_width?, img.data_height?);
            let mut rgba = Vec::with_capacity(bytes.len() / 3 * 4);
            for px in bytes.chunks_exact(3) {
                rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
            let img = image::RgbaImage::from_raw(w, h, rgba)?;
            let mut buf = Vec::new();
            img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                .ok()?;
            let b64 = STANDARD.encode(&buf);
            Some(format!("data:image/png;base64,{b64}"))
        }
        _ => {
            let mime = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
                "image/png"
            } else if bytes.starts_with(&[0xFF, 0xD8]) {
                "image/jpeg"
            } else {
                "image/png"
            };
            let b64 = STANDARD.encode(&bytes);
            Some(format!("data:{mime};base64,{b64}"))
        }
    }
}

/// Parse a `box-shadow` passthrough value (`h v blur color`) into the offset,
/// blur, and resolved color. Returns None if unparseable.
fn parse_box_shadow(inf: &Info) -> Option<(f32, f32, f32, String)> {
    let value = inf.extra.get("box-shadow")?.as_str()?;
    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }
    let h: f32 = parts[0].trim_end_matches("px").parse().ok()?;
    let v: f32 = parts[1].trim_end_matches("px").parse().ok()?;
    let blur: f32 = parts[2].trim_end_matches("px").parse().ok()?;
    let color = resolve_color(&parts[3..].join(" "))?.to_css();
    Some((h, v, blur, color))
}

/// Render `scene` to a PNG at `cols` × `rows` cells, sized to the live cell
/// pixels. This is the same image the proxy transmits via Kitty.
pub fn render_to_png(scene: &Node, cols: u32, rows: u32) -> Vec<u8> {
    let img_w = (cols.max(1)) * px_per_col();
    let img_h = (rows.max(1)) * px_per_row();
    let svg = scene_to_svg(scene, img_w, img_h);
    if std::env::var("TWP_DEBUG_SVG").is_ok() {
        eprintln!("twp-proxy SVG:\n{svg}");
    }

    let opt = usvg_options();
    let tree = match usvg::Tree::from_str(&svg, &opt) {
        Ok(t) => t,
        Err(e) => panic!("twp-proxy: generated SVG failed to parse: {e}\n{svg}"),
    };
    let mut pixmap = resvg::tiny_skia::Pixmap::new(img_w, img_h).expect("pixmap allocation");
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().expect("png encode")
}
