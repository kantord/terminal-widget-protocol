// Generated showcase demos — small "apps" that combine flex layout, mono
// text, and CSS effects. These are render-only (no terminal reference to
// diff against); they exist to demonstrate UIs that are awkward or
// impossible in a plain terminal but fall out naturally here.
//
// Payloads are built with serde_json rather than hand-written escaped
// strings: a minimap or heatmap is a repetitive grid that's far clearer to
// generate in a loop.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

pub struct Demo {
    pub name: &'static str,
    pub category: &'static str,
    pub cols: u32,
    pub rows: u32,
    pub scene: Value,
}

pub fn generated_demos() -> Vec<Demo> {
    vec![
        code_minimap(),
        contribution_heatmap(),
        bar_chart(),
        chat_bubbles(),
        wikipedia_article(),
        profile_card(),
        image_gallery(),
        svg_line_chart(),
        svg_donut(),
        svg_gauge(),
        term_themed_card(),
        svg_themed_chart(),
        now_playing_bar(),
    ]
}

/// Demonstrates `flex-grow` (which has no typed `Style` field — it rides the
/// CSS passthrough). A media bar: a fixed play glyph and a fixed time label
/// pin the ends, while the progress track in the middle carries `flex-grow:1`
/// so it absorbs *all* the remaining width. The canonical "fill the rest" flex
/// idiom that `justify-content` alone can't express.
fn now_playing_bar() -> Demo {
    let play = json!({"n":"mono","t":"▶","s":{"color":"#38bdf8","font-weight":"bold"}});
    let time = json!({"n":"mono","t":"1:23 / 3:40","s":{"color":"#94a3b8"}});
    // The track grows; inside it a filled sub-bar shows ~40% progress.
    let track = json!({"n":"flex","s":{
        "flex-grow":1,
        "height":8,
        "background":"#334155",
        "border-radius":4,
        "align-items":"center"
    },"c":[
        json!({"n":"box","s":{"width":"40%","height":8,"background":"#38bdf8","border-radius":4}})
    ]});
    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"row","align-items":"center","gap":12,"padding":14,
             "width":"100%","height":"100%","background":"#0f172a"},
        "c":[play, track, time]
    }});
    Demo {
        name: "now_playing_bar",
        category: "mini-ui",
        cols: 34,
        rows: 3,
        scene,
    }
}

/// SVG that uses terminal colors: bars filled with `term()` palette colors and
/// a baseline stroked in `currentColor` (which resolves to the terminal fg via
/// the node's `color: term(fg)`). The whole chart adapts to the user's theme,
/// on a transparent card that blends with the terminal.
fn svg_themed_chart() -> Demo {
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 210 88'>\
        <rect x='2' y='6'  width='180' height='16' rx='4' fill='term(2)'/>\
        <rect x='2' y='30' width='120' height='16' rx='4' fill='term(4)'/>\
        <rect x='2' y='54' width='78'  height='16' rx='4' fill='term(1)'/>\
        <line x1='2' y1='2' x2='2' y2='78' stroke='currentColor' stroke-width='2'/>\
        </svg>"
        .to_string();
    let chart = json!({"n":"svg","t":svg,"s":{"width":250,"height":104,"color":"term(fg)"}});
    let legend = json!({"n":"flex","s":{"flex-direction":"row","gap":12},"c":[
        json!({"n":"mono","t":"2xx","s":{"color":"term(2)"}}),
        json!({"n":"mono","t":"3xx","s":{"color":"term(4)"}}),
        json!({"n":"mono","t":"5xx","s":{"color":"term(1)"}})
    ]});
    let title =
        json!({"n":"mono","t":"responses by status","s":{"color":"term(fg)","font-weight":"bold"}});
    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","gap":8,"padding":14,"width":"100%","height":"100%","background":"transparent"},
        "c":[title, chart, legend]
    }});
    Demo {
        name: "svg_themed_chart",
        category: "term",
        cols: 38,
        rows: 10,
        scene,
    }
}

/// A native-vs-TWP comparison: a native terminal command (captured in bare
/// kitty) shown beside a TWP widget (captured through the proxy), in separate
/// windows. They can't share one — placeholder images and printed text don't
/// coexist on screen — so the report places the two screenshots side by side.
pub struct Comparison {
    pub name: String,
    pub label: String,
    pub category: &'static str,
    pub native_cmd: String,
    pub native_cols: u32,
    pub native_rows: u32,
    pub twp_cols: u32,
    pub twp_rows: u32,
    pub twp_scene: Value,
    pub theme: Theme,
}

/// A colour theme installed one-off for a single capture session: the 16 ANSI
/// palette entries plus default fg/bg. Applied to kitty via `--override`s so
/// the *same* widget code renders differently per theme.
#[derive(Clone)]
pub struct Theme {
    pub name: &'static str,
    pub bg: &'static str,
    pub fg: &'static str,
    pub ansi: [&'static str; 16],
}

/// A few well-known palettes. Each is a real terminal theme so the comparison
/// shows recognisable colours reacting to the session theme.
pub fn themes() -> Vec<Theme> {
    vec![
        Theme {
            name: "Gruvbox Dark",
            bg: "#282828",
            fg: "#ebdbb2",
            ansi: [
                "#282828", "#cc241d", "#98971a", "#d79921", "#458588", "#b16286", "#689d6a",
                "#a89984", "#928374", "#fb4934", "#b8bb26", "#fabd2f", "#83a598", "#d3869b",
                "#8ec07c", "#ebdbb2",
            ],
        },
        Theme {
            name: "Dracula",
            bg: "#282a36",
            fg: "#f8f8f2",
            ansi: [
                "#21222c", "#ff5555", "#50fa7b", "#f1fa8c", "#bd93f9", "#ff79c6", "#8be9fd",
                "#f8f8f2", "#6272a4", "#ff6e6e", "#69ff94", "#ffffa5", "#d6acff", "#ff92df",
                "#a4ffff", "#ffffff",
            ],
        },
        Theme {
            name: "Solarized Light",
            bg: "#fdf6e3",
            fg: "#657b83",
            ansi: [
                "#073642", "#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198",
                "#eee8d5", "#002b36", "#cb4b16", "#586e75", "#657b83", "#839496", "#6c71c4",
                "#93a1a1", "#fdf6e3",
            ],
        },
        Theme {
            name: "Nord",
            bg: "#2e3440",
            fg: "#d8dee9",
            ansi: [
                "#3b4252", "#bf616a", "#a3be8c", "#ebcb8b", "#81a1c1", "#b48ead", "#88c0d0",
                "#e5e9f0", "#4c566a", "#bf616a", "#a3be8c", "#ebcb8b", "#81a1c1", "#b48ead",
                "#8fbcbb", "#eceff4",
            ],
        },
    ]
}

pub fn comparison_demos() -> Vec<Comparison> {
    themes().into_iter().map(term_palette_comparison).collect()
}

/// Native ANSI palette swatches (`\e[48;5;Nm`) beside the same 16 colors drawn
/// by TWP via `term(0)`…`term(15)`. Both resolve to the session's palette, so
/// the rows match color-for-color — and because we run this once per `theme`,
/// the report shows the *same* widget reacting to each terminal theme, proving
/// the proxy queries (rather than hardcodes) the palette.
fn term_palette_comparison(theme: Theme) -> Comparison {
    // Native: two rows of 8 ANSI background swatches (8 cells each, 2 lines tall
    // via a leading blank line per row) — sized so the PNG clears the capture
    // floor.
    let mut native = String::from("printf '\\n'; ");
    for i in 0..16 {
        native.push_str(&format!("printf '\\033[48;5;{i}m        \\033[0m'; "));
        if i == 7 {
            native.push_str("printf '\\n\\n'; ");
        }
    }
    native.push_str("printf '\\n'");

    // TWP: the same 16 colors via term(N), 2 rows of 8 to match.
    let swatch =
        |i: u32| json!({"n":"box","s":{"width":52,"height":40,"background":format!("term({i})")}});
    let row = |range: std::ops::Range<u32>| json!({"n":"flex","s":{"flex-direction":"row","gap":2},"c":range.map(swatch).collect::<Vec<_>>()});
    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","gap":4,"justify-content":"center","align-items":"start","width":"100%","height":"100%","background":"term(bg)","padding":6},
        "c":[row(0..8), row(8..16)]
    }});

    let slug = theme.name.to_lowercase().replace(' ', "_");
    Comparison {
        name: format!("term_palette_{slug}"),
        label: theme.name.to_string(),
        category: "term-compare",
        native_cmd: native,
        native_cols: 70,
        native_rows: 5,
        twp_cols: 48,
        twp_rows: 5,
        twp_scene: scene,
        theme,
    }
}

