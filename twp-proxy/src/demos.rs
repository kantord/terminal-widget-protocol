// Generated showcase demos — small "apps" that combine flex layout, mono
// text, and CSS effects. These are render-only (no terminal reference to
// diff against); they exist to demonstrate UIs that are awkward or
// impossible in a plain terminal but fall out naturally here.
//
// Payloads are built with serde_json rather than hand-written escaped
// strings: a minimap or heatmap is a repetitive grid that's far clearer to
// generate in a loop.

use std::io::Cursor;

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
    ]
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

/// A deterministic diagonal-gradient PNG, base64-encoded — stand-in artwork
/// for the `img` demos so they stay self-contained (no external files).
fn gradient_png_b64(w: u32, h: u32, c0: [u8; 3], c1: [u8; 3]) -> String {
    let mut img = image::RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let t = (x as f32 / w as f32 + y as f32 / h as f32) * 0.5;
            let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
            img.put_pixel(
                x,
                y,
                image::Rgba([mix(c0[0], c1[0]), mix(c0[1], c1[1]), mix(c0[2], c1[2]), 255]),
            );
        }
    }
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png)
        .expect("png encode");
    STANDARD.encode(buf.into_inner())
}

/// An `img` node carrying inline base64 PNG data (Kitty `t=d`).
fn img_node(b64: String, w: u32, h: u32, radius: u32) -> Value {
    json!({
        "n":"img",
        "s":{"width":w,"height":h,"border-radius":radius},
        "img":{"d":b64}
    })
}

/// A profile card: a circular avatar image beside proportional name/role text
/// and a row of skill pills — img + flex + mono + sans text together.
fn profile_card() -> Demo {
    let avatar = img_node(gradient_png_b64(96, 96, [124, 58, 237], [34, 211, 238]), 72, 72, 36);

    let name = json!({"n":"text","t":"Ada Lovelace","s":{"color":"#f1f5f9","font-size":18,"font-family":"twp-sans-b","letter-spacing":"0px"}});
    let role = json!({"n":"text","t":"Systems Engineer","s":{"color":"#94a3b8","font-size":14,"font-family":"twp-sans","letter-spacing":"0px"}});
    let pill = |t: &str, bg: &str| {
        json!({"n":"flex","s":{"justify-content":"center","align-items":"center","background":bg,"border-radius":8,"padding":4},
               "c":[json!({"n":"mono","t":t,"s":{"color":"#0f172a","font-weight":"bold"}})]})
    };
    let pills = json!({"n":"flex","s":{"flex-direction":"row","gap":6},"c":[pill(" Rust ", "#fbbf24"), pill(" TUI ", "#34d399")]});
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
    let cells: Vec<Value> = tiles
        .iter()
        .map(|(bytes, who)| {
            let thumb = img_node(STANDARD.encode(bytes), 124, 82, 8);
            let name = json!({"n":"text","t":*who,"s":{"color":"#cbd5e1","font-size":11,"font-family":sans,"letter-spacing":"0px"}});
            let credit = json!({"n":"text","t":"Unsplash","s":{"color":"#64748b","font-size":9,"font-family":sans,"letter-spacing":"0px"}});
            json!({"n":"flex","s":{"flex-direction":"column","gap":3,"align-items":"start","width":124},"c":[thumb, name, credit]})
        })
        .collect();
    let strip = json!({"n":"flex","s":{"flex-direction":"row","gap":12},"c":cells});
    let title = json!({"n":"text","t":"Mountain landscapes","s":{"color":"#f1f5f9","font-size":16,"font-family":sans_b,"letter-spacing":"0px"}});

    let scene = json!({"S":{
        "n":"flex",
        "s":{"flex-direction":"column","gap":10,"padding":16,"width":"100%","height":"100%","background":"#0f172a"},
        "c":[title, strip]
    }});

    Demo { name: "app_gallery", category: "mini-app", cols: 66, rows: 9, scene }
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
