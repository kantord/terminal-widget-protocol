// Converts an expanded protocol::Node tree into a takumi node tree and
// renders it to a PNG.
//
// Phase 1 style vocabulary is hand-mapped onto takumi's StyleDeclaration
// API. Anything outside the documented vocabulary is silently dropped.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use parley::{FontFeature, setting::Tag};
use takumi::{
    GlobalContext,
    layout::{
        Viewport,
        node::Node as TakumiNode,
        style::{
            AlignItems, Color, ColorInput, Display, FlexDirection, FontFamily, FontSize,
            FontWeight as TkFW, FromCss, JustifyContent, Length, LengthDefaultsToZero, SpacePair,
            Style as TkStyle, StyleDeclaration, StyleDeclarationBlock, TextAlign,
        },
    },
    rendering::{ImageOutputFormat, RenderOptions, measure_layout, render, write_image},
    resources::font::FontResource,
};

use crate::protocol::{Border, Dimension, FontWeight, Node};

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

/// Unique family names per weight/style — we force parley to pick the
/// exact file by never asking it to do weight-matching. Crude but
/// guaranteed to match the terminal's font selection (which also has
/// one explicit file per variant).
const FAMILY_REGULAR: &str = "twp-r";
const FAMILY_BOLD: &str = "twp-b";
const FAMILY_ITALIC: &str = "twp-i";
const FAMILY_BOLD_ITALIC: &str = "twp-bi";

#[derive(Debug, Default)]
struct FontSet {
    regular: Option<std::path::PathBuf>,
    bold: Option<std::path::PathBuf>,
    italic: Option<std::path::PathBuf>,
    bold_italic: Option<std::path::PathBuf>,
}

/// Best-effort font discovery:
///   1. `TWP_FONT_PATH` env override (regular only).
///   2. Kitty config: regular + bold + italic + bold_italic.
///   3. `fc-match monospace` for regular.
///   4. `FONT_PROBE` paths.
fn discover_fonts() -> FontSet {
    if let Ok(p) = std::env::var("TWP_FONT_PATH") {
        return FontSet {
            regular: Some(std::path::PathBuf::from(p)),
            ..FontSet::default()
        };
    }
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        let kitty = read_kitty_fonts();
        if kitty.regular.is_some() {
            return FontSet {
                regular: kitty.regular.as_deref().and_then(fc_match),
                bold: kitty.bold.as_deref().and_then(fc_match),
                italic: kitty.italic.as_deref().and_then(fc_match),
                bold_italic: kitty.bold_italic.as_deref().and_then(fc_match),
            };
        }
    }
    if let Some(path) = fc_match("monospace") {
        return FontSet {
            regular: Some(path),
            ..FontSet::default()
        };
    }
    for p in FONT_PROBE {
        if std::path::Path::new(p).exists() {
            return FontSet {
                regular: Some(std::path::PathBuf::from(p)),
                ..FontSet::default()
            };
        }
    }
    FontSet::default()
}

#[derive(Debug, Default)]
struct KittyFonts {
    regular: Option<String>,
    bold: Option<String>,
    italic: Option<String>,
    bold_italic: Option<String>,
}

fn read_kitty_fonts() -> KittyFonts {
    let mut fonts = KittyFonts::default();
    let Some(cfg_dir) = std::env::var("XDG_CONFIG_HOME")
        .or_else(|_| std::env::var("HOME").map(|h| format!("{h}/.config")))
        .ok()
    else {
        return fonts;
    };
    let path = std::path::Path::new(&cfg_dir).join("kitty/kitty.conf");
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return fonts;
    };
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        for key in ["font_family", "bold_font", "italic_font", "bold_italic_font"] {
            if let Some(rest) = line.strip_prefix(key) {
                let value = extract_family(rest.trim());
                match key {
                    "font_family" => fonts.regular = Some(value),
                    "bold_font" => fonts.bold = Some(value),
                    "italic_font" => fonts.italic = Some(value),
                    "bold_italic_font" => fonts.bold_italic = Some(value),
                    _ => {}
                }
                break;
            }
        }
    }
    fonts
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

