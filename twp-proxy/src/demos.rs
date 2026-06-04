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
        markdown_doc(),
        contribution_heatmap(),
        bar_chart(),
        chat_bubbles(),
        wikipedia_article(),
        profile_card(),
        image_gallery(),
        diff_review(),
        svg_line_chart(),
        svg_donut(),
        svg_gauge(),
        term_palette(),
        term_themed_card(),
        svg_themed_chart(),
    ]
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
    let title = json!({"n":"mono","t":"responses by status","s":{"color":"term(fg)","font-weight":"bold"}});
    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","gap":8,"padding":14,"width":"100%","height":"100%","background":"transparent"},
        "c":[title, chart, legend]
    }});
    Demo { name: "svg_themed_chart", category: "term", cols: 38, rows: 10, scene }
}

/// Native-vs-TWP palette comparison. The shell prints a `native` row of real
/// ANSI background swatches (`\e[48;5;Nm`); the TWP widget renders the same 16
/// colors via `term(0)`…`term(15)` directly beneath it, aligned, so you can
/// see they match. Proves the proxy's queried palette equals the terminal's.
/// (The native row is emitted by `native_prefix`.)
fn term_palette() -> Demo {
    let swatch = |i: u32| json!({"n":"box","s":{"width":18,"height":22,"background":format!("term({i})")}});
    let row: Vec<Value> = (0..16).map(swatch).collect();
    let twp = json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","width":"100%","height":"100%","background":"term(bg)"},"c":[
        json!({"n":"mono","t":"twp     ","s":{"color":"term(fg)"}}),
        json!({"n":"flex","s":{"flex-direction":"row","gap":0},"c":row})
    ]});
    Demo { name: "term_palette", category: "term", cols: 40, rows: 2, scene: json!({"S": twp}) }
}

/// Native terminal output to emit *before* a widget, for side-by-side
/// comparison. Only `term_palette` uses it: a row of 16 ANSI background
/// swatches matching the TWP widget's `term()` swatches.
pub fn native_prefix(name: &str) -> Option<String> {
    if name != "term_palette" {
        return None;
    }
    let mut s = String::from("printf 'native  '; ");
    for i in 0..16 {
        s.push_str(&format!("printf '\\033[48;5;{i}m  \\033[0m'; "));
    }
    s.push_str("printf '\\n'");
    Some(s)
}

/// A status card with a **transparent** background (so the terminal shows
/// through) and theme colors (`term(1)`/`term(2)`/`term(3)`) — it reads as a
/// native part of the terminal rather than a pasted rectangle.
fn term_themed_card() -> Demo {
    let dot = |c: &str| json!({"n":"box","s":{"width":10,"height":10,"border-radius":5,"background":c}});
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
    Demo { name: "term_themed_card", category: "term", cols: 26, rows: 7, scene }
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
        dots.push_str(&format!("<circle cx='{x:.1}' cy='{y:.1}' r='3' fill='#38bdf8'/>"));
    }
    area.push_str(&format!(" L {:.1} {:.1} Z", pad + (n as f64 - 1.0) * dx, h - pad));

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
    Demo { name: "app_line_chart", category: "svg", cols: 40, rows: 9, scene }
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
    let svg = format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 120 120'>{paths}</svg>");
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
    Demo { name: "app_donut", category: "svg", cols: 40, rows: 9, scene }
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
    Demo { name: "app_gauge", category: "svg", cols: 30, rows: 9, scene }
}

