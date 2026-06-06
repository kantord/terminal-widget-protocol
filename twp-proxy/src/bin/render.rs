// twp-render — render a TWP scene to a PNG *in-process* (no terminal, no Xvfb).
//
// This is the engine behind the figures and worked examples in RFC.md: the same
// JSON source is shown as a code block AND rendered to an image, so the spec's
// examples and pictures can never disagree. The pixels are identical to what the
// proxy transmits — the Kitty display path only *shows* them.
//
// Usage:
//   twp-render --in <scene.json> --cols N --rows N [--theme NAME] --out <png>
//   twp-render --demo <name> [--theme NAME] --out <png>
//
// `--in` renders a hand-authored scene file (a `{"S": …}` payload). `--demo`
// renders a scene from the demo registry by name (themed demos carry their own
// palette). `--theme` installs one of the named palettes from `demos::themes()`.

use std::fs;

use twp_proxy::{demos, expand, protocol, render};

fn parse_hex(s: &str) -> [u8; 3] {
    let h = s.trim().trim_start_matches('#');
    let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
    [byte(0), byte(2), byte(4)]
}

/// Install the named palette (substring match, case-insensitive) so `term(...)`
/// and `color-mix(...)` resolve to that theme.
fn set_theme(name: &str) {
    let needle = name.to_lowercase();
    let theme = demos::themes()
        .into_iter()
        .find(|t| t.name.to_lowercase().contains(&needle))
        .unwrap_or_else(|| panic!("unknown theme: {name}"));
    let base: [[u8; 3]; 16] = std::array::from_fn(|i| parse_hex(theme.ansi[i]));
    render::set_palette(render::palette_from_base(
        base,
        parse_hex(theme.fg),
        parse_hex(theme.bg),
    ));
}

fn render_payload(value: &serde_json::Value, cols: u32, rows: u32) -> Vec<u8> {
    let payload: protocol::Payload =
        serde_json::from_value(value.clone()).expect("payload is valid TWP JSON");
    let scene = payload.scene.expect("payload has a scene under \"S\"");
    let expanded = expand::expand(scene, &payload.defs);
    render::render_to_png(&expanded, cols, rows)
}

fn arg<'a>(args: &'a [String], i: &mut usize) -> &'a str {
    *i += 1;
    args.get(*i)
        .unwrap_or_else(|| panic!("missing value for {}", args[*i - 1]))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut in_path, mut out, mut demo, mut theme) = (None, None, None, None);
    let (mut cols, mut rows) = (None, None);

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--in" => in_path = Some(arg(&args, &mut i).to_string()),
            "--out" => out = Some(arg(&args, &mut i).to_string()),
            "--demo" => demo = Some(arg(&args, &mut i).to_string()),
            "--theme" => theme = Some(arg(&args, &mut i).to_string()),
            "--cols" => cols = Some(arg(&args, &mut i).parse().expect("--cols is a number")),
            "--rows" => rows = Some(arg(&args, &mut i).parse().expect("--rows is a number")),
            other => panic!("unknown argument: {other}"),
        }
        i += 1;
    }

    let out = out.expect("--out is required");

    let png = if let Some(name) = demo {
        if let Some(td) = demos::themed_demos().into_iter().find(|d| d.name == name) {
            set_theme(td.theme.name); // themed demo carries its own palette
            render_payload(&td.scene, td.cols, td.rows)
        } else if let Some(d) = demos::generated_demos()
            .into_iter()
            .find(|d| d.name == name.as_str())
        {
            if let Some(t) = &theme {
                set_theme(t);
            }
            render_payload(&d.scene, d.cols, d.rows)
        } else {
            panic!("unknown demo: {name}");
        }
    } else {
        let path = in_path.expect("--in or --demo is required");
        if let Some(t) = &theme {
            set_theme(t);
        }
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        let value: serde_json::Value =
            serde_json::from_str(&text).expect("scene file is valid JSON");
        render_payload(
            &value,
            cols.expect("--cols is required with --in"),
            rows.expect("--rows is required with --in"),
        )
    };

    fs::write(&out, &png).unwrap_or_else(|e| panic!("write {out}: {e}"));
    eprintln!("twp-render: wrote {out} ({} bytes)", png.len());
}