/// A status card with a **transparent** background (so the terminal shows
/// through) and theme colors (`term(1)`/`term(2)`/`term(3)`) — it reads as a
/// native part of the terminal rather than a pasted rectangle.
fn term_themed_card() -> Demo {
    let dot =
        |c: &str| json!({"n":"box","s":{"width":10,"height":10,"border-radius":5,"background":c}});
    let row = |c: &str, label: &str| {
        json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","gap":8},"c":[
            dot(c), json!({"n":"mono","t":label,"s":{"color":"term(fg)"}})
        ]})
    };
    let rows = json!({"n":"flex","s":{"flex-direction":"column","gap":6},"c":[
        row("term(2)", "build    passed"),
        row("term(3)", "lint     warnings"),
        row("term(1)", "deploy   failed"),
    ]});
    let title = json!({"n":"mono","t":"CI status","s":{"color":"term(fg)","font-weight":"bold"}});
    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","gap":8,"padding":14,"width":"100%","height":"100%","background":"transparent"},
        "c":[title, rows]
    }});
    Demo {
        name: "term_themed_card",
        category: "term",
        cols: 26,
        rows: 7,
        scene,
    }
}

/// An `svg` node carrying inline SVG markup (it's text — a shell script could
/// `printf` this).
fn svg_node(svg: String, w: u32, h: u32) -> Value {
    json!({"n":"svg","t":svg,"s":{"width":w,"height":h}})
}

/// A line chart with an area-fill gradient and point markers — curves that
/// flex/box fundamentally can't draw, described as SVG and rasterized by the
/// terminal.
fn svg_line_chart() -> Demo {
    let data = [12.0, 28.0, 18.0, 41.0, 33.0, 54.0, 44.0, 62.0, 49.0, 68.0];
    let (w, h, pad, maxv) = (300.0, 140.0, 10.0_f64, 75.0);
    let n = data.len();
    let dx = (w - 2.0 * pad) / (n as f64 - 1.0);
    let pt = |i: usize, v: f64| -> (f64, f64) {
        (pad + i as f64 * dx, h - pad - (v / maxv) * (h - 2.0 * pad))
    };

    let mut line = String::new();
    let mut area = format!("M {:.1} {:.1}", pad, h - pad);
    let mut dots = String::new();
    for (i, &v) in data.iter().enumerate() {
        let (x, y) = pt(i, v);
        if i == 0 {
            line.push_str(&format!("M {x:.1} {y:.1}"));
        } else {
            line.push_str(&format!(" L {x:.1} {y:.1}"));
        }
        area.push_str(&format!(" L {x:.1} {y:.1}"));
        dots.push_str(&format!(
            "<circle cx='{x:.1}' cy='{y:.1}' r='3' fill='#38bdf8'/>"
        ));
    }
    area.push_str(&format!(
        " L {:.1} {:.1} Z",
        pad + (n as f64 - 1.0) * dx,
        h - pad
    ));

    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 {w} {h}'>\
         <defs><linearGradient id='g' x1='0' y1='0' x2='0' y2='1'>\
         <stop offset='0' stop-color='#38bdf8' stop-opacity='0.45'/>\
         <stop offset='1' stop-color='#38bdf8' stop-opacity='0'/></linearGradient></defs>\
         <path d='{area}' fill='url(#g)'/>\
         <path d='{line}' fill='none' stroke='#38bdf8' stroke-width='2.5' stroke-linejoin='round' stroke-linecap='round'/>\
         {dots}</svg>"
    );

    let title = json!({"n":"text","t":"Latency p95 (ms)","s":{"color":"#e2e8f0","font-size":15,"font-family":"twp-sans-b","letter-spacing":"0px"}});
    let chart = svg_node(svg, 300, 140);
    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","gap":8,"padding":16,"width":"100%","height":"100%","background":"#0f172a"},
        "c":[title, chart]
    }});
    Demo {
        name: "app_line_chart",
        category: "svg",
        cols: 40,
        rows: 9,
        scene,
    }
}

/// A donut chart (arc segments) beside a flex legend — SVG drawing composed
/// inside TWP layout, the web's HTML+SVG division.
fn svg_donut() -> Demo {
    let segs = [
        ("Rust", 45.0, "#fb923c"),
        ("Go", 25.0, "#38bdf8"),
        ("Lua", 18.0, "#a78bfa"),
        ("Other", 12.0, "#34d399"),
    ];
    let total: f64 = segs.iter().map(|s| s.1).sum();
    let (cx, cy, r_out, r_in) = (60.0_f64, 60.0_f64, 52.0_f64, 30.0_f64);
    let mut a = -std::f64::consts::FRAC_PI_2; // start at top

    let mut paths = String::new();
    for (_, v, color) in &segs {
        let sweep = v / total * std::f64::consts::TAU;
        let a1 = a + sweep;
        let large = if sweep > std::f64::consts::PI { 1 } else { 0 };
        let (x0, y0) = (cx + r_out * a.cos(), cy + r_out * a.sin());
        let (x1, y1) = (cx + r_out * a1.cos(), cy + r_out * a1.sin());
        let (xi1, yi1) = (cx + r_in * a1.cos(), cy + r_in * a1.sin());
        let (xi0, yi0) = (cx + r_in * a.cos(), cy + r_in * a.sin());
        paths.push_str(&format!(
            "<path d='M {x0:.2} {y0:.2} A {r_out} {r_out} 0 {large} 1 {x1:.2} {y1:.2} \
             L {xi1:.2} {yi1:.2} A {r_in} {r_in} 0 {large} 0 {xi0:.2} {yi0:.2} Z' fill='{color}'/>"
        ));
        a = a1;
    }
    let svg =
        format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 120 120'>{paths}</svg>");
    let donut = svg_node(svg, 120, 120);

    let legend_rows: Vec<Value> = segs
        .iter()
        .map(|(name, v, color)| {
            json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","gap":6},"c":[
                json!({"n":"box","s":{"width":10,"height":10,"border-radius":3,"background":*color}}),
                json!({"n":"mono","t":format!("{name}  {v:.0}%"),"s":{"color":"#cbd5e1"}})
            ]})
        })
        .collect();
    let legend = json!({"n":"flex","s":{"flex-direction":"column","gap":7,"justify-content":"center"},"c":legend_rows});

    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"row","align-items":"center","gap":20,"padding":16,"width":"100%","height":"100%","background":"#0f172a"},
        "c":[donut, legend]
    }});
    Demo {
        name: "app_donut",
        category: "svg",
        cols: 40,
        rows: 9,
        scene,
    }
}