fn context() -> &'static GlobalContext {
    static CTX: OnceLock<GlobalContext> = OnceLock::new();
    CTX.get_or_init(|| {
        let mut ctx = GlobalContext::default();
        let set = fonts();
        if set.regular.is_none() {
            eprintln!(
                "twp-proxy: no font found via Kitty config, fc-match, or probe paths; \
                 text widgets will render empty"
            );
            return ctx;
        }
        // Each variant gets a unique family name — we select the right
        // one explicitly in build_style rather than relying on parley's
        // weight-matching (which doesn't reliably pick overridden variants).
        load_variant(&mut ctx, set.regular.as_deref(), FAMILY_REGULAR, false, false);
        load_variant(&mut ctx, set.bold.as_deref(), FAMILY_BOLD, true, false);
        load_variant(&mut ctx, set.italic.as_deref(), FAMILY_ITALIC, false, true);
        load_variant(
            &mut ctx,
            set.bold_italic.as_deref(),
            FAMILY_BOLD_ITALIC,
            true,
            true,
        );
        ctx
    })
}

fn load_variant(
    ctx: &mut GlobalContext,
    path: Option<&std::path::Path>,
    family_name: &str,
    bold: bool,
    italic: bool,
) {
    let Some(path) = path else { return };
    let Ok(bytes) = std::fs::read(path) else {
        eprintln!("twp-proxy: failed to read {}", path.display());
        return;
    };
    use parley::fontique::{FontInfoOverride, FontStyle, FontWeight as FqWeight};
    let override_info = FontInfoOverride {
        family_name: Some(family_name),
        weight: Some(if bold { FqWeight::BOLD } else { FqWeight::NORMAL }),
        style: Some(if italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        }),
        ..Default::default()
    };
    match ctx
        .font_context
        .load_and_store(FontResource::new(bytes).override_info(override_info))
    {
        Ok(()) => {
            let variant = match (bold, italic) {
                (false, false) => "regular",
                (true, false) => "bold",
                (false, true) => "italic",
                (true, true) => "bold-italic",
            };
            if std::env::var("TWP_DEBUG").is_ok() {
                eprintln!(
                    "twp-proxy: loaded {variant} font {} (family={family_name})",
                    path.display(),
                );
            }
        }
        Err(e) => eprintln!(
            "twp-proxy: failed to load {}: {e:?}",
            path.display()
        ),
    }
}

/// Default font-size in render-resolution pixels when a `text` node
/// doesn't specify one. Tuned to be roughly cell-height; this is what
/// glyphs in a "unstyled" text widget will render at.
const DEFAULT_FONT_SIZE_PX: f32 = 32.0;

/// Measured natural glyph advance for a (font-size px, weight) pair —
/// what parley produces with default settings before we add any
/// letter-spacing. Cached because each entry costs a layout pass.
fn natural_advance(font_size_px: f32, weight: u16) -> f32 {
    static CACHE: OnceLock<Mutex<HashMap<(u32, u16), f32>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = (font_size_px.to_bits(), weight);
    if let Some(&a) = cache.lock().unwrap().get(&key) {
        return a;
    }

    // Measure a long ASCII run; we want the inline text run's width so we
    // can divide by char count for per-glyph advance. Wrap the text in a
    // flex container so parley produces an inline run we can read.
    let sample = "0123456789 abcdefghij klmnopqrstu vwxyz ABCDE";
    let family_key = if weight >= 700 { FAMILY_BOLD } else { FAMILY_REGULAR };
    let mut probe_style = TkStyle::default()
        .with(StyleDeclaration::font_size(FontSize::Length(Length::Px(
            font_size_px,
        ))))
        .with(StyleDeclaration::font_weight(TkFW::from(weight as f32)))
        .with(StyleDeclaration::font_feature_settings(disable_ligatures()));
    if let Ok(ff) = FontFamily::from_str(&format!("\"{family_key}\"")) {
        probe_style = probe_style.with(StyleDeclaration::font_family(ff));
    }
    let text_node = TakumiNode::text(sample).with_style(probe_style);
    let probe = TakumiNode::container(vec![text_node]).with_style(
        TkStyle::default().with(StyleDeclaration::display(Display::Flex)),
    );
    let opts = RenderOptions::builder()
        .viewport(Viewport::new((4096u32, 256u32)))
        .node(probe)
        .global(context())
        .build();
    let measured = measure_layout(opts).expect("measure_layout");
    // The flex container has no explicit width, so it shrinks to its
    // single text child's natural width — exactly what we want.
    let advance = measured.width / sample.chars().count() as f32;
    cache.lock().unwrap().insert(key, advance);
    advance
}

