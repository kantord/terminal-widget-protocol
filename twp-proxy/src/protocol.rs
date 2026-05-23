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
/// Phase 1 primitives:
///   * `n = "box"`     — container
///   * `n = "text"`    — text run (string in `t`)
///   * `n = "$param"`  — placeholder, substituted from enclosing $call's props
///   * `n = "$<name>"` — invocation of a component registered in `Payload::defs`
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
    // Layout
    pub display: Option<String>,
    #[serde(rename = "flex-direction")]
    pub flex_direction: Option<String>,
    #[serde(rename = "justify-content")]
    pub justify_content: Option<String>,
    #[serde(rename = "align-items")]
    pub align_items: Option<String>,
    pub gap: Option<f32>,
    pub padding: Option<f32>,

    // Sizing
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,

    // Visual
    pub background: Option<String>,
    pub color: Option<String>,
    #[serde(rename = "border-radius")]
    pub border_radius: Option<Dimension>,
    pub border: Option<Border>,

    // Text
    #[serde(rename = "font-size")]
    pub font_size: Option<f32>,
    #[serde(rename = "font-weight")]
    pub font_weight: Option<FontWeight>,
    #[serde(rename = "text-align")]
    pub text_align: Option<String>,
}

/// A length, either pixels (numeric) or a percentage string like `"50%"`.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(untagged)]
pub enum Dimension {
    Px(f32),
    #[serde(deserialize_with = "deserialize_percent")]
    Percent(f32),
}

fn deserialize_percent<'de, D>(d: D) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    let trimmed = s.trim();
    let pct = trimmed
        .strip_suffix('%')
        .ok_or_else(|| serde::de::Error::custom("expected `Npx` number or `N%` string"))?;
    pct.trim()
        .parse::<f32>()
        .map_err(|e| serde::de::Error::custom(format!("invalid percent value: {e}")))
}

/// font-weight: `"normal"`, `"bold"`, or a number 100–900. The name is
/// resolved into a numeric weight in render.rs.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FontWeight {
    Name(String),
    Number(u16),
}

/// Solid border. Phase 1 supports no other border styles.
#[derive(Debug, Clone, Deserialize)]
pub struct Border {
    pub width: f32,
    pub color: String,
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
