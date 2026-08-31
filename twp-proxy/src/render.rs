// Converts an expanded protocol::Node tree into a takumi node tree and
// renders it to a PNG.
//
// Phase 1 style vocabulary is hand-mapped onto takumi's StyleDeclaration
// API. Anything outside the documented vocabulary is silently dropped.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use image::RgbaImage;
use parley::{FontFeature, setting::Tag};
use takumi::{
    GlobalContext,
    layout::{
        Viewport,
        node::{ImageData, Node as TakumiNode},
        style::{
            AlignItems, Color, ColorInput, Display, FlexDirection, FontFamily, FontSize,
            FontWeight as TkFW, FromCss, JustifyContent, Length, LengthDefaultsToZero, SpacePair,
            Style as TkStyle, StyleDeclaration, StyleDeclarationBlock, TextAlign,
        },
    },
    rendering::{ImageOutputFormat, RenderOptions, measure_layout, render, write_image},
    resources::{
        font::FontResource,
        image::{ImageSource, SvgSource},
    },
};

use crate::protocol::{Dimension, FontWeight, Img, Node};

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
// standard xterm default is used. `transparent` (a normal CSS keyword)
// lets the terminal background show through, no query needed.

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

/// Replace every `term(...)` token in a CSS/SVG value string with its `#rrggbb`
/// equivalent. The token is self-delimiting, so this is a safe substitution in
/// any color-bearing value without parsing the surrounding CSS.
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