/// Build a font-feature-settings list that disables OpenType ligatures
/// (`liga`, `clig`, `dlig`) and discretionary calt — Nerd Fonts and
/// programming fonts often have these enabled, which can shift per-glyph
/// advance widths and break our integer-pixel assumption.
fn disable_ligatures() -> Box<[FontFeature]> {
    Box::new([
        FontFeature::new(Tag::new(b"liga"), 0),
        FontFeature::new(Tag::new(b"clig"), 0),
        FontFeature::new(Tag::new(b"dlig"), 0),
        FontFeature::new(Tag::new(b"calt"), 0),
    ])
}

/// Returns the `letter-spacing` value (in px at our render resolution)
/// needed to coerce per-glyph advance to the nearest integer pixel.
/// Without this, parley's fractional advances accumulate across long
/// strings and the resulting glyphs drift away from the cell grid when
/// Kitty downscales to the host cell box.
fn integer_pixel_letter_spacing(font_size_px: f32, weight: u16) -> f32 {
    let natural = natural_advance(font_size_px, weight);
    natural.ceil() - natural
}

pub fn render_to_png(scene: &Node, cols: u32, rows: u32) -> Vec<u8> {
    let img_w = (cols.max(1)) * px_per_col();
    let img_h = (rows.max(1)) * px_per_row();
    let takumi_root = to_takumi(scene);
    let opts = RenderOptions::builder()
        .viewport(Viewport::new((img_w, img_h)))
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
        "mono" => build_mono(node),
        // "flex", "box", or anything not text (including unfilled-component
        // placeholders). Layout differences are encoded via the `display`
        // declaration applied in build_style.
        _ => {
            let children: Vec<TakumiNode> = node.c.iter().map(to_takumi).collect();
            TakumiNode::container(children).with_style(style)
        }
    }
}

/// Parse the style's CSS passthrough map (`Style::extra`) into a list of
/// takumi declarations. Each `key: value` pair is assembled into one CSS
/// block and parsed by takumi's own CSS engine, so any property takumi
/// supports — `text-shadow`, `opacity`, `-webkit-text-stroke`,
/// `text-decoration`, `filter`, … — works with no per-property code.
/// Malformed input is dropped (forward-compat), matching the typed
/// vocabulary's "unknown props silently ignored" behavior.
fn css_passthrough_decls(extra: &HashMap<String, serde_json::Value>) -> Vec<StyleDeclaration> {
    let mut decls = Vec::new();
    // Parse each property on its own so one unparseable value is dropped in
    // isolation rather than discarding the whole style (takumi's block parser
    // is all-or-nothing).
    for (key, value) in extra {
        let val = match value {
            serde_json::Value::String(s) => Cow::Borrowed(s.as_str()),
            serde_json::Value::Number(n) => Cow::Owned(n.to_string()),
            serde_json::Value::Bool(b) => Cow::Owned(b.to_string()),
            // arrays / objects / null are not valid CSS values
            _ => continue,
        };
        let css = format!("{key}: {val}");
        match css.parse::<StyleDeclarationBlock>() {
            Ok(block) => decls.extend(block.iter().cloned()),
            Err(_) => eprintln!("twp-proxy: ignoring invalid CSS property: {css}"),
        }
    }
    decls
}