/// A speedometer-style gauge: a track arc, a value arc, and a needle — pure
/// SVG arcs/rotation, impossible with boxes.
fn svg_gauge() -> Demo {
    let value = 0.72; // 0..1
    let (cx, cy, r) = (70.0_f64, 70.0_f64, 56.0_f64);
    // 180° sweep from left (180°) to right (0°), i.e. the top half.
    let start = std::f64::consts::PI;
    let end = start - value * std::f64::consts::PI;
    let polar = |ang: f64| (cx + r * ang.cos(), cy - r * ang.sin());
    let (tx0, ty0) = polar(std::f64::consts::PI);
    let (tx1, ty1) = polar(0.0);
    let (vx0, vy0) = polar(start);
    let (vx1, vy1) = polar(end);
    let large = if value > 0.5 { 0 } else { 0 }; // semicircle arcs, always 0
    let (nx, ny) = {
        let nr = r - 10.0;
        (cx + nr * end.cos(), cy - nr * end.sin())
    };
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 140 90'>\
         <path d='M {tx0:.1} {ty0:.1} A {r} {r} 0 {large} 1 {tx1:.1} {ty1:.1}' fill='none' stroke='#1e293b' stroke-width='12' stroke-linecap='round'/>\
         <path d='M {vx0:.1} {vy0:.1} A {r} {r} 0 {large} 1 {vx1:.1} {vy1:.1}' fill='none' stroke='#34d399' stroke-width='12' stroke-linecap='round'/>\
         <line x1='{cx}' y1='{cy}' x2='{nx:.1}' y2='{ny:.1}' stroke='#e2e8f0' stroke-width='3' stroke-linecap='round'/>\
         <circle cx='{cx}' cy='{cy}' r='5' fill='#e2e8f0'/></svg>"
    );
    let gauge = svg_node(svg, 180, 116);
    let label = json!({"n":"mono","t":"72%","s":{"color":"#34d399","font-weight":"bold"}});
    let caption = json!({"n":"text","t":"Disk usage","s":{"color":"#94a3b8","font-size":13,"font-family":"twp-sans","letter-spacing":"0px"}});
    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","align-items":"center","gap":4,"padding":14,"width":"100%","height":"100%","background":"#0f172a"},
        "c":[gauge, label, caption]
    }});
    Demo {
        name: "app_gauge",
        category: "svg",
        cols: 30,
        rows: 9,
        scene,
    }
}

/// A demo plus the terminal theme to render it under — captured one-off with
/// that palette installed, so the *same* scene re-tones itself to each theme.
pub struct ThemedDemo {
    pub name: String,
    pub label: String,
    pub category: &'static str,
    pub cols: u32,
    pub rows: u32,
    pub scene: Value,
    pub theme: Theme,
}

/// Real, sensible UIs whose every colour is *derived from the terminal palette*
/// — each rendered once per theme to prove the theming holds up on a full app,
/// not just solid swatches. The Docker dashboard is the flagship: gauges,
/// sparklines, a gradient area chart, status badges and a container table, all
/// re-toned by the session palette.
pub fn themed_demos() -> Vec<ThemedDemo> {
    // (base name, cols, rows, scene builder)
    let builders: Vec<(&str, u32, u32, fn() -> Value)> = vec![
        ("docker_dashboard", 92, 24, docker_dashboard_scene),
        ("diff_review", 74, 16, diff_review_scene),
    ];
    let mut out = Vec::new();
    for (name, cols, rows, build) in builders {
        for theme in themes() {
            let slug = theme.name.to_lowercase().replace(' ', "_");
            out.push(ThemedDemo {
                name: format!("{name}_{slug}"),
                label: theme.name.to_string(),
                category: "term-themed",
                cols,
                rows,
                scene: build(),
                theme,
            });
        }
    }
    out
}

// ── Docker dashboard ──────────────────────────────────────────────

/// A semicircular gauge as SVG, `value` in 0..1. `fill` is any colour the SVG
/// parser accepts after term() substitution (so `term(2)` etc. work); the track
/// uses `term(8)` (the palette's "bright black"), keeping it theme-derived.
fn gauge_svg(value: f64, fill: &str) -> String {
    let (cx, cy, r) = (70.0_f64, 70.0_f64, 52.0_f64);
    let start = std::f64::consts::PI;
    let end = start - value.clamp(0.0, 1.0) * std::f64::consts::PI;
    let polar = |a: f64| (cx + r * a.cos(), cy - r * a.sin());
    let (tx0, ty0) = polar(start);
    let (tx1, ty1) = polar(0.0);
    let (vx1, vy1) = polar(end);
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 140 80'>\
         <path d='M {tx0:.1} {ty0:.1} A {r} {r} 0 0 1 {tx1:.1} {ty1:.1}' fill='none' stroke='term(8)' stroke-width='11' stroke-linecap='round'/>\
         <path d='M {tx0:.1} {ty0:.1} A {r} {r} 0 0 1 {vx1:.1} {vy1:.1}' fill='none' stroke='{fill}' stroke-width='11' stroke-linecap='round'/>\
         </svg>"
    )
}

/// A sparkline polyline as SVG over `data`, scaled to fit `w`×`h`.
fn sparkline_svg(data: &[f64], stroke: &str, w: f64, h: f64) -> String {
    let n = data.len().max(2);
    let max = data.iter().cloned().fold(0.0_f64, f64::max).max(1e-6);
    let dx = w / (n as f64 - 1.0);
    let pts: String = data
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = i as f64 * dx;
            let y = h - (v / max) * (h - 3.0) - 1.5;
            format!("{}{x:.1},{y:.1}", if i == 0 { "" } else { " " })
        })
        .collect();
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 {w:.0} {h:.0}'>\
         <polyline points='{pts}' fill='none' stroke='{stroke}' stroke-width='1.5' stroke-linejoin='round' stroke-linecap='round'/></svg>"
    )
}