/// A code-review diff: a syntax-tinted hunk (added / removed / context lines,
/// GitHub-dark colours) with an inline review comment — a real avatar
/// (Grace Hopper's public-domain Navy portrait) beside a message bubble that
/// casts a drop shadow.
fn diff_review() -> Demo {
    const AVATAR: &[u8] = include_bytes!("../demo/assets/grace_hopper.jpg");
    let code_fg = "#adbac7";
    let sans = "twp-sans";
    let sans_b = "twp-sans-b";

    // (gutter, kind, sign, code)
    let lines: Vec<(&str, &str, &str, &str)> = vec![
        ("41", "ctx", " ", "fn px_per_col() -> u32 {"),
        ("42", "del", "-", "    CELL_PX.get().map(|c| c.0).unwrap_or(20)"),
        ("42", "add", "+", "    CELL_PX.get().map(|c| c.0).unwrap_or(DEFAULT_PX_PER_COL)"),
        ("43", "ctx", " ", "}"),
    ];
    let diff_rows: Vec<Value> = lines
        .iter()
        .map(|(gutter, kind, sign, code)| {
            let (bg, sign_color) = match *kind {
                "add" => ("#12261c", "#3fb950"),
                "del" => ("#2d1214", "#f85149"),
                _ => ("#0d1117", "#6e7681"),
            };
            json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","gap":10,"width":"100%","background":bg,"padding":2},"c":[
                json!({"n":"mono","t":gutter,"s":{"color":"#6e7681"}}),
                json!({"n":"flex","s":{"flex-direction":"row"},"c":[
                    json!({"n":"mono","t":sign,"s":{"color":sign_color,"font-weight":"bold"}}),
                    json!({"n":"mono","t":code,"s":{"color":code_fg}})
                ]})
            ]})
        })
        .collect();

    let file_header = json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","padding":6,"background":"#161b22","border-radius":6},"c":[
        json!({"n":"mono","t":"src/render.rs","s":{"color":"#adbac7"}})
    ]});
    let diff_box = json!({"n":"flex","s":{"flex-direction":"column","gap":1,"padding":4,"background":"#0d1117","border-radius":6,"border":{"width":1,"color":"#30363d"}},"c":diff_rows});

    // Inline review comment: avatar (with online dot) + shadowed bubble.
    let avatar = avatar_with_status(STANDARD.encode(AVATAR), 36, "#3fb950", "#0d1117");
    let author = json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","gap":6},"c":[
        json!({"n":"text","t":"Grace Hopper","s":{"color":"#f0f6fc","font-size":13,"font-family":sans_b,"letter-spacing":"0px"}}),
        json!({"n":"text","t":"reviewed 2h ago","s":{"color":"#8b949e","font-size":11,"font-family":sans,"letter-spacing":"0px"}})
    ]});
    let comment_text = json!({"n":"text","t":"Good — a named constant beats a magic number. Ship it.","s":{"color":"#c9d1d9","font-size":13,"font-family":sans,"letter-spacing":"0px"}});
    let bubble = json!({"n":"flex","s":{"flex-direction":"column","gap":5,"padding":10,"background":"#2d333b","border-radius":10,"border":{"width":1,"color":"#444c56"},"box-shadow":"0 8px 22px #000000aa","max-width":"62%"},"c":[author, comment_text]});
    let comment = json!({"n":"flex","s":{"flex-direction":"row","align-items":"start","gap":10},"c":[avatar, bubble]});

    // The comment floats *over* the diff (a review popover), positioned by a
    // flex overlay inside a stack — anchored near the changed line, its shadow
    // lifting it off the code.
    let floating = json!({"n":"flex","s":{"flex-direction":"column","width":"100%","height":"100%","justify-content":"flex-start","align-items":"flex-start","padding-top":"62px","padding-left":"90px"},"c":[comment]});
    let stack = json!({"n":"stack","s":{"width":"100%","height":190},"c":[diff_box, floating]});

    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","gap":12,"padding":18,"width":"100%","height":"100%","background":"#0d1117"},
        "c":[file_header, stack]
    }});

    Demo { name: "app_diff_review", category: "mini-app", cols: 74, rows: 16, scene }
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

    Demo { name: "app_wikipedia", category: "mini-app", cols: 78, rows: 20, scene }
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

    Demo { name: "app_profile_card", category: "mini-app", cols: 34, rows: 6, scene }
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

    Demo { name: "app_gallery", category: "mini-app", cols: 66, rows: 8, scene }
}