/// Build a monospace-grid text node: each character gets its own
/// fixed-width cell box, so glyph positions are determined by the grid
/// and not by the font's advance width. Zero drift by construction.
fn build_mono(node: &Node) -> TakumiNode {
    let text = node.t.as_deref().unwrap_or("");
    let s = &node.s;

    // Text-sizing parameters — mirrors Kitty's OSC 66 [2]:
    //   scale (s): each char occupies scale×scale cells
    //   char-width (w): override horizontal cell count per char
    //   subscale-n/d: fractional glyph size within the cell block
    let scale = s.scale.unwrap_or(1).max(1);
    let char_w_cells = s.char_width.unwrap_or(scale); // w=0 → same as scale
    let (sub_n, sub_d) = match (s.subscale_n, s.subscale_d) {
        (Some(n), Some(d)) if d > 0 => (n.min(d) as f32, d as f32),
        _ => (1.0, 1.0), // no subscale → full size
    };

    let base_font = px_per_row() as f32 * 0.75;
    let font_size_px = s
        .font_size
        .unwrap_or(base_font * scale as f32 * sub_n / sub_d);

    let weight: u16 = match &s.font_weight {
        Some(FontWeight::Name(n)) if n == "bold" => 700,
        Some(FontWeight::Number(n)) => *n,
        _ => 400,
    };
    let family_key = if weight >= 700 { FAMILY_BOLD } else { FAMILY_REGULAR };
    let fg_color = s.color.as_deref().and_then(parse_color);

    // Text effects (text-shadow, opacity, stroke, …) come through the CSS
    // passthrough and apply per glyph. Parsed once, cloned onto each cell.
    let extra_decls = css_passthrough_decls(&s.extra);

    // Build a flex row of single-character cells. Each cell is
    // char_w_cells × scale terminal cells.
    let cell_w = px_per_col() as f32 * char_w_cells as f32;
    let cell_h = px_per_row() as f32 * scale as f32;
    let cells: Vec<TakumiNode> = text
        .chars()
        .map(|ch| {
            let mut char_style = TkStyle::default()
                .with(StyleDeclaration::font_size(FontSize::Length(Length::Px(
                    font_size_px,
                ))))
                .with(StyleDeclaration::font_feature_settings(
                    disable_ligatures(),
                ));
            if let Ok(ff) = FontFamily::from_str(&format!("\"{family_key}\"")) {
                char_style = char_style.with(StyleDeclaration::font_family(ff));
            }
            // Don't set font-weight — we already selected the correct
            // font file via family_key. Requesting weight 700 on a font
            // whose internal metadata says 400 makes parley fail to match.
            if let Some(c) = fg_color {
                char_style =
                    char_style.with(StyleDeclaration::color(ColorInput::from(c)));
            }
            for decl in &extra_decls {
                char_style = char_style.with(decl.clone());
            }
            let glyph = TakumiNode::text(ch.to_string()).with_style(char_style);

            let cell_style = TkStyle::default()
                .with(StyleDeclaration::display(Display::Flex))
                .with(StyleDeclaration::justify_content(JustifyContent::Center))
                .with(StyleDeclaration::align_items(AlignItems::Center))
                .with(StyleDeclaration::width(Length::Px(cell_w)))
                .with(StyleDeclaration::height(Length::Px(cell_h)));
            TakumiNode::container(vec![glyph]).with_style(cell_style)
        })
        .collect();

    // Outer container: flex row, no gap.
    let mut outer_style = TkStyle::default()
        .with(StyleDeclaration::display(Display::Flex))
        .with(StyleDeclaration::flex_direction(FlexDirection::Row))
        .with(StyleDeclaration::align_items(AlignItems::Center));

    // Apply visual styling from the node (background, padding, etc.).
    if let Some(bg) = s.background.as_deref().and_then(parse_color) {
        outer_style =
            outer_style.with(StyleDeclaration::background_color(ColorInput::from(bg)));
    }
    if let Some(w) = s.width {
        outer_style = outer_style.with(StyleDeclaration::width(to_length(w)));
    }
    if let Some(h) = s.height {
        outer_style = outer_style.with(StyleDeclaration::height(to_length(h)));
    }
    if let Some(r) = s.border_radius {
        let pair = SpacePair::from_single(to_length_zero(r));
        outer_style = outer_style
            .with(StyleDeclaration::border_top_left_radius(pair))
            .with(StyleDeclaration::border_top_right_radius(pair))
            .with(StyleDeclaration::border_bottom_right_radius(pair))
            .with(StyleDeclaration::border_bottom_left_radius(pair));
    }

    TakumiNode::container(cells).with_style(outer_style)
}