/// Deterministic pseudo-history (no RNG): a smooth-ish wave seeded per row.
fn fake_series(seed: u32, base: f64, amp: f64, n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| {
            let t = i as f64 * 0.6 + seed as f64 * 1.7;
            (base + amp * (t.sin() * 0.6 + (t * 0.5).cos() * 0.4)).clamp(0.02, 1.0)
        })
        .collect()
}

/// **Flagship demo.** A Docker monitoring dashboard — the kind of thing people
/// open Docker Desktop / Portainer or fall back to plain `docker stats` for.
/// Combines flex-grow layout, SVG gauges + sparklines + a gradient area chart,
/// status badges and a container table. Every colour derives from the terminal
/// palette (`term()` solids, `color-mix()` surfaces), so it re-tones per theme.
fn docker_dashboard_scene() -> Value {
    let editor = "term(bg)";
    let surface = "color-mix(in srgb, term(fg) 6%, term(bg))";
    let border = "color-mix(in srgb, term(fg) 16%, term(bg))";
    let row_line = "color-mix(in srgb, term(fg) 9%, term(bg))";
    let muted = "color-mix(in srgb, term(fg) 50%, term(bg))";
    let dim = "color-mix(in srgb, term(fg) 38%, term(bg))";
    let fg = "term(fg)";
    let green = "term(2)";
    let yellow = "term(3)";
    let red = "term(1)";
    let blue = "term(4)";
    let cyan = "term(6)";

    let sans = "twp-sans";

    // Sizes are in monospace cell units (mcw = cell widths, mch = cell heights,
    // mcmin = min(w,h) for squares). The whole dashboard therefore aligns to the
    // character grid at *any* terminal's cell size, not just the one it was
    // drawn at. px remains only for sub-cell cosmetics (1px borders).
    // ── Header: logo + title, flex-grow spacer, cluster summary ──
    let logo = json!({"n":"box","s":{"width":"1.5mcmin","height":"1.5mcmin","border-radius":"0.45mcmin","background":blue}});
    let title = json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","gap":"0.6mcw"},"c":[
        logo,
        json!({"n":"mono","t":"docker","s":{"color":fg,"font-weight":"bold"}}),
        json!({"n":"mono","t":"prod-cluster","s":{"color":muted}})
    ]});
    let stat = |label: &str, value: &str, color: &str| {
        json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","gap":"0.4mcw"},"c":[
            json!({"n":"box","s":{"width":"0.6mcmin","height":"0.6mcmin","border-radius":"0.3mcmin","background":color}}),
            json!({"n":"mono","t":value,"s":{"color":fg}}),
            json!({"n":"mono","t":label,"s":{"color":muted}})
        ]})
    };
    let spacer = json!({"n":"box","s":{"flex-grow":1}});
    let summary = json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","gap":"1.4mcw"},"c":[
        stat("running", "6", green),
        stat("restarting", "1", yellow),
        stat("exited", "1", red)
    ]});
    let header = json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","width":"100%","gap":"0.9mcw"},"c":[title, spacer, summary]});

    // ── Stat cards: two gauges + a gradient network area chart ──
    let gauge_card = |value: f64, fill: &str, label: &str| {
        let pct = format!("{}%", (value * 100.0).round() as i64);
        json!({"n":"flex","s":{"flex-direction":"column","align-items":"center","justify-content":"center","gap":"0.1mch","width":"11.5mcw","height":"4.8mch","background":surface,"border-radius":"0.75mcmin","border-width":"1px","border-style":"solid","border-color":border,"padding":"0.6mcw"},"c":[
            json!({"n":"svg","t":gauge_svg(value, fill),"s":{"width":"9.2mcw","height":"2.2mch"}}),
            json!({"n":"mono","t":pct,"s":{"color":fill,"font-weight":"bold"}}),
            json!({"n":"text","t":label,"s":{"color":muted,"font-size":12,"font-family":sans,"letter-spacing":"0px"}})
        ]})
    };
    let net = fake_series(2, 0.45, 0.4, 28);
    let nmax = net.iter().cloned().fold(0.0_f64, f64::max).max(1e-6);
    let (nw, nh) = (320.0_f64, 70.0_f64);
    let ndx = nw / (net.len() as f64 - 1.0);
    let mut line = String::new();
    let mut area = format!("M 0 {nh:.0}");
    for (i, &v) in net.iter().enumerate() {
        let x = i as f64 * ndx;
        let y = nh - (v / nmax) * (nh - 6.0) - 3.0;
        if i == 0 {
            line.push_str(&format!("M {x:.1} {y:.1}"));
        } else {
            line.push_str(&format!(" L {x:.1} {y:.1}"));
        }
        area.push_str(&format!(" L {x:.1} {y:.1}"));
    }
    area.push_str(&format!(" L {nw:.0} {nh:.0} Z"));
    let net_svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 {nw:.0} {nh:.0}' preserveAspectRatio='none'>\
         <defs><linearGradient id='ng' x1='0' y1='0' x2='0' y2='1'>\
         <stop offset='0' stop-color='term(4)' stop-opacity='0.5'/>\
         <stop offset='1' stop-color='term(4)' stop-opacity='0'/></linearGradient></defs>\
         <path d='{area}' fill='url(#ng)'/>\
         <path d='{line}' fill='none' stroke='term(4)' stroke-width='2' stroke-linejoin='round'/></svg>"
    );
    let net_card = json!({"n":"flex","s":{"flex-direction":"column","flex-grow":1,"gap":"0.2mch","height":"4.8mch","background":surface,"border-radius":"0.75mcmin","border-width":"1px","border-style":"solid","border-color":border,"padding":"0.75mcw"},"c":[
        json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","gap":"0.6mcw"},"c":[
            json!({"n":"text","t":"Network I/O","s":{"color":muted,"font-size":12,"font-family":sans,"letter-spacing":"0px"}}),
            json!({"n":"mono","t":"↑ 4.2MB/s","s":{"color":blue}})
        ]}),
        json!({"n":"svg","t":net_svg,"s":{"width":"100%","height":"2.4mch"}})
    ]});
    let cards = json!({"n":"flex","s":{"flex-direction":"row","gap":"0.9mcw","width":"100%"},"c":[
        gauge_card(0.34, blue, "CPU"), gauge_card(0.61, cyan, "Memory"), net_card
    ]});

    // ── Container table ──
    // (name, image, status, cpu, mem, uptime, spark-seed)
    let rows: Vec<(&str, &str, &str, f64, f64, &str, u32)> = vec![
        (
            "api-gateway",
            "nginx:1.27",
            "running",
            0.12,
            0.34,
            "4d 2h",
            1,
        ),
        (
            "postgres",
            "postgres:16",
            "running",
            0.41,
            0.62,
            "12d 6h",
            3,
        ),
        ("redis", "redis:7.2", "running", 0.06, 0.18, "12d 6h", 5),
        ("worker-1", "app:2.3.1", "running", 0.78, 0.55, "3h 11m", 7),
        ("scheduler", "app:2.3.1", "running", 0.22, 0.30, "5d 9h", 9),
        ("worker-2", "app:2.3.1", "restarting", 0.0, 0.0, "—", 11),
        ("legacy-batch", "app:1.9.4", "exited", 0.0, 0.0, "—", 13),
    ];
    let status_color = |s: &str| match s {
        "running" => green,
        "restarting" => yellow,
        _ => red,
    };
    // Text columns are padded mono strings — one glyph per cell, so they align
    // on the grid for free. Graphical columns are fixed *cell-unit* widths, so
    // they line up with the text columns at any cell size. Same widths in
    // header + body keep the grid consistent.
    let (cpu_w, trend_w, status_w) = ("10.8mcw", "6.2mcw", "11.5mcw");
    let gcell = |w: &str, child: Value| json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","width":w},"c":[child]});
    let hlabel = |t: &str| json!({"n":"mono","t":t,"s":{"color":dim}});

    let header_row = json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","gap":"0.9mcw","width":"100%","padding":"0.45mcw"},"c":[
        json!({"n":"flex","s":{"width":"0.9mcw"}}),
        hlabel(&format!("{:<14}", "CONTAINER")),
        hlabel(&format!("{:<13}", "IMAGE")),
        gcell(cpu_w, hlabel("CPU")),
        gcell(trend_w, hlabel("MEM")),
        gcell(status_w, hlabel("STATUS")),
        hlabel("UPTIME")
    ]});

    let body_rows: Vec<Value> = rows
        .iter()
        .map(|(name, image, status, cpu, mem, uptime, seed)| {
            let sc = status_color(status);
            let dot = json!({"n":"box","s":{"width":"0.77mcmin","height":"0.77mcmin","border-radius":"0.4mcmin","background":sc}});
            let bar_fill = if *cpu < 0.5 { green } else if *cpu < 0.8 { yellow } else { red };
            let cpu_cell = gcell(cpu_w, json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","gap":"0.6mcw"},"c":[
                json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","width":"5.4mcw","height":"0.25mch","background":row_line,"border-radius":"0.3mcmin"},"c":[
                    json!({"n":"box","s":{"width":format!("{}%", (cpu*100.0).round() as i64),"height":"0.25mch","background":bar_fill,"border-radius":"0.3mcmin"}})
                ]}),
                json!({"n":"mono","t":format!("{:>3}%", (cpu*100.0).round() as i64),"s":{"color":muted}})
            ]}));
            let running = *status == "running";
            let series = if running {
                fake_series(*seed, *mem, 0.25, 16)
            } else {
                vec![0.04_f64; 16] // stopped → flat line
            };
            let spark = sparkline_svg(&series, if running { cyan } else { dim }, 70.0, 18.0);
            // Badge sizes to its (padded) content so the pill always wraps the
            // word; the surrounding status cell is fixed-width for alignment.
            let badge = json!({"n":"flex","s":{"justify-content":"center","align-items":"center","background":format!("color-mix(in srgb, {sc} 18%, term(bg))"),"border-radius":"0.45mcmin","padding-top":"0.1mch","padding-bottom":"0.1mch","padding-left":"0.5mcw","padding-right":"0.5mcw"},"c":[
                json!({"n":"mono","t":*status,"s":{"color":sc}})
            ]});
            json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","gap":"0.9mcw","width":"100%","padding":"0.45mcw","border-top-width":"1px","border-top-style":"solid","border-top-color":row_line},"c":[
                json!({"n":"flex","s":{"width":"0.9mcw","justify-content":"center"},"c":[dot]}),
                json!({"n":"mono","t":format!("{:<14}", name),"s":{"color":fg}}),
                json!({"n":"mono","t":format!("{:<13}", image),"s":{"color":muted}}),
                cpu_cell,
                gcell(trend_w, json!({"n":"svg","t":spark,"s":{"width":"5.7mcw","height":"0.7mch"}})),
                gcell(status_w, badge),
                json!({"n":"mono","t":format!("{:<8}", uptime),"s":{"color":dim}})
            ]})
        })
        .collect();

    let mut table_children = vec![header_row];
    table_children.extend(body_rows);
    let table = json!({"n":"flex","s":{"flex-direction":"column","width":"100%","background":surface,"border-radius":"0.75mcmin","border-width":"1px","border-style":"solid","border-color":border,"padding":"0.3mcw"},"c":table_children});

    json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","gap":"0.45mch","padding":"1.2mcw","width":"100%","height":"100%","background":editor},
        "c":[header, cards, table]
    }})
}