/// A rendered Markdown document: heading hierarchy at real, different font
/// sizes, a bulleted list, a code block with its own background, and a
/// blockquote with an accent bar — none of which a terminal can do (it has
/// one font at one size).
fn markdown_doc() -> Demo {
    let head = "#e6edf3";
    let body = "#c9d1d9";
    let muted = "#8b949e";
    let accent = "#58a6ff";
    let str_c = "#a5d6ff";
    let fn_c = "#d2a8ff";
    // Proportional family registered by the renderer (falls back to a system
    // sans if unavailable). letter-spacing:0 neutralises the mono cell-grid
    // tuning that build_style applies to every text node.
    let sans = "twp-sans";
    let sans_b = "twp-sans-b";

    let h1 = json!({"n":"text","t":"Terminal Widget Protocol","s":{"color":head,"font-size":26,"font-family":sans_b,"letter-spacing":"0px"}});
    let lede = json!({"n":"text","t":"Render rich UI inline in your terminal — real font sizes, colour, and layout.","s":{"color":body,"font-size":15,"font-family":sans,"letter-spacing":"0px"}});

    let h2 = json!({"n":"text","t":"Features","s":{"color":head,"font-size":18,"font-family":sans_b,"letter-spacing":"0px"}});
    let divider = json!({"n":"box","s":{"width":"100%","height":1,"background":"#30363d"}});

    let bullet = |s: &str| {
        json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","gap":8},"c":[
            json!({"n":"box","s":{"width":5,"height":5,"border-radius":3,"background":accent}}),
            json!({"n":"text","t":s,"s":{"color":body,"font-size":14,"font-family":sans,"letter-spacing":"0px"}})
        ]})
    };
    let bullets = json!({"n":"flex","s":{"flex-direction":"column","gap":6},"c":[
        bullet("Proportional font sizes for headings and body"),
        bullet("Backgrounds, rounding, shadows, and gradients"),
        bullet("Flexbox layout for assembling real widgets"),
    ]});

    let code_line = |segs: Vec<(&str, &str)>| {
        let kids: Vec<Value> = segs
            .iter()
            .map(|(t, c)| json!({"n":"mono","t":t,"s":{"color":c}}))
            .collect();
        json!({"n":"flex","s":{"flex-direction":"row","height":20},"c":kids})
    };
    let code_block = json!({"n":"flex","s":{"flex-direction":"column","padding":10,"border-radius":8,"background":"#161b22"},"c":[
        code_line(vec![("twp ", fn_c), ("'v=1,c=8,r=2' ", muted)]),
        code_line(vec![("    '{\"S\":{\"n\":\"mono\",\"t\":", muted), ("\"OK\"", str_c), ("}}'", muted)]),
    ]});

    let quote = json!({"n":"flex","s":{"flex-direction":"row","align-items":"center","gap":10},"c":[
        json!({"n":"box","s":{"width":4,"height":18,"border-radius":2,"background":"#30363d"}}),
        json!({"n":"text","t":"Note: pure software rendering — works over SSH, no GPU required.","s":{"color":muted,"font-size":13,"font-family":sans,"letter-spacing":"0px"}})
    ]});

    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","gap":12,"padding":22,"width":"100%","height":"100%","background":"#0d1117"},
        "c":[h1, lede, h2, divider, bullets, code_block, quote]
    }});

    Demo { name: "app_markdown", category: "mini-app", cols: 66, rows: 22, scene }
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

    Demo { name: "app_code_minimap", category: "mini-app", cols: 76, rows: 18, scene }
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

    let title = json!({"n":"mono","t":"1,024 contributions in the last year","s":{"color":"#e6edf3"}});

    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","gap":10,"padding":16,"width":"100%","height":"100%","background":"#0d1117"},
        "c":[title, grid, legend]
    }});

    Demo { name: "app_heatmap", category: "mini-app", cols: 84, rows: 10, scene }
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

    Demo { name: "app_bar_chart", category: "mini-app", cols: 32, rows: 9, scene }
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

    Demo { name: "app_chat", category: "mini-app", cols: 40, rows: 10, scene }
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
