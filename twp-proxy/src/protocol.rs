// TWP Phase 1 wire format — typed mirror of the JSON schema documented in
// README.md. Deserialization-only; we never need to re-emit this format.

use std::collections::HashMap;

use serde::Deserialize;

/// Root payload: optional scene and optional component definitions.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct Payload {
    #[serde(rename = "S", default)]
    pub scene: Option<Node>,

    #[serde(rename = "C", default)]
    pub defs: HashMap<String, Node>,
}

/// A node in the widget tree.
///
/// Phase 1 node types (`n`):
///   * `flex` — flexbox container · `box` — block container
///   * `text` — proportional text · `mono` — monospace-grid text (string in `t`)
///   * `svg` — inline vector (markup in `t`) · `img` — bitmap (see `img`)
///   * `stack` — z-layered overlay
///   * `$param` — placeholder filled from the enclosing `$<name>` call's props
///   * `$<name>` — invocation of a component registered in `Payload::defs`
#[derive(Debug, Default, Clone, Deserialize)]
pub struct Node {
    pub n: String,

    #[serde(default)]
    pub s: Style,

    #[serde(default)]
    pub c: Vec<Node>,

    /// Text content; only meaningful for `text` nodes.
    #[serde(default)]
    pub t: Option<String>,

    /// For `$param` nodes: which hole this fills.
    #[serde(default)]
    pub name: Option<String>,

    /// For `$<name>` component invocations: values that fill the def's holes.
    #[serde(default)]
    pub props: HashMap<String, PropValue>,

    /// For `img` nodes: where the bitmap comes from.
    #[serde(default)]
    pub img: Option<Img>,
}

/// Image source for an `img` node. Keys mirror the Kitty graphics protocol so
/// the same source description works in both places:
///   * `f` — format: 100 = PNG/encoded (default), 32 = RGBA, 24 = RGB
///   * `t` — transmission: `"d"` = direct base64 in `d`, `"f"` = file `path`
///   * `s` / `v` — pixel width / height (required for raw `f=32`/`f=24`)
///   * `d` — base64 payload (Kitty's APC data)
///   * `path` — filesystem path (for `t=f`)
/// Transmission defaults to direct when `d` is present, file when `path` is.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct Img {
    #[serde(rename = "f", default)]
    pub format: Option<u32>,
    #[serde(rename = "t", default)]
    pub transmission: Option<String>,
    #[serde(rename = "s", default)]
    pub data_width: Option<u32>,
    #[serde(rename = "v", default)]
    pub data_height: Option<u32>,
    #[serde(rename = "d", default)]
    pub data: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

/// A component invocation's prop value. For ergonomics, a bare string is
/// auto-wrapped in a `text` node at expansion time; an object is parsed as
/// a full node tree.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum PropValue {
    Text(String),
    Node(Box<Node>),
}

impl PropValue {
    pub fn into_node(self) -> Node {
        match self {
            PropValue::Node(n) => *n,
            PropValue::Text(s) => Node {
                n: "text".to_string(),
                t: Some(s),
                ..Node::default()
            },
        }
    }
}

/// The Phase 1 style vocabulary. Any property absent here is silently
/// dropped — that's the protocol's compatibility story for older renderers.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Style {
    // Layout (only meaningful on `flex` containers)
    #[serde(rename = "flex-direction")]
    pub flex_direction: Option<String>,
    #[serde(rename = "justify-content")]
    pub justify_content: Option<String>,
    #[serde(rename = "align-items")]
    pub align_items: Option<String>,
    pub gap: Option<Dimension>,
    pub padding: Option<Dimension>,

    // Sizing
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,

    // Visual
    pub background: Option<String>,
    pub color: Option<String>,
    #[serde(rename = "border-radius")]
    pub border_radius: Option<Dimension>,
    // `border` / `border-*` are not typed: they are the real CSS border
    // properties (shorthand or per-side longhands), collected in `extra` and
    // forwarded to the renderer with CSS semantics. See the grid note below.

    // Text
    #[serde(rename = "font-size")]
    pub font_size: Option<f32>,
    #[serde(rename = "font-weight")]
    pub font_weight: Option<FontWeight>,
    #[serde(rename = "text-align")]
    pub text_align: Option<String>,

    // Mono text sizing (mirrors Kitty text-sizing protocol [2])
    pub scale: Option<u32>,
    #[serde(rename = "char-width")]
    pub char_width: Option<u32>,
    #[serde(rename = "subscale-n")]
    pub subscale_n: Option<u32>,
    #[serde(rename = "subscale-d")]
    pub subscale_d: Option<u32>,

    /// Any style key not recognized above is collected here and passed
    /// through to the renderer as a raw CSS `property: value` declaration.
    /// This is how effects beyond the typed vocabulary — `text-shadow`,
    /// `opacity`, `-webkit-text-stroke`, `text-decoration`, `filter`, etc.
    /// — reach the rasterizer without per-property wiring.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A length. A bare number is pixels; a string carries a unit:
///   * `"50%"`  — percentage of the parent
///   * `"3mcw"` — monospace cell *widths* (x-axis, `n · px_per_col`)
///   * `"2mch"` — monospace cell *heights* (y-axis, `n · px_per_row`)
///
/// The cell units are the protocol's native, terminal-portable lengths: they
/// resolve against the real per-terminal cell size (queried at runtime), so a
/// widget lines up with the character grid regardless of font, size, or DPI.
/// A pixel-square element is the same unit on both axes (a cell unit resolves
/// to a fixed pixel count whichever axis it's used on). Pixels are the escape
/// hatch for sub-cell cosmetics.
#[derive(Debug, Clone, Copy)]
pub enum Dimension {
    Px(f32),
    Percent(f32),
    /// Cell widths (× px_per_col).
    ColWidth(f32),
    /// Cell heights (× px_per_row).
    RowHeight(f32),
}

impl<'de> Deserialize<'de> for Dimension {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct DimVisitor;
        impl serde::de::Visitor<'_> for DimVisitor {
            type Value = Dimension;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a number (px) or a string like \"50%\", \"3mcw\", \"2mch\"")
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Dimension, E> {
                Ok(Dimension::Px(v as f32))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Dimension, E> {
                Ok(Dimension::Px(v as f32))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Dimension, E> {
                Ok(Dimension::Px(v as f32))
            }
            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Dimension, E> {
                parse_dimension(s).ok_or_else(|| {
                    E::custom(format!(
                        "invalid length: `{s}` (expected px number, %, or m-cell unit)"
                    ))
                })
            }
        }
        d.deserialize_any(DimVisitor)
    }
}

/// Parse a string length. Returns `None` on anything unrecognized.
fn parse_dimension(s: &str) -> Option<Dimension> {
    let s = s.trim();
    if let Some(p) = s.strip_suffix('%') {
        return p.trim().parse().ok().map(Dimension::Percent);
    }
    for (suffix, ctor) in [
        ("mcw", Dimension::ColWidth as fn(f32) -> Dimension),
        ("mch", Dimension::RowHeight as fn(f32) -> Dimension),
        ("px", Dimension::Px as fn(f32) -> Dimension),
    ] {
        if let Some(n) = s.strip_suffix(suffix) {
            return n.trim().parse().ok().map(ctor);
        }
    }
    // Bare numeric string → px.
    s.parse().ok().map(Dimension::Px)
}

/// font-weight: `"normal"`, `"bold"`, or a number 100–900. The name is
/// resolved into a numeric weight in render.rs.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FontWeight {
    Name(String),
    Number(u16),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Payload {
        serde_json::from_str(json).expect("valid payload")
    }

    #[test]
    fn empty_payload_is_no_op() {
        let p = parse("{}");
        assert!(p.scene.is_none());
        assert!(p.defs.is_empty());
    }

    #[test]
    fn parses_minimal_scene() {
        let p = parse(r##"{"S":{"n":"box","s":{"background":"#abc"}}}"##);
        let scene = p.scene.unwrap();
        assert_eq!(scene.n, "box");
        assert_eq!(scene.s.background.as_deref(), Some("#abc"));
    }

    #[test]
    fn parses_text_node() {
        let p = parse(r#"{"S":{"n":"text","t":"hello","s":{"font-size":16}}}"#);
        let scene = p.scene.unwrap();
        assert_eq!(scene.n, "text");
        assert_eq!(scene.t.as_deref(), Some("hello"));
        assert_eq!(scene.s.font_size, Some(16.0));
    }

    #[test]
    fn parses_dimension_px_and_percent() {
        let p = parse(r#"{"S":{"n":"box","s":{"width":200,"height":"50%"}}}"#);
        let scene = p.scene.unwrap();
        match scene.s.width.unwrap() {
            Dimension::Px(v) => assert_eq!(v, 200.0),
            _ => panic!("expected px"),
        }
        match scene.s.height.unwrap() {
            Dimension::Percent(v) => assert_eq!(v, 50.0),
            _ => panic!("expected percent"),
        }
    }

    #[test]
    fn parses_component_def_and_invocation() {
        let p = parse(
            r#"{
                "S": {"n":"$badge","props":{"label":"PASS"}},
                "C": {
                    "badge": {"n":"box","c":[{"n":"$param","name":"label"}]}
                }
            }"#,
        );
        assert!(p.defs.contains_key("badge"));
        let scene = p.scene.unwrap();
        assert_eq!(scene.n, "$badge");
        assert!(matches!(scene.props.get("label"), Some(PropValue::Text(_))));
    }

    #[test]
    fn unknown_style_props_silently_dropped() {
        // gradient is Phase 2; must not error
        let p = parse(r#"{"S":{"n":"box","s":{"gradient":"red,blue"}}}"#);
        assert_eq!(p.scene.unwrap().n, "box");
    }
}