/// A code-review diff (added / removed / context lines + an inline review
/// comment from Grace Hopper's public-domain Navy portrait).
///
/// Every colour is derived from the terminal palette — no hardcoded hex:
///   * solid roles use `term()` directly (added = `term(2)`, removed =
///     `term(1)`, text = `term(fg)`);
///   * tones in between are *computed* with `color-mix()` — an added-line
///     background is the green accent mixed a little into the editor
///     background, "muted" text is the foreground pulled toward the
///     background, panels/borders are subtle lifts off `term(bg)`.
/// Re-tones itself to any palette with zero per-theme code.
fn diff_review_scene() -> Value {
    const AVATAR: &[u8] = include_bytes!("../demo/assets/grace_hopper.jpg");
    let sans = "twp-sans";
    let sans_b = "twp-sans-b";

    // Derived design tokens. `color-mix(in srgb, A p%, B)` = p% of A over B.
    let editor = "term(bg)";
    let surface = "color-mix(in srgb, term(fg) 7%, term(bg))"; // raised panel
    let surface2 = "color-mix(in srgb, term(fg) 13%, term(bg))"; // popover
    let border = "color-mix(in srgb, term(fg) 22%, term(bg))";
    let muted = "color-mix(in srgb, term(fg) 45%, term(bg))"; // gutter / meta
    let body = "color-mix(in srgb, term(fg) 80%, term(bg))"; // comment body
    let fg = "term(fg)";
    let add = "term(2)";
    let del = "term(1)";
    let add_bg = "color-mix(in srgb, term(2) 15%, term(bg))";
    let del_bg = "color-mix(in srgb, term(1) 15%, term(bg))";

    // (gutter, kind, sign, code)
    let lines: Vec<(&str, &str, &str, &str)> = vec![
        ("41", "ctx", " ", "fn px_per_col() -> u32 {"),
        (
            "42",
            "del",
            "-",
            "    CELL_PX.get().map(|c| c.0).unwrap_or(20)",
        ),
        (
            "42",
            "add",
            "+",
            "    CELL_PX.get().map(|c| c.0).unwrap_or(DEFAULT_PX_PER_COL)",
        ),
        ("43", "ctx", " ", "}"),
    ];
    let diff_rows: Vec<Value> = lines
        .iter()
        .map(|(gutter, kind, sign, code)| {
            let (bg, sign_color) = match *kind {
                "add" => (add_bg, add),
                "del" => (del_bg, del),
                _ => (editor, muted),
            };
            json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","gap":"0.77mcw","width":"100%","background":bg,"padding":"0.15mcw"},"c":[
                json!({"n":"mono","t":gutter,"s":{"color":muted}}),
                json!({"n":"flex","s":{"flex-direction":"row"},"c":[
                    json!({"n":"mono","t":sign,"s":{"color":sign_color,"font-weight":"bold"}}),
                    json!({"n":"mono","t":code,"s":{"color":fg}})
                ]})
            ]})
        })
        .collect();

    let file_header = json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","padding":"0.45mcw","background":surface,"border-radius":"0.45mcmin"},"c":[
        json!({"n":"mono","t":"src/render.rs","s":{"color":body}})
    ]});
    // Border colour is derived (color-mix), so it goes through the CSS
    // passthrough longhands rather than the typed `border` (which only takes a
    // plain colour).
    let diff_box = json!({"n":"flex","s":{"flex-direction":"column","gap":"0.05mch","padding":"0.3mcw","background":editor,"border-radius":"0.45mcmin","border-width":"1px","border-style":"solid","border-color":border},"c":diff_rows});

    // Inline review comment: avatar (with online dot) + shadowed bubble. The
    // dot's ring is the editor background so it reads as cut into the surface.
    let avatar = avatar_with_status(STANDARD.encode(AVATAR), 36, add, editor);
    let author = json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","gap":"0.45mcw"},"c":[
        json!({"n":"text","t":"Grace Hopper","s":{"color":fg,"font-size":13,"font-family":sans_b,"letter-spacing":"0px"}}),
        json!({"n":"text","t":"reviewed 2h ago","s":{"color":muted,"font-size":11,"font-family":sans,"letter-spacing":"0px"}})
    ]});
    let comment_text = json!({"n":"text","t":"Good — a named constant beats a magic number. Ship it.","s":{"color":body,"font-size":13,"font-family":sans,"letter-spacing":"0px"}});
    let bubble = json!({"n":"flex","s":{"flex-direction":"column","gap":"0.35mch","padding":"0.75mcw","background":surface2,"border-radius":"0.75mcmin","border-width":"1px","border-style":"solid","border-color":border,"box-shadow":"0 8px 22px #00000055","max-width":"62%"},"c":[author, comment_text]});
    let comment = json!({"n":"flex","s":{"flex-direction":"row","align-items":"start","gap":"0.77mcw"},"c":[avatar, bubble]});

    // The comment floats *over* the diff (a review popover), positioned by a
    // flex overlay inside a stack — anchored near the changed line, its shadow
    // lifting it off the code.
    let floating = json!({"n":"flex","s":{"flex-direction":"column","width":"100%","height":"100%","justify-content":"flex-start","align-items":"flex-start","padding-top":"2.14mch","padding-left":"6.9mcw"},"c":[comment]});
    let stack =
        json!({"n":"stack","s":{"width":"100%","height":"6.55mch"},"c":[diff_box, floating]});

    json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","gap":"0.45mch","padding":"1.4mcw","width":"100%","height":"100%","background":editor},
        "c":[file_header, stack]
    }})
}

