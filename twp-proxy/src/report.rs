use std::fs;
use std::io;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::compare::CompareResult;

pub struct TestEntry {
    pub name: String,
    pub result: CompareResult,
    pub native_png: Option<Vec<u8>>,
    pub twp_png: Option<Vec<u8>>,
    pub category: String,
    pub native_label: String,
}

pub fn generate_html(
    entries: &[TestEntry],
    font_info: &str,
    output: &Path,
) -> io::Result<()> {
    let pass = entries.iter().filter(|e| e.result.is_pass()).count();
    let fail = entries.len() - pass;

    let mut html = String::with_capacity(64 * 1024);

    html.push_str(r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8">
<title>TWP Visual Comparison</title>
<style>
  *{box-sizing:border-box;margin:0;padding:0}
  body{font-family:system-ui,sans-serif;background:#0f172a;color:#e2e8f0;padding:2rem}
  h1{margin-bottom:.5rem} .meta{color:#94a3b8;margin-bottom:2rem}
  .summary{display:flex;gap:1rem;margin-bottom:2rem}
  .badge{padding:.4rem 1rem;border-radius:8px;font-weight:bold;font-size:1.1rem}
  .pass-bg{background:#16a34a} .fail-bg{background:#dc2626} .skip-bg{background:#ca8a04}
  .test{background:#1e293b;border-radius:12px;padding:1.5rem;margin-bottom:1.5rem}
  .test h2{margin-bottom:.75rem;font-size:1.1rem}
  .status{display:inline-block;padding:.2rem .6rem;border-radius:4px;font-size:.85rem;font-weight:bold;margin-left:.5rem}
  .metrics{margin:.5rem 0;font-family:monospace;font-size:.9rem;color:#94a3b8}
  .images{display:grid;grid-template-columns:1fr 1fr;gap:1rem;margin-top:1rem}
  .img-box{background:#0f172a;border-radius:8px;padding:.75rem}
  .img-box h3{font-size:.75rem;color:#64748b;margin-bottom:.5rem;text-transform:uppercase;letter-spacing:.05em}
  .img-box img{width:100%;image-rendering:auto;border:1px solid #334155;border-radius:4px}
  .note{font-size:.85rem;color:#64748b;margin-bottom:1.5rem;line-height:1.5}
  h2.section{margin:1.5rem 0 1rem;font-size:1.3rem;border-bottom:1px solid #334155;padding-bottom:.5rem}
</style></head><body>
<h1>TWP Visual Comparison Report</h1>
"#);

    html.push_str(&format!(
        "<p class=\"meta\">{font_info}</p>\n"
    ));
    html.push_str("<p class=\"note\">Screenshots from Kitty running <code>twp-proxy</code> on a headless Xvfb display (llvmpipe software rendering). Captured via <code>twp-screenshot</code>. Basic tests compare native text vs TWP mono widget. Text-sizing tests compare Kitty OSC 66 output vs TWP mono with equivalent <code>scale</code>, <code>char-width</code>, and <code>subscale</code> parameters.</p>\n");

    html.push_str("<div class=\"summary\">\n");
    html.push_str(&format!(
        "<div class=\"badge pass-bg\">{pass} passed</div>\n"
    ));
    if fail > 0 {
        html.push_str(&format!(
            "<div class=\"badge fail-bg\">{fail} failed</div>\n"
        ));
    }
    html.push_str("</div>\n");

    let mut current_category = String::new();
    for entry in entries {
        if entry.category != current_category {
            current_category = entry.category.clone();
            let title = match current_category.as_str() {
                "basic" => "Basic mono (scale=1)",
                "text-sizing" => "Text-sizing (OSC 66 vs TWP)",
                "flex-mono" => "Flex + mono (manual text reference)",
                "flex-nested" => "Nested flex (tables / dashboards)",
                "css-effects" => "CSS text effects (no terminal equivalent)",
                "mini-ui" => "Mini UIs (flex + mono + effects)",
                "mini-app" => "Mini apps (minimap, heatmap, charts, chat)",
                "svg" => "Vector graphics (SVG — curves, arcs, gauges)",
                other => other,
            };
            html.push_str(&format!(
                "<h2 class=\"section\">{title}</h2>\n"
            ));
        }

        let showcase = matches!(
            entry.category.as_str(),
            "css-effects" | "mini-ui" | "mini-app" | "svg"
        );
        let summary = entry.result.summary();
        let status_word = summary.split_whitespace().next().unwrap_or("SKIP");
        let status_class = match status_word {
            "PASS" => "pass-bg",
            "FAIL" => "fail-bg",
            _ => "skip-bg",
        };

        // Showcase entries have no comparison metric — show just the status.
        let metrics = if showcase { String::new() } else {
            format!("<p class=\"metrics\">{summary}</p>")
        };
        html.push_str(&format!(
            "<div class=\"test\">\n  <h2>{} <span class=\"status {status_class}\">{status_word}</span></h2>\n  {metrics}\n  <div class=\"images\">\n",
            entry.name
        ));

        // Showcase: single TWP pane (no native reference). Otherwise the
        // usual native-vs-TWP pair.
        let twp_label = "TWP mono".to_string();
        let panes: Vec<(&String, &Option<Vec<u8>>)> = if showcase {
            vec![(&twp_label, &entry.twp_png)]
        } else {
            vec![
                (&entry.native_label, &entry.native_png),
                (&twp_label, &entry.twp_png),
            ]
        };
        for (label, png_data) in panes {
            html.push_str(&format!("<div class=\"img-box\"><h3>{label}</h3>\n"));
            if let Some(data) = png_data {
                let b64 = STANDARD.encode(data);
                html.push_str(&format!(
                    "<img src=\"data:image/png;base64,{b64}\">\n"
                ));
            } else {
                html.push_str("<p style=\"color:#64748b\">not available</p>\n");
            }
            html.push_str("</div>\n");
        }

        html.push_str("</div></div>\n");
    }

    html.push_str("</body></html>\n");
    fs::write(output, html)
}
