// Converts an expanded protocol::Node tree into a takumi node tree and
// renders it to a PNG.
//
// Phase 1 style vocabulary is hand-mapped onto takumi's StyleDeclaration
// API. Anything outside the documented vocabulary is silently dropped.

use std::borrow::Cow;
use std::sync::OnceLock;

use takumi::{
    GlobalContext,
    layout::{
        Viewport,
        node::Node as TakumiNode,
        style::{
            AlignItems, Color, ColorInput, Display, FlexDirection, FontSize, FontWeight as TkFW,
            JustifyContent, Length, LengthDefaultsToZero, SpacePair, Style as TkStyle,
            StyleDeclaration, TextAlign,
        },
    },
    rendering::{ImageOutputFormat, RenderOptions, render, write_image},
    resources::font::FontResource,
};

use crate::protocol::{Border, Dimension, FontWeight, Node};

/// Render viewport. 5:2 aspect matches a 20×4 cell footprint at typical
/// terminal cell aspect (~1:2 W:H).
pub const IMG_W: u32 = 400;
pub const IMG_H: u32 = 160;

/// First system font path that exists wins. Best-effort; if none are
/// found, text nodes render as empty space. Phase 2 will let the
/// terminal supply its own font.
const FONT_PROBE: &[&str] = &[
    "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
    "/usr/share/fonts/adobe-source-code-pro/SourceCodePro-Regular.otf",
    "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
    "/usr/share/fonts/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/System/Library/Fonts/Menlo.ttc",
    "/System/Library/Fonts/Monaco.ttf",
];

fn context() -> &'static GlobalContext {
    static CTX: OnceLock<GlobalContext> = OnceLock::new();
    CTX.get_or_init(|| {
        let mut ctx = GlobalContext::default();
        for path in FONT_PROBE {
            if let Ok(bytes) = std::fs::read(path) {
                if ctx
                    .font_context
                    .load_and_store(FontResource::new(bytes))
                    .is_ok()
                {
                    return ctx;
                }
            }
        }
        eprintln!(
            "twp-proxy: no system font found; text widgets will render empty. \
             Looked in: {FONT_PROBE:?}"
        );
        ctx
    })
}

pub fn render_to_png(scene: &Node) -> Vec<u8> {
    let takumi_root = to_takumi(scene);
    let opts = RenderOptions::builder()
        .viewport(Viewport::new((IMG_W, IMG_H)))
        .node(takumi_root)
        .global(context())
        .build();
    let img = render(opts).expect("takumi render");
    let mut buf = Vec::with_capacity(8 * 1024);
    write_image(Cow::Owned(img), &mut buf, ImageOutputFormat::Png, None)
        .expect("png encode");
    buf
}

fn to_takumi(node: &Node) -> TakumiNode {
    let style = build_style(node);
    match node.n.as_str() {
        "text" => {
            let text = node.t.as_deref().unwrap_or("");
            TakumiNode::text(text).with_style(style)
        }
        // "box" or anything not text (including unfilled-component placeholders).
        _ => {
            let children: Vec<TakumiNode> = node.c.iter().map(to_takumi).collect();
            TakumiNode::container(children).with_style(style)
        }
    }
}

fn build_style(node: &Node) -> TkStyle {
    let s = &node.s;
    let mut style = TkStyle::default();

    // Layout
    let display = s
        .display
        .as_deref()
        .map(parse_display)
        .unwrap_or_else(|| if node.n == "box" { Display::Flex } else { Display::Inline });
    style = style.with(StyleDeclaration::display(display));

    if let Some(fd) = s.flex_direction.as_deref().and_then(parse_flex_direction) {
        style = style.with(StyleDeclaration::flex_direction(fd));
    }
    if let Some(jc) = s.justify_content.as_deref().and_then(parse_justify_content) {
        style = style.with(StyleDeclaration::justify_content(jc));
    }
    if let Some(ai) = s.align_items.as_deref().and_then(parse_align_items) {
        style = style.with(StyleDeclaration::align_items(ai));
    }
    if let Some(gap) = s.gap {
        let len = LengthDefaultsToZero::Px(gap);
        style = style
            .with(StyleDeclaration::column_gap(len))
            .with(StyleDeclaration::row_gap(len));
    }
    if let Some(p) = s.padding {
        let len = LengthDefaultsToZero::Px(p);
        style = style
            .with(StyleDeclaration::padding_top(len))
            .with(StyleDeclaration::padding_right(len))
            .with(StyleDeclaration::padding_bottom(len))
            .with(StyleDeclaration::padding_left(len));
    }

    // Sizing
    if let Some(w) = s.width {
        style = style.with(StyleDeclaration::width(to_length(w)));
    }
    if let Some(h) = s.height {
        style = style.with(StyleDeclaration::height(to_length(h)));
    }

    // Visual
    if let Some(bg) = s.background.as_deref().and_then(parse_color) {
        style = style.with(StyleDeclaration::background_color(ColorInput::from(bg)));
    }
    if let Some(c) = s.color.as_deref().and_then(parse_color) {
        style = style.with(StyleDeclaration::color(ColorInput::from(c)));
    }
    if let Some(r) = s.border_radius {
        let pair = SpacePair::from_single(to_length_zero(r));
        style = style
            .with(StyleDeclaration::border_top_left_radius(pair))
            .with(StyleDeclaration::border_top_right_radius(pair))
            .with(StyleDeclaration::border_bottom_right_radius(pair))
            .with(StyleDeclaration::border_bottom_left_radius(pair));
    }
    if let Some(b) = &s.border {
        apply_border(&mut style, b);
    }

    // Text
    if let Some(fs) = s.font_size {
        style = style.with(StyleDeclaration::font_size(FontSize::Length(Length::Px(fs))));
    }
    if let Some(fw) = &s.font_weight {
        let n: f32 = match fw {
            FontWeight::Name(n) if n == "bold" => 700.0,
            FontWeight::Name(n) if n == "normal" => 400.0,
            FontWeight::Name(_) => 400.0,
            FontWeight::Number(n) => *n as f32,
        };
        style = style.with(StyleDeclaration::font_weight(TkFW::from(n)));
    }
    if let Some(ta) = s.text_align.as_deref().and_then(parse_text_align) {
        style = style.with(StyleDeclaration::text_align(ta));
    }

    style
}