/// A Wikipedia-style article: a title, wrapping prose in a text column, and a
/// bordered image infobox with a real (public-domain) JPEG and caption. Merges
/// the Markdown renderer with the `img` node — proportional text, an embedded
/// photograph, and multi-column layout in one terminal widget. The asset is
/// embedded at compile time so the demo stays self-contained.
fn wikipedia_article() -> Demo {
    const PHOTO: &[u8] = include_bytes!("../demo/assets/charros.jpg");
    let b64 = STANDARD.encode(PHOTO);

    let head = "#e6edf3";
    let body = "#c9d1d9";
    let muted = "#8b949e";
    let sans = "twp-sans";
    let sans_b = "twp-sans-b";

    let title = json!({"n":"text","t":"Charrería","s":{"color":head,"font-size":24,"font-family":sans_b,"letter-spacing":"0px"}});
    let divider = json!({"n":"box","s":{"width":"100%","height":1,"background":"#30363d"}});

    // Left column: wrapping prose (the text node wraps to the column width).
    let lead = json!({"n":"text","t":"Charrería is a competitive equestrian sport that originated in Mexico. It grew out of the everyday work of charros — ranch horsemen — and is widely regarded as the country's national sport.","s":{"color":body,"font-size":14,"font-family":sans,"letter-spacing":"0px"}});
    let h2 = json!({"n":"text","t":"Heritage","s":{"color":head,"font-size":17,"font-family":sans_b,"letter-spacing":"0px"}});
    let para = json!({"n":"text","t":"In 2016 UNESCO inscribed charrería on the Representative List of the Intangible Cultural Heritage of Humanity, recognising its place in Mexican identity and rural tradition.","s":{"color":body,"font-size":13,"font-family":sans,"letter-spacing":"0px"}});
    let left = json!({"n":"flex","s":{"flex-direction":"column","gap":9,"width":"54%"},"c":[lead, h2, para]});

    // Right column: a bordered infobox with the photograph and its caption.
    let photo = json!({"n":"img","s":{"width":226,"height":142,"border-radius":4},"img":{"d":b64}});
    let cap1 = json!({"n":"text","t":"Mexican Charros Roping a Bull","s":{"color":body,"font-size":11,"font-family":sans,"letter-spacing":"0px"}});
    let cap2 = json!({"n":"text","t":"Oil painting · public domain","s":{"color":muted,"font-size":10,"font-family":sans,"letter-spacing":"0px"}});
    let infobox = json!({"n":"flex","s":{"flex-direction":"column","gap":5,"padding":10,"width":250,"background":"#161b22","border-radius":6,"border":{"width":1,"color":"#30363d"}},"c":[photo, cap1, cap2]});

    let content = json!({"n":"flex","s":{"flex-direction":"row","gap":18,"align-items":"start","width":"100%"},"c":[left, infobox]});

    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","gap":12,"padding":22,"width":"100%","height":"100%","background":"#0d1117"},
        "c":[title, divider, content]
    }});

    Demo {
        name: "app_wikipedia",
        category: "mini-app",
        cols: 78,
        rows: 20,
        scene,
    }
}

/// An `img` node carrying inline base64 PNG data (Kitty `t=d`).
fn img_node(b64: String, w: u32, h: u32, radius: u32) -> Value {
    json!({
        "n":"img",
        "s":{"width":w,"height":h,"border-radius":radius},
        "img":{"d":b64}
    })
}

/// A circular avatar with a status dot overlapping its corner — a `stack` of
/// the image plus a flex-aligned dot. `ring` is the surrounding surface colour
/// so the dot reads as cut out of it.
fn avatar_with_status(b64: String, size: u32, dot: &str, ring: &str) -> Value {
    let avatar = img_node(b64, size, size, size / 2);
    let d = (size as f64 * 0.30).round();
    let dot = json!({"n":"box","s":{"width":d,"height":d,"border-radius":d/2.0,"background":dot,"border":{"width":2,"color":ring}}});
    let overlay = json!({"n":"flex","s":{"width":"100%","height":"100%","justify-content":"flex-end","align-items":"flex-end"},"c":[dot]});
    json!({"n":"stack","s":{"width":size,"height":size},"c":[avatar, overlay]})
}