fn default_display_for(node_name: &str) -> Display {
    match node_name {
        "flex" => Display::Flex,
        "box" => Display::Block,
        _ => Display::Block, // unknown / placeholder
    }
}

fn build_style(node: &Node) -> TkStyle {
    let s = &node.s;
    let mut style = TkStyle::default();

    // Layout — node name is the source of truth for the layout algorithm.
    // `flex` always uses flex; `box` is a no-layout styled container.
    let display = default_display_for(&node.n);
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
    let font_size_px = s.font_size.unwrap_or(DEFAULT_FONT_SIZE_PX);
    if s.font_size.is_some() || node.n == "text" {
        style = style.with(StyleDeclaration::font_size(FontSize::Length(Length::Px(
            font_size_px,
        ))));
    }
    let weight: u16 = match &s.font_weight {
        Some(FontWeight::Name(n)) if n == "bold" => 700,
        Some(FontWeight::Name(_)) => 400,
        Some(FontWeight::Number(n)) => *n,
        None => 400,
    };
    if s.font_weight.is_some() {
        style = style.with(StyleDeclaration::font_weight(TkFW::from(weight as f32)));
    }
    if let Some(ta) = s.text_align.as_deref().and_then(parse_text_align) {
        style = style.with(StyleDeclaration::text_align(ta));
    }

    // For text nodes: explicitly select the right font file (by family
    // name) based on weight, disable ligatures, and add letter-spacing
    // tuned to round per-glyph advance to the nearest integer pixel.
    // All implementation magic — the protocol's `text` node just
    // declares a string; how the rasterizer does the rest is our problem.
    if node.n == "text" {
        let family_key = if weight >= 700 { FAMILY_BOLD } else { FAMILY_REGULAR };
        if let Ok(ff) = FontFamily::from_str(&format!("\"{family_key}\"")) {
            style = style.with(StyleDeclaration::font_family(ff));
        }
        style = style.with(StyleDeclaration::font_feature_settings(disable_ligatures()));
        let spacing = integer_pixel_letter_spacing(font_size_px, weight);
        style = style.with(StyleDeclaration::letter_spacing(Length::Px(spacing)));
    }

    // CSS passthrough: any property outside the typed vocabulary (box-shadow,
    // background gradients, opacity, filter, …) applies to flex/box/text nodes
    // here, just as it does per-glyph for mono.
    for decl in css_passthrough_decls(&s.extra) {
        style = style.with(decl);
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

    fn render_json(json: &str, cols: u32, rows: u32) -> Vec<u8> {
        let payload: Payload = serde_json::from_str(json).unwrap();
        let scene = expand(payload.scene.unwrap(), &payload.defs);
        render_to_png(&scene, cols, rows)
    }

    #[test]
    fn renders_minimal_box() {
        let bytes = render_json(
            r##"{"S":{"n":"box","s":{"background":"#0a0","width":200,"height":100}}}"##,
            20,
            4,
        );
        assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
    }

    #[test]
    fn renders_traffic_light_via_components() {
        let bytes = render_json(
            r##"{
                "S": {"n":"flex","s":{"flex-direction":"row","justify-content":"space-around","align-items":"center","background":"#2a2d3a","width":400,"height":160,"border-radius":40},
                      "c":[
                        {"n":"$dot","props":{"col":{"n":"box","s":{"width":80,"height":80,"background":"#f04646","border-radius":"50%"}}}},
                        {"n":"$dot","props":{"col":{"n":"box","s":{"width":80,"height":80,"background":"#fac83c","border-radius":"50%"}}}},
                        {"n":"$dot","props":{"col":{"n":"box","s":{"width":80,"height":80,"background":"#50dc6e","border-radius":"50%"}}}}
                      ]},
                "C": { "dot": {"n":"$param","name":"col"} }
            }"##,
            20,
            4,
        );
        assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
    }
}