fn apply_border(style: &mut TkStyle, b: &Border) {
    let w = Length::Px(b.width);
    let take = std::mem::take(style);
    let mut next = take
        .with(StyleDeclaration::border_top_width(w))
        .with(StyleDeclaration::border_right_width(w))
        .with(StyleDeclaration::border_bottom_width(w))
        .with(StyleDeclaration::border_left_width(w))
        .with(StyleDeclaration::border_top_style(
            takumi::layout::style::BorderStyle::Solid,
        ))
        .with(StyleDeclaration::border_right_style(
            takumi::layout::style::BorderStyle::Solid,
        ))
        .with(StyleDeclaration::border_bottom_style(
            takumi::layout::style::BorderStyle::Solid,
        ))
        .with(StyleDeclaration::border_left_style(
            takumi::layout::style::BorderStyle::Solid,
        ));
    if let Some(color) = parse_color(&b.color) {
        let ci = ColorInput::from(color);
        next = next
            .with(StyleDeclaration::border_top_color(ci.clone()))
            .with(StyleDeclaration::border_right_color(ci.clone()))
            .with(StyleDeclaration::border_bottom_color(ci.clone()))
            .with(StyleDeclaration::border_left_color(ci));
    }
    *style = next;
}

fn to_length(d: Dimension) -> Length {
    match d {
        Dimension::Px(v) => Length::Px(v),
        Dimension::Percent(v) => Length::Percentage(v),
    }
}

fn to_length_zero(d: Dimension) -> LengthDefaultsToZero {
    match d {
        Dimension::Px(v) => LengthDefaultsToZero::Px(v),
        Dimension::Percent(v) => LengthDefaultsToZero::Percentage(v),
    }
}

fn parse_color(s: &str) -> Option<Color> {
    let hex = s.strip_prefix('#')?;
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
    Some(Color::from([r, g, b, a]))
}

fn parse_display(s: &str) -> Display {
    match s {
        "flex" => Display::Flex,
        "block" => Display::Block,
        "inline" => Display::Inline,
        "none" => Display::None,
        _ => Display::Flex,
    }
}

fn parse_flex_direction(s: &str) -> Option<FlexDirection> {
    Some(match s {
        "row" => FlexDirection::Row,
        "column" => FlexDirection::Column,
        _ => return None,
    })
}

fn parse_justify_content(s: &str) -> Option<JustifyContent> {
    Some(match s {
        "start" | "flex-start" => JustifyContent::FlexStart,
        "end" | "flex-end" => JustifyContent::FlexEnd,
        "center" => JustifyContent::Center,
        "space-between" => JustifyContent::SpaceBetween,
        "space-around" => JustifyContent::SpaceAround,
        "space-evenly" => JustifyContent::SpaceEvenly,
        _ => return None,
    })
}

fn parse_align_items(s: &str) -> Option<AlignItems> {
    Some(match s {
        "start" | "flex-start" => AlignItems::FlexStart,
        "end" | "flex-end" => AlignItems::FlexEnd,
        "center" => AlignItems::Center,
        "stretch" => AlignItems::Stretch,
        _ => return None,
    })
}

fn parse_text_align(s: &str) -> Option<TextAlign> {
    Some(match s {
        "left" => TextAlign::Left,
        "center" => TextAlign::Center,
        "right" => TextAlign::Right,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Payload;
    use crate::expand::expand;

    fn render_json(json: &str) -> Vec<u8> {
        let payload: Payload = serde_json::from_str(json).unwrap();
        let scene = expand(payload.scene.unwrap(), &payload.defs);
        render_to_png(&scene)
    }

    #[test]
    fn renders_minimal_box() {
        let bytes = render_json(r##"{"S":{"n":"box","s":{"background":"#0a0","width":200,"height":100}}}"##);
        assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
    }

    #[test]
    fn renders_traffic_light_via_components() {
        let bytes = render_json(
            r##"{
                "S": {"n":"box","s":{"display":"flex","flex-direction":"row","justify-content":"space-around","align-items":"center","background":"#2a2d3a","width":400,"height":160,"border-radius":40},
                      "c":[
                        {"n":"$dot","props":{"col":{"n":"box","s":{"width":80,"height":80,"background":"#f04646","border-radius":"50%"}}}},
                        {"n":"$dot","props":{"col":{"n":"box","s":{"width":80,"height":80,"background":"#fac83c","border-radius":"50%"}}}},
                        {"n":"$dot","props":{"col":{"n":"box","s":{"width":80,"height":80,"background":"#50dc6e","border-radius":"50%"}}}}
                      ]},
                "C": { "dot": {"n":"$param","name":"col"} }
            }"##,
        );
        assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
    }
}