/// A profile card: a circular avatar (Ada Lovelace's public-domain portrait)
/// beside proportional name/role text and a row of skill pills — img + flex +
/// mono + sans text together.
fn profile_card() -> Demo {
    const PORTRAIT: &[u8] = include_bytes!("../demo/assets/ada_lovelace.jpg");
    let avatar = avatar_with_status(STANDARD.encode(PORTRAIT), 72, "#22c55e", "#1e293b");

    let name = json!({"n":"text","t":"Ada Lovelace","s":{"color":"#f1f5f9","font-size":18,"font-family":"twp-sans-b","letter-spacing":"0px"}});
    let role = json!({"n":"text","t":"Mathematician · first programmer","s":{"color":"#94a3b8","font-size":14,"font-family":"twp-sans","letter-spacing":"0px"}});
    let pill = |t: &str, bg: &str| {
        json!({"n":"flex","s":{"justify-content":"center","align-items":"center","background":bg,"border-radius":8,"padding":4},
               "c":[json!({"n":"mono","t":t,"s":{"color":"#0f172a","font-weight":"bold"}})]})
    };
    let pills = json!({"n":"flex","s":{"flex-direction":"row","gap":6},"c":[pill(" Algorithms ", "#fbbf24"), pill(" 1843 ", "#34d399")]});
    let info = json!({"n":"flex","s":{"flex-direction":"column","gap":5},"c":[name, role, pills]});

    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"row","align-items":"center","gap":14,"padding":16,"width":"100%","height":"100%","background":"#1e293b","border-radius":14},
        "c":[avatar, info]
    }});

    Demo {
        name: "app_profile_card",
        category: "mini-app",
        cols: 34,
        rows: 6,
        scene,
    }
}

/// An image gallery: a row of rounded thumbnails of real photographs, each
/// credited to its photographer. Images are free under the Unsplash License
/// (see demo/assets/CREDITS.md) and embedded at compile time.
fn image_gallery() -> Demo {
    let sans = "twp-sans";
    let sans_b = "twp-sans-b";
    // (embedded JPEG, photographer) — free Unsplash License, attributed.
    let tiles: [(&[u8], &str); 4] = [
        (
            include_bytes!("../demo/assets/unsplash_pietro_de_grandi.jpg"),
            "Pietro De Grandi",
        ),
        (
            include_bytes!("../demo/assets/unsplash_simon_berger.jpg"),
            "Simon Berger",
        ),
        (
            include_bytes!("../demo/assets/unsplash_daniela_kokina.jpg"),
            "Daniela Kokina",
        ),
        (
            include_bytes!("../demo/assets/unsplash_sven_pieren.jpg"),
            "Sven Pieren",
        ),
    ];
    // Each tile is a `stack`: the photo, with a dark scrim + credit overlaid
    // on its lower edge (a Netflix/YouTube-style thumbnail caption).
    let cells: Vec<Value> = tiles
        .iter()
        .map(|(bytes, who)| {
            let thumb = img_node(STANDARD.encode(bytes), 124, 82, 8);
            let credit_band = json!({"n":"flex","s":{"flex-direction":"column","width":"100%","padding":5,"background":"#000000b3","border-bottom-left-radius":"8px","border-bottom-right-radius":"8px"},"c":[
                json!({"n":"text","t":*who,"s":{"color":"#ffffff","font-size":10,"font-family":sans,"letter-spacing":"0px"}}),
                json!({"n":"text","t":"Unsplash","s":{"color":"#cbd5e1","font-size":8,"font-family":sans,"letter-spacing":"0px"}})
            ]});
            let scrim = json!({"n":"flex","s":{"flex-direction":"column","justify-content":"flex-end","width":"100%","height":"100%"},"c":[credit_band]});
            json!({"n":"stack","s":{"width":124,"height":82},"c":[thumb, scrim]})
        })
        .collect();
    let strip = json!({"n":"flex","s":{"flex-direction":"row","gap":12},"c":cells});
    let title = json!({"n":"text","t":"Mountain landscapes","s":{"color":"#f1f5f9","font-size":16,"font-family":sans_b,"letter-spacing":"0px"}});

    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","gap":10,"padding":16,"width":"100%","height":"100%","background":"#0f172a"},
        "c":[title, strip]
    }});

    Demo {
        name: "app_gallery",
        category: "mini-app",
        cols: 66,
        rows: 8,
        scene,
    }
}

/// Sublime-style code panel + minimap. The code is mono text (a terminal
/// could print this with ANSI colour); the minimap — a zoomed-out column of
/// token-coloured bars — is the part a terminal can't draw.
fn code_minimap() -> Demo {
    let kw = "#c678dd"; // keyword (purple)
    let func = "#61afef"; // function (blue)
    let strc = "#98c379"; // string (green)
    let num = "#d19a66"; // number (orange)
    let txt = "#abb2bf"; // default

    // (indent, [(token, colour)])
    let lines: Vec<(usize, Vec<(&str, &str)>)> = vec![
        (0, vec![("def ", kw), ("parse", func), ("(tokens):", txt)]),
        (4, vec![("depth ", txt), ("= ", txt), ("0", num)]),
        (4, vec![("out ", txt), ("= ", txt), ("[]", txt)]),
        (0, vec![]),
        (
            4,
            vec![("for ", kw), ("tok ", txt), ("in ", kw), ("tokens:", txt)],
        ),
        (
            8,
            vec![("if ", kw), ("tok == ", txt), ("\"(\"", strc), (":", txt)],
        ),
        (12, vec![("depth ", txt), ("+= ", txt), ("1", num)]),
        (
            8,
            vec![("elif ", kw), ("tok == ", txt), ("\")\"", strc), (":", txt)],
        ),
        (12, vec![("depth ", txt), ("-= ", txt), ("1", num)]),
        (8, vec![("out", txt), (".append", func), ("(tok)", txt)]),
        (0, vec![]),
        (
            4,
            vec![
                ("if ", kw),
                ("depth ", txt),
                ("!= ", txt),
                ("0", num),
                (":", txt),
            ],
        ),
        (
            8,
            vec![
                ("raise ", kw),
                ("ValueError", func),
                ("(", txt),
                ("\"unbalanced\"", strc),
                (")", txt),
            ],
        ),
        (0, vec![]),
        (4, vec![("return ", kw), ("out", txt)]),
    ];

    // Code panel: each line is a mono row, indentation as leading spaces.
    let code_rows: Vec<Value> = lines
        .iter()
        .map(|(indent, segs)| {
            let mut kids: Vec<Value> = Vec::new();
            if *indent > 0 {
                kids.push(json!({"n":"mono","t":" ".repeat(*indent),"s":{"color":txt}}));
            }
            for (t, c) in segs {
                kids.push(json!({"n":"mono","t":t,"s":{"color":c}}));
            }
            json!({"n":"flex","s":{"flex-direction":"row","height":26},"c":kids})
        })
        .collect();
    let code_panel = json!({
        "n":"flex",
        "s":{"flex-direction":"column","padding":16,"background":"#282c34","width":"78%","height":"100%"},
        "c":code_rows
    });

    // Minimap: the real Sublime trick — the *actual letters*, just tiny.
    // Each token is a small `text` span at its syntax colour; indentation is
    // a transparent spacer so the code's shape is preserved.
    let map_fs = 7.0; // minimap font size (px)
    let map_cw = map_fs * 0.6; // approx char advance for the indent spacer
    let map_rows: Vec<Value> = lines
        .iter()
        .map(|(indent, segs)| {
            let mut kids: Vec<Value> = Vec::new();
            if *indent > 0 {
                let w = *indent as f64 * map_cw;
                kids.push(json!({"n":"box","s":{"width":w,"height":map_fs,"background":"#21252b"}}));
            }
            for (t, c) in segs {
                // Use non-breaking spaces so inter-token spacing survives the
                // text layout (separate flex spans otherwise collapse the
                // trailing space, gluing "def" and "parse" together).
                let t = t.replace(' ', "\u{00A0}");
                kids.push(json!({"n":"text","t":t,"s":{"color":c,"font-size":map_fs}}));
            }
            json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","height":10},"c":kids})
        })
        .collect();
    let minimap = json!({
        "n":"flex",
        "s":{"flex-direction":"column","padding":10,"gap":1,"background":"#21252b","width":"22%","height":"100%"},
        "c":map_rows
    });

    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"row","width":"100%","height":"100%","background":"#1b1d23"},
        "c":[code_panel, minimap]
    }});

    Demo {
        name: "app_code_minimap",
        category: "mini-app",
        cols: 76,
        rows: 18,
        scene,
    }
}