/// Resolve `m`-cell units (`3mcw`, `0.5mch`) embedded in a CSS *value* string to
/// pixels, so they work in the passthrough path too (not just the typed
/// `width`/`height`/`gap`/`padding` fields). Anything that isn't
/// `<number><cell-unit>` is copied through untouched.
pub(crate) fn substitute_cell_units(value: &str) -> String {
    let pc = px_per_col() as f32;
    let pr = px_per_row() as f32;
    let units: [(&str, f32); 2] = [("mcw", pc), ("mch", pr)];
    let b = value.as_bytes();
    let mut out = String::with_capacity(value.len());
    let mut i = 0;
    let mut copied = 0; // bytes [copied..) not yet flushed to `out`
    while i < b.len() {
        let starts_num =
            b[i].is_ascii_digit() || (b[i] == b'.' && i + 1 < b.len() && b[i + 1].is_ascii_digit());
        if !starts_num {
            i += 1;
            continue;
        }
        let num_start = i;
        let mut j = i;
        while j < b.len() && (b[j].is_ascii_digit() || b[j] == b'.') {
            j += 1;
        }
        let rest = &value[j..];
        let mut matched = false;
        for (suffix, mult) in units {
            let Some(after) = rest.strip_prefix(suffix) else {
                continue;
            };
            // The unit must end at a non-alphanumeric boundary (so we don't
            // rewrite the `mcw` inside a hypothetical `3mcworld`).
            let boundary = after
                .as_bytes()
                .first()
                .is_none_or(|c| !c.is_ascii_alphanumeric());
            if boundary {
                if let Ok(n) = value[num_start..j].parse::<f32>() {
                    out.push_str(&value[copied..num_start]);
                    out.push_str(&format!("{}px", n * mult));
                    i = j + suffix.len();
                    copied = i;
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            i = j; // leave the bare number for the next bulk copy
        }
    }
    out.push_str(&value[copied..]);
    out
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

/// Optional proportional (sans-serif) family. Unlike the terminal mono font,
/// this is only used when a node explicitly asks for it via
/// `font-family: twp-sans` / `twp-sans-b` (e.g. the Markdown demo's prose and
/// headings, where mono looks wrong). Registered best-effort from
/// `fc-match sans-serif`; absent it, parley falls back to a system face.
pub const FAMILY_SANS: &str = "twp-sans";
pub const FAMILY_SANS_BOLD: &str = "twp-sans-b";

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
        if let Some(reg) = kitty.regular.clone() {
            // Kitty's `auto`/`none` for bold_font/italic_font are *directives*
            // ("derive from font_family"), not real font names. Passing them to
            // fc-match yields a proportional fallback (e.g. Noto Sans), which
            // wrecks the monospace grid for bold/italic mono text. So resolve
            // those slots from the regular family with a fontconfig style query
            // instead, getting the matching *monospace* face.
            let variant = |slot: Option<&str>, style: &str| -> Option<std::path::PathBuf> {
                match slot {
                    Some(v)
                        if !matches!(
                            v.trim().to_ascii_lowercase().as_str(),
                            "auto" | "none" | ""
                        ) =>
                    {
                        fc_match(v)
                    }
                    _ => fc_match(&format!("{reg}:{style}")),
                }
            };
            return FontSet {
                regular: fc_match(&reg),
                bold: variant(kitty.bold.as_deref(), "bold"),
                italic: variant(kitty.italic.as_deref(), "italic"),
                bold_italic: variant(kitty.bold_italic.as_deref(), "bold:italic"),
            };
        }
    }
    if let Some(path) = fc_match("monospace") {
        return FontSet {
            regular: Some(path),
            bold: fc_match("monospace:bold"),
            italic: fc_match("monospace:italic"),
            bold_italic: fc_match("monospace:bold:italic"),
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
        for key in [
            "font_family",
            "bold_font",
            "italic_font",
            "bold_italic_font",
        ] {
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
        // The bold/italic slots fall back to the regular face when no real
        // variant was found. The result may not look bolder, but it stays
        // monospace — never a proportional fallback that breaks the cell grid.
        load_variant(
            &mut ctx,
            set.regular.as_deref(),
            FAMILY_REGULAR,
            false,
            false,
        );
        load_variant(
            &mut ctx,
            set.bold.as_deref().or(set.regular.as_deref()),
            FAMILY_BOLD,
            true,
            false,
        );
        load_variant(
            &mut ctx,
            set.italic.as_deref().or(set.regular.as_deref()),
            FAMILY_ITALIC,
            false,
            true,
        );
        load_variant(
            &mut ctx,
            set.bold_italic
                .as_deref()
                .or(set.bold.as_deref())
                .or(set.regular.as_deref()),
            FAMILY_BOLD_ITALIC,
            true,
            true,
        );

        // Proportional family for prose/heading demos — best effort.
        if let Some(p) = fc_match("sans-serif") {
            load_variant(&mut ctx, Some(p.as_path()), FAMILY_SANS, false, false);
        }
        if let Some(p) = fc_match("sans-serif:bold") {
            load_variant(&mut ctx, Some(p.as_path()), FAMILY_SANS_BOLD, true, false);
        }
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
        weight: Some(if bold {
            FqWeight::BOLD
        } else {
            FqWeight::NORMAL
        }),
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
        Err(e) => eprintln!("twp-proxy: failed to load {}: {e:?}", path.display()),
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
    let family_key = if weight >= 700 {
        FAMILY_BOLD
    } else {
        FAMILY_REGULAR
    };
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
    let probe = TakumiNode::container(vec![text_node])
        .with_style(TkStyle::default().with(StyleDeclaration::display(Display::Flex)));
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
    write_image(Cow::Owned(img), &mut buf, ImageOutputFormat::Png, None).expect("png encode");
    buf
}

/// Raw bytes for an `img` node, decoded from base64 (`t=d`) or read from a
/// file (`t=f`). Returns None if no source resolves.
fn img_source_bytes(img: &Img) -> Option<Vec<u8>> {
    let medium = img
        .transmission
        .as_deref()
        .unwrap_or(if img.data.is_some() { "d" } else { "f" });
    match medium {
        "d" => STANDARD.decode(img.data.as_deref()?.trim()).ok(),
        "f" => std::fs::read(img.path.as_deref()?).ok(),
        _ => None,
    }
}

/// Build a takumi image node from an `img` node's Kitty-style source. PNG/
/// encoded formats are decoded by takumi; raw RGBA/RGB are wrapped directly.
/// On any failure the node degrades to an empty styled box so the surrounding
/// widget still renders.
fn build_img(node: &Node, style: TkStyle) -> TakumiNode {
    let Some(img) = &node.img else {
        return TakumiNode::container(vec![]).with_style(style);
    };
    let Some(bytes) = img_source_bytes(img) else {
        eprintln!("twp-proxy: img node has no resolvable source");
        return TakumiNode::container(vec![]).with_style(style);
    };

    let format = img.format.unwrap_or(100);
    let data: Option<ImageData> = match format {
        // Raw RGBA pixels.
        32 => match (img.data_width, img.data_height) {
            (Some(w), Some(h)) => RgbaImage::from_raw(w, h, bytes).map(ImageData::from),
            _ => None,
        },
        // Raw RGB pixels — expand to RGBA.
        24 => match (img.data_width, img.data_height) {
            (Some(w), Some(h)) => {
                let mut rgba = Vec::with_capacity(bytes.len() / 3 * 4);
                for px in bytes.chunks_exact(3) {
                    rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
                }
                RgbaImage::from_raw(w, h, rgba).map(ImageData::from)
            }
            _ => None,
        },
        // PNG or any other encoded format — takumi decodes the buffer.
        _ => Some(ImageData::from(bytes)),
    };

    match data {
        Some(d) => TakumiNode::image(d).with_style(style),
        None => {
            eprintln!("twp-proxy: img node has invalid pixel data (f={format})");
            TakumiNode::container(vec![]).with_style(style)
        }
    }
}

/// Apply a literal CSS declaration string to a style. Used for the internal
/// positioning of `stack` layers (position/inset/z-index). Parse failures are
/// logged and skipped.
fn apply_css_str(mut style: TkStyle, css: &str) -> TkStyle {
    match css.parse::<StyleDeclarationBlock>() {
        Ok(block) => {
            for decl in block.iter() {
                style = style.with(decl.clone());
            }
        }
        Err(_) => eprintln!("twp-proxy: internal css failed to parse: {css}"),
    }
    style
}

/// Build a `stack` node: its children are painted as full-bleed layers in
/// array order (later = on top). The stack only provides overlap + z-order;
/// any positioning *within* a layer is done with a flex inside it. The layers
/// fill the stack's cell box — a native terminal would composite them using
/// the graphics protocol's own z-index.
fn build_stack(node: &Node) -> TakumiNode {
    let container = apply_css_str(build_style(node), "position: relative");
    let layers: Vec<TakumiNode> = node
        .c
        .iter()
        .enumerate()
        .map(|(i, child)| {
            let inner = to_takumi(child);
            let wrap = apply_css_str(
                TkStyle::default(),
                &format!(
                    "position: absolute; top: 0; left: 0; width: 100%; height: 100%; z-index: {i}"
                ),
            );
            TakumiNode::container(vec![inner]).with_style(wrap)
        })
        .collect();
    TakumiNode::container(layers).with_style(container)
}

/// Build an `svg` node: the SVG markup lives in the node's `t` field (it's
/// text, so it can go inline — a shell script can `printf` it). takumi parses
/// and rasterizes it via resvg into the node's box. On parse failure the node
/// degrades to an empty styled box.
fn build_svg(node: &Node, style: TkStyle) -> TakumiNode {
    // Resolve `term(...)` tokens in the markup (self-delimiting → safe), then
    // parse. `currentColor` inside the SVG resolves to the node's `color`
    // (set e.g. via `color: term(fg)`), handled natively by resvg/takumi.
    let svg = substitute_term(node.t.as_deref().unwrap_or(""));
    match svg.parse::<SvgSource>() {
        Ok(src) => {
            let source: ImageSource = src.into();
            TakumiNode::image(ImageData::from(source)).with_style(style)
        }
        Err(_) => {
            eprintln!("twp-proxy: svg node failed to parse");
            TakumiNode::container(vec![]).with_style(style)
        }
    }
}

fn to_takumi(node: &Node) -> TakumiNode {
    let style = build_style(node);
    match node.n.as_str() {
        "text" => {
            let text = node.t.as_deref().unwrap_or("");
            TakumiNode::text(text).with_style(style)
        }
        "mono" => build_mono(node),
        "img" => build_img(node, style),
        "svg" => build_svg(node, style),
        "stack" => build_stack(node),
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
    // Parse each property on its own so one unparsable value is dropped in
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
        // Resolve `term(...)` palette tokens and `m`-cell units before takumi
        // parses the value.
        let css = format!("{key}: {}", substitute_cell_units(&substitute_term(&val)));
        match css.parse::<StyleDeclarationBlock>() {
            Ok(block) => decls.extend(block.iter().cloned()),
            Err(_) => eprintln!("twp-proxy: ignoring invalid CSS property: {css}"),
        }
    }
    decls
}

/// Resolve a colour value for CSS property `prop` (e.g. `"background-color"`,
/// `"color"`) into a takumi declaration via its full colour parser, after
/// `term()` substitution. This is the fallback for the typed `color` /
/// `background` fields when our simple `parse_color` doesn't recognise the
/// value — enabling *derived* colours like `color-mix(in srgb, term(2) 18%,
/// term(bg))` and relative `rgb(from term(1) r g b / .3)` in those fields.
fn color_decl(prop: &str, value: &str) -> Option<StyleDeclaration> {
    let css = format!("{prop}: {}", substitute_term(value));
    css.parse::<StyleDeclarationBlock>()
        .ok()
        .and_then(|block| block.iter().next().cloned())
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
    let family_key = if weight >= 700 {
        FAMILY_BOLD
    } else {
        FAMILY_REGULAR
    };
    // Foreground as a declaration: fast path via parse_color (term/hex), else
    // takumi's colour parser for derived colours (color-mix(), relative rgb()).
    let fg_decl: Option<StyleDeclaration> = s.color.as_deref().and_then(|v| {
        parse_color(v)
            .map(|c| StyleDeclaration::color(ColorInput::from(c)))
            .or_else(|| color_decl("color", v))
    });

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
                .with(StyleDeclaration::font_feature_settings(disable_ligatures()));
            if let Ok(ff) = FontFamily::from_str(&format!("\"{family_key}\"")) {
                char_style = char_style.with(StyleDeclaration::font_family(ff));
            }
            // Don't set font-weight — we already selected the correct
            // font file via family_key. Requesting weight 700 on a font
            // whose internal metadata says 400 makes parley fail to match.
            if let Some(d) = &fg_decl {
                char_style = char_style.with(d.clone());
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
    if let Some(v) = s.background.as_deref() {
        if let Some(bg) = parse_color(v) {
            outer_style =
                outer_style.with(StyleDeclaration::background_color(ColorInput::from(bg)));
        } else if let Some(d) = color_decl("background-color", v) {
            outer_style = outer_style.with(d);
        }
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
        let len = to_length_zero(gap);
        style = style
            .with(StyleDeclaration::column_gap(len))
            .with(StyleDeclaration::row_gap(len));
    }
    if let Some(p) = s.padding {
        let len = to_length_zero(p);
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
    if let Some(v) = s.background.as_deref() {
        if let Some(bg) = parse_color(v) {
            style = style.with(StyleDeclaration::background_color(ColorInput::from(bg)));
        } else if let Some(d) = color_decl("background-color", v) {
            style = style.with(d);
        }
    }
    if let Some(v) = s.color.as_deref() {
        if let Some(c) = parse_color(v) {
            style = style.with(StyleDeclaration::color(ColorInput::from(c)));
        } else if let Some(d) = color_decl("color", v) {
            style = style.with(d);
        }
    }
    if let Some(r) = s.border_radius {
        let pair = SpacePair::from_single(to_length_zero(r));
        style = style
            .with(StyleDeclaration::border_top_left_radius(pair))
            .with(StyleDeclaration::border_top_right_radius(pair))
            .with(StyleDeclaration::border_bottom_right_radius(pair))
            .with(StyleDeclaration::border_bottom_left_radius(pair));
    }
    // `border` / `border-*` are real CSS border properties; they flow through
    // the CSS value path (term()/cell-unit substitution → takumi's native
    // border, which is border-box, per-side capable, and supports line styles).

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
        let family_key = if weight >= 700 {
            FAMILY_BOLD
        } else {
            FAMILY_REGULAR
        };
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

/// Resolve a cell unit (mcw/mch) to pixels against the live cell size; returns
/// `None` for px/percent (handled by the callers below).
fn cell_unit_px(d: Dimension) -> Option<f32> {
    let (pc, pr) = (px_per_col() as f32, px_per_row() as f32);
    match d {
        Dimension::ColWidth(v) => Some(v * pc),
        Dimension::RowHeight(v) => Some(v * pr),
        _ => None,
    }
}

fn to_length(d: Dimension) -> Length {
    if let Some(px) = cell_unit_px(d) {
        return Length::Px(px);
    }
    match d {
        Dimension::Px(v) => Length::Px(v),
        Dimension::Percent(v) => Length::Percentage(v),
        _ => unreachable!("cell units handled above"),
    }
}

fn to_length_zero(d: Dimension) -> LengthDefaultsToZero {
    if let Some(px) = cell_unit_px(d) {
        return LengthDefaultsToZero::Px(px);
    }
    match d {
        Dimension::Px(v) => LengthDefaultsToZero::Px(v),
        Dimension::Percent(v) => LengthDefaultsToZero::Percentage(v),
        _ => unreachable!("cell units handled above"),
    }
}

fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("transparent") {
        return Some(Color::from([0, 0, 0, 0]));
    }
    if let Some([r, g, b]) = resolve_term(s) {
        return Some(Color::from([r, g, b, 255]));
    }
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
    use crate::expand::expand;
    use crate::protocol::Payload;

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