/// GitHub-style contribution heatmap: a grid of rounded, colour-graded
/// squares. Pure flex + box; deterministic intensities so it's stable.
fn contribution_heatmap() -> Demo {
    let levels = ["#161b22", "#0e4429", "#006d32", "#26a641", "#39d353"];
    let weeks = 52;
    let days = 7;

    let mut week_cols: Vec<Value> = Vec::with_capacity(weeks);
    for w in 0..weeks {
        let mut day_cells: Vec<Value> = Vec::with_capacity(days);
        for d in 0..days {
            // Deterministic pseudo-intensity — no RNG (forbidden in scripts,
            // and we want stable output).
            let v = (w * 3 + d * 5 + (w * d) % 11) % 5;
            day_cells.push(json!({
                "n":"box",
                "s":{"width":11,"height":11,"background":levels[v],"border-radius":2}
            }));
        }
        week_cols.push(json!({"n":"flex","s":{"flex-direction":"column","gap":3},"c":day_cells}));
    }
    let grid = json!({"n":"flex","s":{"flex-direction":"row","gap":3},"c":week_cols});

    let legend_cells: Vec<Value> = levels
        .iter()
        .map(|c| json!({"n":"box","s":{"width":11,"height":11,"background":c,"border-radius":2}}))
        .collect();
    let mut legend_row = vec![json!({"n":"mono","t":"less ","s":{"color":"#7d8590"}})];
    legend_row.extend(legend_cells);
    legend_row.push(json!({"n":"mono","t":" more","s":{"color":"#7d8590"}}));
    let legend = json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","gap":3},"c":legend_row});

    let title =
        json!({"n":"mono","t":"1,024 contributions in the last year","s":{"color":"#e6edf3"}});

    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","gap":10,"padding":16,"width":"100%","height":"100%","background":"#0d1117"},
        "c":[title, grid, legend]
    }});

    Demo {
        name: "app_heatmap",
        category: "mini-app",
        cols: 84,
        rows: 10,
        scene,
    }
}

/// A bar chart with rounded bars, value-dependent colour, and a baseline.
fn bar_chart() -> Demo {
    let heights = [34, 58, 42, 80, 66, 96, 52, 72, 38, 62, 88, 46];
    let bars: Vec<Value> = heights
        .iter()
        .map(|h| {
            let color = if *h >= 80 { "#fb923c" } else { "#38bdf8" };
            json!({"n":"box","s":{"width":18,"height":*h,"background":color,"border-radius":4}})
        })
        .collect();
    let chart = json!({
        "n":"flex",
        "s":{"flex-direction":"row","align-items":"flex-end","gap":7,"height":110},
        "c":bars
    });
    let baseline = json!({"n":"box","s":{"width":"100%","height":2,"background":"#334155"}});
    let title = json!({"n":"mono","t":"requests / sec","s":{"color":"#94a3b8"}});

    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","justify-content":"flex-end","gap":6,"padding":16,"width":"100%","height":"100%","background":"#0f172a"},
        "c":[title, chart, baseline]
    }});

    Demo {
        name: "app_bar_chart",
        category: "mini-app",
        cols: 32,
        rows: 9,
        scene,
    }
}

/// A chat thread with rounded message bubbles, incoming left / outgoing
/// right, distinct colours.
fn chat_bubbles() -> Demo {
    // (text, mine)
    let msgs: Vec<(&str, bool)> = vec![
        ("hey, is the build green?", false),
        ("yep, all 47 tests pass", true),
        ("ship it then", false),
        ("deploying now", true),
    ];

    let rows: Vec<Value> = msgs
        .iter()
        .map(|(t, mine)| {
            let bubble = json!({
                "n":"flex",
                "s":{
                    "padding":8,
                    "border-radius":12,
                    "background": if *mine {"#2563eb"} else {"#374151"},
                    "max-width":"72%"
                },
                "c":[json!({"n":"mono","t":t,"s":{"color":"#ffffff"}})]
            });
            let justify = if *mine { "flex-end" } else { "flex-start" };
            json!({
                "n":"flex",
                "s":{"flex-direction":"row","justify-content":justify,"width":"100%"},
                "c":[bubble]
            })
        })
        .collect();

    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","gap":6,"padding":12,"width":"100%","height":"100%","background":"#111827"},
        "c":rows
    }});

    Demo {
        name: "app_chat",
        category: "mini-app",
        cols: 40,
        rows: 10,
        scene,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expand::expand;
    use crate::protocol::Payload;
    use crate::render::render_to_png;

    // Each generated demo must parse as a valid payload and rasterize to a
    // non-blank image. This catches schema/JSON mistakes at `cargo test` time,
    // with no Xvfb or kitty — the structural counterpart to eyeballing the
    // screenshots.
    #[test]
    fn all_demos_render_nonblank() {
        for demo in generated_demos() {
            let payload: Payload = serde_json::from_value(demo.scene.clone())
                .unwrap_or_else(|e| panic!("demo {} is not a valid payload: {e}", demo.name));
            let scene = payload
                .scene
                .unwrap_or_else(|| panic!("demo {} has no scene", demo.name));
            let expanded = expand(scene, &payload.defs);
            let png = render_to_png(&expanded, demo.cols, demo.rows);
            let img = image::load_from_memory(&png)
                .unwrap_or_else(|e| panic!("demo {} produced invalid PNG: {e}", demo.name))
                .to_rgba8();
            let first = img.get_pixel(0, 0);
            let varied = img.pixels().any(|p| p != first);
            assert!(varied, "demo {} rendered blank", demo.name);
        }
    }
}
