use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use twp_proxy::compare::{self, CompareResult, TestStatus};
use twp_proxy::demos;
use twp_proxy::report::{self, TestEntry};

/// Wrap a generated widget scene into the `printf` command that emits its
/// TWP escape. The JSON is passed as a `%s` *argument*, not embedded in the
/// format string, so printf does no backslash/`%` processing on it — the
/// escaped quotes serde_json emits survive intact. Single quotes in the
/// payload are escaped for the surrounding bash single-quoting (`'\''`), so
/// the scene may contain any character.
fn demo_twp_cmd(cols: u32, rows: u32, scene: &serde_json::Value) -> String {
    let json = scene.to_string().replace('\'', "'\\''");
    format!("printf '\\x1b_twp;v=1,c={cols},r={rows};%s\\x1b\\\\' '{json}'")
}

// ── Xvfb session ──────────────────────────────────────────────────

struct XvfbSession {
    child: Option<Child>,
    display: String,
}

impl XvfbSession {
    fn ensure(display: &str) -> Result<Self, String> {
        if display_is_available(display) {
            return Ok(Self {
                child: None,
                display: display.to_string(),
            });
        }
        let child = Command::new("Xvfb")
            .args([
                display,
                "-screen",
                "0",
                "1920x1080x24",
                "+extension",
                "GLX",
                "+render",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("failed to start Xvfb: {e}"))?;

        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            if display_is_available(display) {
                return Ok(Self {
                    child: Some(child),
                    display: display.to_string(),
                });
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err("Xvfb started but display not available after 5s".to_string())
    }

    fn display(&self) -> &str {
        &self.display
    }
}

impl Drop for XvfbSession {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

// ── Capture config ────────────────────────────────────────────────

struct CaptureConfig {
    output: PathBuf,
    display: String,
    proxy: Option<String>,
    font: String,
    font_size: String,
    cols: u32,
    rows: u32,
    bg: String,
    fg: String,
    /// Extra kitty `--override` bodies (e.g. `color0=#282828`) applied for this
    /// capture — used to install a one-off colour theme per session.
    palette: Vec<String>,
    class: String,
    timeout: u64,
    command: Vec<String>,
}

// ── Test definitions ──────────────────────────────────────────────

struct TestCase {
    name: &'static str,
    text: &'static str,
    cols: u32,
    native_cmd: &'static str,
    twp_cmd: &'static str,
    native_uses_proxy: bool,
    category: &'static str,
}

const TESTS: &[TestCase] = &[
    // Basic mono (scale=1)
    TestCase {
        name: "letters",
        text: "ABCDEFGHIJ",
        cols: 10,
        native_cmd: "printf '%s' 'ABCDEFGHIJ'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=10,r=1;{\"S\":{\"n\":\"mono\",\"t\":\"ABCDEFGHIJ\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\\x1b\\\\'",
        native_uses_proxy: true,
        category: "basic",
    },
    TestCase {
        name: "pangram",
        text: "The quick brown fox",
        cols: 19,
        native_cmd: "printf '%s' 'The quick brown fox'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=19,r=1;{\"S\":{\"n\":\"mono\",\"t\":\"The quick brown fox\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\\x1b\\\\'",
        native_uses_proxy: true,
        category: "basic",
    },
    TestCase {
        name: "digits",
        text: "0123456789012345",
        cols: 16,
        native_cmd: "printf '%s' '0123456789012345'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=16,r=1;{\"S\":{\"n\":\"mono\",\"t\":\"0123456789012345\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\\x1b\\\\'",
        native_uses_proxy: true,
        category: "basic",
    },
    TestCase {
        name: "wide_M",
        text: "MMMMMMMMMMMMMMMMMMMM",
        cols: 20,
        native_cmd: "printf '%s' 'MMMMMMMMMMMMMMMMMMMM'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=20,r=1;{\"S\":{\"n\":\"mono\",\"t\":\"MMMMMMMMMMMMMMMMMMMM\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\\x1b\\\\'",
        native_uses_proxy: true,
        category: "basic",
    },
    TestCase {
        name: "mixed",
        text: "Hello world 12345",
        cols: 17,
        native_cmd: "printf '%s' 'Hello world 12345'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=17,r=1;{\"S\":{\"n\":\"mono\",\"t\":\"Hello world 12345\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\\x1b\\\\'",
        native_uses_proxy: true,
        category: "basic",
    },
    // Text-sizing (OSC 66 vs TWP)
    TestCase {
        name: "scale2",
        text: "ABCDE",
        cols: 10,
        native_cmd: "printf '\\x1b]66;s=2;ABCDE\\x07'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=10,r=2;{\"S\":{\"n\":\"mono\",\"t\":\"ABCDE\",\"s\":{\"scale\":2,\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\\x1b\\\\'",
        native_uses_proxy: false,
        category: "text-sizing",
    },
    TestCase {
        name: "scale3",
        text: "ABC",
        cols: 9,
        native_cmd: "printf '\\x1b]66;s=3;ABC\\x07'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=9,r=3;{\"S\":{\"n\":\"mono\",\"t\":\"ABC\",\"s\":{\"scale\":3,\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\\x1b\\\\'",
        native_uses_proxy: false,
        category: "text-sizing",
    },
    TestCase {
        name: "charw2",
        text: "ABCDE",
        cols: 10,
        native_cmd: "printf '\\x1b]66;w=2;ABCDE\\x07'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=10,r=1;{\"S\":{\"n\":\"mono\",\"t\":\"ABCDE\",\"s\":{\"char-width\":2,\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\\x1b\\\\'",
        native_uses_proxy: false,
        category: "text-sizing",
    },
    TestCase {
        name: "sub_half",
        text: "ABCDEFGHIJ",
        cols: 10,
        native_cmd: "printf '\\x1b]66;n=1:d=2;ABCDEFGHIJ\\x07'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=10,r=1;{\"S\":{\"n\":\"mono\",\"t\":\"ABCDEFGHIJ\",\"s\":{\"subscale-n\":1,\"subscale-d\":2,\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\\x1b\\\\'",
        native_uses_proxy: false,
        category: "text-sizing",
    },
    TestCase {
        name: "scale2_sub_half",
        text: "ABCDE",
        cols: 10,
        native_cmd: "printf '\\x1b]66;s=2:n=1:d=2;ABCDE\\x07'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=10,r=2;{\"S\":{\"n\":\"mono\",\"t\":\"ABCDE\",\"s\":{\"scale\":2,\"subscale-n\":1,\"subscale-d\":2,\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\\x1b\\\\'",
        native_uses_proxy: false,
        category: "text-sizing",
    },
    TestCase {
        name: "scale2_digits",
        text: "0123456789",
        cols: 20,
        native_cmd: "printf '\\x1b]66;s=2;0123456789\\x07'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=20,r=2;{\"S\":{\"n\":\"mono\",\"t\":\"0123456789\",\"s\":{\"scale\":2,\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}}\\x1b\\\\'",
        native_uses_proxy: false,
        category: "text-sizing",
    },
    // Flex + mono: justify-content distributes labels to exact cells.
    // The native reference uses printf with a precise number of spaces
    // that reproduces flex's computed gaps.
    //
    // space-between, 2 items in c=10: "AA" @ 0-1, "BB" @ 8-9 (6-cell gap)
    TestCase {
        name: "flex_between_2",
        text: "AA      BB",
        cols: 10,
        native_cmd: "printf '%s' 'AA      BB'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=10,r=1;{\"S\":{\"n\":\"flex\",\"s\":{\"flex-direction\":\"row\",\"justify-content\":\"space-between\",\"width\":\"100%%\",\"height\":\"100%%\",\"background\":\"#0a1e24\"},\"c\":[{\"n\":\"mono\",\"t\":\"AA\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}},{\"n\":\"mono\",\"t\":\"BB\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}]}}\\x1b\\\\'",
        native_uses_proxy: true,
        category: "flex-mono",
    },
    // space-between, 3 items in c=11: "A" @ 0, "B" @ 5, "C" @ 10 (4-cell gaps)
    TestCase {
        name: "flex_between_3",
        text: "A    B    C",
        cols: 11,
        native_cmd: "printf '%s' 'A    B    C'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=11,r=1;{\"S\":{\"n\":\"flex\",\"s\":{\"flex-direction\":\"row\",\"justify-content\":\"space-between\",\"width\":\"100%%\",\"height\":\"100%%\",\"background\":\"#0a1e24\"},\"c\":[{\"n\":\"mono\",\"t\":\"A\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}},{\"n\":\"mono\",\"t\":\"B\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}},{\"n\":\"mono\",\"t\":\"C\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}]}}\\x1b\\\\'",
        native_uses_proxy: true,
        category: "flex-mono",
    },
    // space-between, wide gap in c=20: "MMM" @ 0-2, "MMM" @ 17-19 (14-cell gap)
    TestCase {
        name: "flex_between_wide",
        text: "MMM              MMM",
        cols: 20,
        native_cmd: "printf '%s' 'MMM              MMM'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=20,r=1;{\"S\":{\"n\":\"flex\",\"s\":{\"flex-direction\":\"row\",\"justify-content\":\"space-between\",\"width\":\"100%%\",\"height\":\"100%%\",\"background\":\"#0a1e24\"},\"c\":[{\"n\":\"mono\",\"t\":\"MMM\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}},{\"n\":\"mono\",\"t\":\"MMM\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}]}}\\x1b\\\\'",
        native_uses_proxy: true,
        category: "flex-mono",
    },
    // space-between, uneven widths in c=8: "AAA" @ 0-2, "B" @ 7 (4-cell gap)
    TestCase {
        name: "flex_between_uneven",
        text: "AAA    B",
        cols: 8,
        native_cmd: "printf '%s' 'AAA    B'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=8,r=1;{\"S\":{\"n\":\"flex\",\"s\":{\"flex-direction\":\"row\",\"justify-content\":\"space-between\",\"width\":\"100%%\",\"height\":\"100%%\",\"background\":\"#0a1e24\"},\"c\":[{\"n\":\"mono\",\"t\":\"AAA\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}},{\"n\":\"mono\",\"t\":\"B\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}]}}\\x1b\\\\'",
        native_uses_proxy: true,
        category: "flex-mono",
    },
    // Vertical flex stacks mono lines into separate rows (multi-row composition)
    TestCase {
        name: "flex_col_stack",
        text: "ABCDE",
        cols: 5,
        native_cmd: "printf 'ABCDE\\nFGHIJ'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=5,r=2;{\"S\":{\"n\":\"flex\",\"s\":{\"flex-direction\":\"column\",\"width\":\"100%%\",\"height\":\"100%%\",\"background\":\"#0a1e24\"},\"c\":[{\"n\":\"mono\",\"t\":\"ABCDE\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}},{\"n\":\"mono\",\"t\":\"FGHIJ\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}]}}\\x1b\\\\'",
        native_uses_proxy: true,
        category: "flex-mono",
    },
    // Nested flex: column of space-between rows = a key-value table.
    // Each row: label left, value right. c=12, 3 rows.
    //   CPU      100
    //   MEM      050
    //   NET      012
    TestCase {
        name: "nested_kv_table",
        text: "CPU      100",
        cols: 12,
        native_cmd: "printf '%s\\n%s\\n%s' 'CPU      100' 'MEM      050' 'NET      012'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=12,r=3;{\"S\":{\"n\":\"flex\",\"s\":{\"flex-direction\":\"column\",\"width\":\"100%%\",\"height\":\"100%%\",\"background\":\"#0a1e24\"},\"c\":[{\"n\":\"flex\",\"s\":{\"flex-direction\":\"row\",\"justify-content\":\"space-between\",\"width\":\"100%%\",\"background\":\"#0a1e24\"},\"c\":[{\"n\":\"mono\",\"t\":\"CPU\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}},{\"n\":\"mono\",\"t\":\"100\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}]},{\"n\":\"flex\",\"s\":{\"flex-direction\":\"row\",\"justify-content\":\"space-between\",\"width\":\"100%%\",\"background\":\"#0a1e24\"},\"c\":[{\"n\":\"mono\",\"t\":\"MEM\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}},{\"n\":\"mono\",\"t\":\"050\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}]},{\"n\":\"flex\",\"s\":{\"flex-direction\":\"row\",\"justify-content\":\"space-between\",\"width\":\"100%%\",\"background\":\"#0a1e24\"},\"c\":[{\"n\":\"mono\",\"t\":\"NET\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}},{\"n\":\"mono\",\"t\":\"012\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}]}]}}\\x1b\\\\'",
        native_uses_proxy: true,
        category: "flex-nested",
    },
    // Nested flex: left-aligned header row over a space-between footer row.
    // c=10, 2 rows. (3-char blocks so cell-fill is unambiguous.)
    //   HEADER
    //   AAA    ZZZ
    TestCase {
        name: "nested_header_footer",
        text: "HEADER",
        cols: 10,
        native_cmd: "printf '%s\\n%s' 'HEADER' 'AAA    ZZZ'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=10,r=2;{\"S\":{\"n\":\"flex\",\"s\":{\"flex-direction\":\"column\",\"width\":\"100%%\",\"height\":\"100%%\",\"background\":\"#0a1e24\"},\"c\":[{\"n\":\"mono\",\"t\":\"HEADER\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}},{\"n\":\"flex\",\"s\":{\"flex-direction\":\"row\",\"justify-content\":\"space-between\",\"width\":\"100%%\",\"background\":\"#0a1e24\"},\"c\":[{\"n\":\"mono\",\"t\":\"AAA\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}},{\"n\":\"mono\",\"t\":\"ZZZ\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}]}]}}\\x1b\\\\'",
        native_uses_proxy: true,
        category: "flex-nested",
    },
    // Nested flex: outer row (space-between) holding two column stacks =
    // a two-pane sidebar layout. c=12, 2 rows tall.
    //   AAA      YYY
    //   BBB      ZZZ
    TestCase {
        name: "nested_two_panes",
        text: "AAA      YYY",
        cols: 12,
        native_cmd: "printf '%s\\n%s' 'AAA      YYY' 'BBB      ZZZ'",
        twp_cmd: "printf '\\x1b_twp;v=1,c=12,r=2;{\"S\":{\"n\":\"flex\",\"s\":{\"flex-direction\":\"row\",\"justify-content\":\"space-between\",\"width\":\"100%%\",\"height\":\"100%%\",\"background\":\"#0a1e24\"},\"c\":[{\"n\":\"flex\",\"s\":{\"flex-direction\":\"column\",\"background\":\"#0a1e24\"},\"c\":[{\"n\":\"mono\",\"t\":\"AAA\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}},{\"n\":\"mono\",\"t\":\"BBB\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}]},{\"n\":\"flex\",\"s\":{\"flex-direction\":\"column\",\"background\":\"#0a1e24\"},\"c\":[{\"n\":\"mono\",\"t\":\"YYY\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}},{\"n\":\"mono\",\"t\":\"ZZZ\",\"s\":{\"color\":\"#ecefc1\",\"background\":\"#0a1e24\"}}]}]}}\\x1b\\\\'",
        native_uses_proxy: true,
        category: "flex-nested",
    },
];

// ── Showcase: CSS text effects with no terminal equivalent ─────────
// These render via the CSS passthrough (Style::extra). There's no native
// reference to diff against, so they're displayed for visual inspection
// only — a render is a pass, a blank/failed capture is a fail. Structural
// invariants for these effects live in src/render_tests.rs.
struct Showcase {
    name: &'static str,
    twp_cmd: &'static str,
    category: &'static str,
}

const SHOWCASE: &[Showcase] = &[
    // ── Text effects ──────────────────────────────────────────────
    // Drop shadow — offset, blurred, dark.
    Showcase {
        name: "fx_drop_shadow",
        twp_cmd: "printf '\\x1b_twp;v=1,c=14,r=2;{\"S\":{\"n\":\"mono\",\"t\":\"SHADOW\",\"s\":{\"scale\":2,\"color\":\"#ecefc1\",\"background\":\"#0a1e24\",\"text-shadow\":\"3px 3px 4px #000000\"}}}\\x1b\\\\'",
        category: "css-effects",
    },
    // Neon glow — zero-offset coloured blur behind a bright glyph.
    Showcase {
        name: "fx_neon_glow",
        twp_cmd: "printf '\\x1b_twp;v=1,c=10,r=2;{\"S\":{\"n\":\"mono\",\"t\":\"NEON\",\"s\":{\"scale\":2,\"color\":\"#7df9ff\",\"background\":\"#0a1e24\",\"text-shadow\":\"0 0 8px #00e5ff\"}}}\\x1b\\\\'",
        category: "css-effects",
    },
    // Outline only — fill matches the background, a coloured stroke draws
    // the glyph edges.
    Showcase {
        name: "fx_outline",
        twp_cmd: "printf '\\x1b_twp;v=1,c=10,r=2;{\"S\":{\"n\":\"mono\",\"t\":\"EDGE\",\"s\":{\"scale\":2,\"color\":\"#0a1e24\",\"background\":\"#0a1e24\",\"-webkit-text-stroke\":\"1px #ff5fa2\"}}}\\x1b\\\\'",
        category: "css-effects",
    },
    // Opacity — faded text.
    Showcase {
        name: "fx_opacity",
        twp_cmd: "printf '\\x1b_twp;v=1,c=10,r=2;{\"S\":{\"n\":\"mono\",\"t\":\"FADE\",\"s\":{\"scale\":2,\"color\":\"#ecefc1\",\"background\":\"#0a1e24\",\"opacity\":0.35}}}\\x1b\\\\'",
        category: "css-effects",
    },
    // Coloured underline — decoration in a different colour from the text
    // (takumi only supports solid style; longhands, not the shorthand).
    Showcase {
        name: "fx_colored_underline",
        twp_cmd: "printf '\\x1b_twp;v=1,c=10,r=2;{\"S\":{\"n\":\"mono\",\"t\":\"LINK\",\"s\":{\"scale\":2,\"color\":\"#ecefc1\",\"background\":\"#0a1e24\",\"text-decoration-line\":\"underline\",\"text-decoration-color\":\"#ff5fa2\"}}}\\x1b\\\\'",
        category: "css-effects",
    },
    // ── Mini UIs: styled flex containers + mono text + effects ─────
    // Status pill — rounded green badge with bold white label.
    Showcase {
        name: "ui_status_pill",
        twp_cmd: "printf '\\x1b_twp;v=1,c=16,r=3;{\"S\":{\"n\":\"flex\",\"s\":{\"justify-content\":\"center\",\"align-items\":\"center\",\"width\":\"100%%\",\"height\":\"100%%\",\"background\":\"#0a1e24\",\"padding\":8},\"c\":[{\"n\":\"flex\",\"s\":{\"justify-content\":\"center\",\"align-items\":\"center\",\"background\":\"#16a34a\",\"border-radius\":14,\"padding\":6},\"c\":[{\"n\":\"mono\",\"t\":\" ONLINE \",\"s\":{\"color\":\"#ffffff\",\"font-weight\":\"bold\"}}]}]}}\\x1b\\\\'",
        category: "mini-ui",
    },
    // Raised button — indigo card with a soft drop shadow under it.
    Showcase {
        name: "ui_button_shadow",
        twp_cmd: "printf '\\x1b_twp;v=1,c=18,r=4;{\"S\":{\"n\":\"flex\",\"s\":{\"justify-content\":\"center\",\"align-items\":\"center\",\"width\":\"100%%\",\"height\":\"100%%\",\"background\":\"#0a1e24\",\"padding\":12},\"c\":[{\"n\":\"flex\",\"s\":{\"justify-content\":\"center\",\"align-items\":\"center\",\"background\":\"#6366f1\",\"border-radius\":10,\"padding\":8,\"box-shadow\":\"0 5px 12px #000000aa\"},\"c\":[{\"n\":\"mono\",\"t\":\" DEPLOY \",\"s\":{\"color\":\"#ffffff\",\"font-weight\":\"bold\"}}]}]}}\\x1b\\\\'",
        category: "mini-ui",
    },
    // Toast — card with a coloured accent bar, title and subtitle.
    Showcase {
        name: "ui_toast",
        twp_cmd: "printf '\\x1b_twp;v=1,c=26,r=4;{\"S\":{\"n\":\"flex\",\"s\":{\"flex-direction\":\"row\",\"width\":\"100%%\",\"height\":\"100%%\",\"background\":\"#1e293b\",\"border-radius\":8},\"c\":[{\"n\":\"box\",\"s\":{\"width\":6,\"height\":\"100%%\",\"background\":\"#22c55e\",\"border-radius\":8}},{\"n\":\"flex\",\"s\":{\"flex-direction\":\"column\",\"justify-content\":\"center\",\"padding\":8,\"gap\":2},\"c\":[{\"n\":\"mono\",\"t\":\"Deployed\",\"s\":{\"color\":\"#ecefc1\",\"font-weight\":\"bold\"}},{\"n\":\"mono\",\"t\":\"2 min ago\",\"s\":{\"color\":\"#94a3b8\"}}]}]}}\\x1b\\\\'",
        category: "mini-ui",
    },
    // Progress bar — rounded track with a 60%% cyan fill.
    Showcase {
        name: "ui_progress",
        twp_cmd: "printf '\\x1b_twp;v=1,c=26,r=3;{\"S\":{\"n\":\"flex\",\"s\":{\"align-items\":\"center\",\"width\":\"100%%\",\"height\":\"100%%\",\"background\":\"#0a1e24\",\"padding\":10},\"c\":[{\"n\":\"flex\",\"s\":{\"flex-direction\":\"row\",\"align-items\":\"center\",\"width\":\"100%%\",\"height\":14,\"background\":\"#1e293b\",\"border-radius\":7},\"c\":[{\"n\":\"box\",\"s\":{\"width\":\"60%%\",\"height\":\"100%%\",\"background\":\"#38bdf8\",\"border-radius\":7}}]}]}}\\x1b\\\\'",
        category: "mini-ui",
    },
    // Gradient banner — linear-gradient background with shadowed label.
    Showcase {
        name: "ui_gradient_banner",
        twp_cmd: "printf '\\x1b_twp;v=1,c=22,r=3;{\"S\":{\"n\":\"flex\",\"s\":{\"justify-content\":\"center\",\"align-items\":\"center\",\"width\":\"100%%\",\"height\":\"100%%\",\"border-radius\":12,\"background-image\":\"linear-gradient(90deg,#6366f1,#ec4899)\"},\"c\":[{\"n\":\"mono\",\"t\":\"GRADIENT\",\"s\":{\"color\":\"#ffffff\",\"font-weight\":\"bold\",\"text-shadow\":\"0 1px 3px #00000088\"}}]}}\\x1b\\\\'",
        category: "mini-ui",
    },
];

// ── Helpers ───────────────────────────────────────────────────────

fn display_is_available(display: &str) -> bool {
    Command::new("xdpyinfo")
        .arg("-display")
        .arg(display)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn wait_for_file(path: &Path, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if path.exists() {
            return true;
        }
        thread::sleep(Duration::from_millis(200));
    }
    false
}

fn find_window(display: &str, class: &str) -> Option<String> {
    let output = Command::new("xdotool")
        .args(["search", "--class", class])
        .env("DISPLAY", display)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().last().map(|s| s.trim().to_string())
}

fn capture_window(display: &str, wid: &str, output: &Path) -> bool {
    Command::new("import")
        .args(["-window", wid, output.to_str().unwrap_or("")])
        .env("DISPLAY", display)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn capture_one(cfg: &CaptureConfig) -> Result<Vec<u8>, String> {
    let sig_file = {
        let mut p = env::temp_dir();
        p.push(format!(
            "twp-ss-sig-{}-{}",
            std::process::id(),
            cfg.output.file_name().unwrap_or_default().to_string_lossy()
        ));
        p
    };
    let script_file = {
        let mut p = env::temp_dir();
        p.push(format!(
            "twp-ss-script-{}-{}.sh",
            std::process::id(),
            cfg.output.file_name().unwrap_or_default().to_string_lossy()
        ));
        p
    };
    let _ = fs::remove_file(&sig_file);

    let script_content = format!(
        "#!/bin/bash\nprintf '\\x1b[?25l\\x1b[2J\\x1b[H'\nsleep 0.3\n{}\ntouch {}\nsleep 120\n",
        cfg.command.join(" "),
        sig_file.display()
    );
    fs::write(&script_file, &script_content).map_err(|e| format!("failed to write script: {e}"))?;

    let mut kitty_args: Vec<String> = vec![
        format!("--class={}", cfg.class),
        "--config=NONE".to_string(),
        "--override=allow_remote_control=yes".to_string(),
        format!("--override=font_family={}", cfg.font),
        format!("--override=font_size={}", cfg.font_size),
        format!("--override=background={}", cfg.bg),
        format!("--override=foreground={}", cfg.fg),
        "--override=remember_window_size=no".to_string(),
        format!("--override=initial_window_width={}c", cfg.cols),
        format!("--override=initial_window_height={}c", cfg.rows),
        "--override=confirm_os_window_close=0".to_string(),
        "--override=shell_integration=disabled".to_string(),
        "--override=window_padding_width=0".to_string(),
    ];
    for ov in &cfg.palette {
        kitty_args.push(format!("--override={ov}"));
    }

    if let Some(ref proxy) = cfg.proxy {
        kitty_args.push(proxy.clone());
    }
    kitty_args.extend([
        "bash".to_string(),
        script_file.to_string_lossy().to_string(),
    ]);

    let mut kitty = Command::new("kitty")
        .args(&kitty_args)
        .env("DISPLAY", &cfg.display)
        .env("LIBGL_ALWAYS_SOFTWARE", "1")
        .env("GALLIUM_DRIVER", "llvmpipe")
        .env("KITTY_DISABLE_WAYLAND", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to launch kitty: {e}"))?;

    let timeout = Duration::from_secs(cfg.timeout);
    if !wait_for_file(&sig_file, timeout) {
        let _ = kitty.kill();
        let _ = kitty.wait();
        let _ = fs::remove_file(&sig_file);
        return Err("timed out waiting for render".to_string());
    }

    // Poll until the capture is non-empty (>500 bytes also lets slow software
    // rendering finish before we grab the frame).
    let capture_timeout = Duration::from_secs(10);
    let capture_start = Instant::now();
    let mut captured = false;

    while capture_start.elapsed() < capture_timeout {
        thread::sleep(Duration::from_secs(1));
        if let Some(wid) = find_window(&cfg.display, &cfg.class) {
            if capture_window(&cfg.display, &wid, &cfg.output) && image_has_content(&cfg.output) {
                captured = true;
                break;
            }
        }
    }

    let _ = kitty.kill();
    let _ = kitty.wait();
    let _ = fs::remove_file(&sig_file);
    let _ = fs::remove_file(&script_file);

    if !captured {
        return Err("failed to capture screenshot".to_string());
    }

    fs::read(&cfg.output).map_err(|e| format!("failed to read screenshot: {e}"))
}

/// True once the captured PNG holds real content (not just a blank, freshly
/// cleared window). Decodes the image and counts pixels that differ from the
/// top-right corner (treated as the background) — robust regardless of image
/// size or theme contrast, unlike a raw byte-length floor.
fn image_has_content(path: &Path) -> bool {
    let img = match image::open(path) {
        Ok(i) => i.to_rgba8(),
        Err(_) => return false,
    };
    let (w, h) = img.dimensions();
    if w < 2 || h < 2 {
        return false;
    }
    let bg = *img.get_pixel(w - 1, 0);
    let mut differing = 0u32;
    for p in img.pixels() {
        let d = (p[0] as i32 - bg[0] as i32).abs()
            + (p[1] as i32 - bg[1] as i32).abs()
            + (p[2] as i32 - bg[2] as i32).abs();
        if d > 60 {
            differing += 1;
            if differing > 200 {
                return true;
            }
        }
    }
    false
}

// ── Test runner ───────────────────────────────────────────────────

struct TestConfig {
    display: String,
    proxy_path: String,
    font: String,
    font_size: String,
    report_path: Option<PathBuf>,
    results_dir: PathBuf,
}

fn run_tests(cfg: &TestConfig) -> ExitCode {
    let xvfb = match XvfbSession::ensure(&cfg.display) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("twp-screenshot: {e}");
            return ExitCode::FAILURE;
        }
    };

    let _ = fs::create_dir_all(&cfg.results_dir);

    let mut entries: Vec<TestEntry> = Vec::new();
    let mut pass = 0u32;
    let mut fail = 0u32;
    let mut skip = 0u32;
    let mut current_category = "";

    for tc in TESTS {
        if tc.category != current_category {
            current_category = tc.category;
            let label = match current_category {
                "basic" => "Basic mono (scale=1)",
                "text-sizing" => "Text-sizing (OSC 66 vs TWP)",
                "flex-mono" => "Flex + mono (manual text reference)",
                "flex-nested" => "Nested flex (tables / dashboards)",
                other => other,
            };
            eprintln!("── {label} ──");
        }

        eprint!("  {}: ", tc.name);

        let native_proxy = if tc.native_uses_proxy {
            Some(cfg.proxy_path.clone())
        } else {
            None
        };

        let native_cfg = CaptureConfig {
            output: cfg.results_dir.join(format!("kitty_{}.png", tc.name)),
            display: xvfb.display().to_string(),
            proxy: native_proxy,
            font: cfg.font.clone(),
            font_size: cfg.font_size.clone(),
            cols: 60,
            rows: 10,
            bg: "#0a1e24".to_string(),
            fg: "#ecefc1".to_string(),
            palette: Vec::new(),
            class: "twp-screenshot".to_string(),
            timeout: 15,
            command: vec![tc.native_cmd.to_string()],
        };

        let twp_cfg = CaptureConfig {
            output: cfg.results_dir.join(format!("twp_{}.png", tc.name)),
            proxy: Some(cfg.proxy_path.clone()),
            ..CaptureConfig {
                output: cfg.results_dir.join(format!("twp_{}.png", tc.name)),
                display: xvfb.display().to_string(),
                proxy: Some(cfg.proxy_path.clone()),
                font: cfg.font.clone(),
                font_size: cfg.font_size.clone(),
                cols: 60,
                rows: 10,
                bg: "#0a1e24".to_string(),
                fg: "#ecefc1".to_string(),
                palette: Vec::new(),
                class: "twp-screenshot".to_string(),
                timeout: 15,
                command: vec![tc.twp_cmd.to_string()],
            }
        };

        let native_png = match capture_one(&native_cfg) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("SKIP (native: {e})");
                skip += 1;
                entries.push(TestEntry {
                    name: tc.name.to_string(),
                    result: CompareResult {
                        status: TestStatus::Skip(format!("native: {e}")),
                        matches: 0,
                        total: tc.cols,
                        mismatches: vec![],
                    },
                    native_png: None,
                    twp_png: None,
                    category: tc.category.to_string(),
                    native_label: if tc.native_uses_proxy {
                        "Kitty native"
                    } else {
                        "Kitty OSC 66"
                    }
                    .to_string(),
                });
                continue;
            }
        };

        let twp_png = match capture_one(&twp_cfg) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("SKIP (twp: {e})");
                skip += 1;
                entries.push(TestEntry {
                    name: tc.name.to_string(),
                    result: CompareResult {
                        status: TestStatus::Skip(format!("twp: {e}")),
                        matches: 0,
                        total: tc.cols,
                        mismatches: vec![],
                    },
                    native_png: Some(native_png),
                    twp_png: None,
                    category: tc.category.to_string(),
                    native_label: if tc.native_uses_proxy {
                        "Kitty native"
                    } else {
                        "Kitty OSC 66"
                    }
                    .to_string(),
                });
                continue;
            }
        };

        let native_img = image::load_from_memory(&native_png)
            .expect("invalid native PNG")
            .to_rgba8();
        let twp_img = image::load_from_memory(&twp_png)
            .expect("invalid TWP PNG")
            .to_rgba8();

        let result = compare::compare_images(&native_img, &twp_img, tc.text, tc.cols);
        let summary = result.summary();
        eprintln!("{summary}");

        if result.is_pass() {
            pass += 1;
        } else {
            fail += 1;
        }

        entries.push(TestEntry {
            name: tc.name.to_string(),
            result,
            native_png: Some(native_png),
            twp_png: Some(twp_png),
            category: tc.category.to_string(),
            native_label: if tc.native_uses_proxy {
                "Kitty native"
            } else {
                "Kitty OSC 66"
            }
            .to_string(),
        });
    }

    // ── Showcase: render-only effects & widgets (no comparison) ────
    let display = xvfb.display().to_string();

    // Scoped so the `entries`-borrowing closure is dropped before the
    // comparison loop below (which also touches `entries`).
    {
        // Capture one render-only showcase widget and record it. Returns whether
        // it rendered. `cols`/`rows` control the kitty window the widget renders
        // into (bigger widgets need a bigger window).
        let mut run_showcase =
            |name: &str, category: &str, twp_cmd: String, win_cols: u32, win_rows: u32| -> bool {
                eprint!("  {name}: ");
                let cfg_sc = CaptureConfig {
                    output: cfg.results_dir.join(format!("twp_{name}.png")),
                    display: display.clone(),
                    proxy: Some(cfg.proxy_path.clone()),
                    font: cfg.font.clone(),
                    font_size: cfg.font_size.clone(),
                    cols: win_cols,
                    rows: win_rows,
                    bg: "#0a1e24".to_string(),
                    fg: "#ecefc1".to_string(),
                    palette: Vec::new(),
                    class: "twp-screenshot".to_string(),
                    timeout: 15,
                    command: vec![twp_cmd],
                };
                match capture_one(&cfg_sc) {
                    Ok(png) => {
                        eprintln!("RENDERED");
                        entries.push(TestEntry {
                            name: name.to_string(),
                            result: CompareResult {
                                status: TestStatus::Pass,
                                matches: 0,
                                total: 0,
                                mismatches: vec![],
                            },
                            native_png: None,
                            twp_png: Some(png),
                            category: category.to_string(),
                            native_label: "(no terminal equivalent)".to_string(),
                        });
                        true
                    }
                    Err(e) => {
                        eprintln!("FAIL ({e})");
                        entries.push(TestEntry {
                            name: name.to_string(),
                            result: CompareResult {
                                status: TestStatus::Fail(format!("render failed: {e}")),
                                matches: 0,
                                total: 0,
                                mismatches: vec![],
                            },
                            native_png: None,
                            twp_png: None,
                            category: category.to_string(),
                            native_label: "(no terminal equivalent)".to_string(),
                        });
                        false
                    }
                }
            };

        let mut showcase_category = "";
        for sc in SHOWCASE {
            if sc.category != showcase_category {
                showcase_category = sc.category;
                let label = match showcase_category {
                    "css-effects" => "CSS text effects (visual only)",
                    "mini-ui" => "Mini UIs (visual only)",
                    other => other,
                };
                eprintln!("── {label} ──");
            }
            // The static showcases fit comfortably in a 60×10 window.
            if run_showcase(sc.name, sc.category, sc.twp_cmd.to_string(), 60, 10) {
                pass += 1;
            } else {
                fail += 1;
            }
        }

        // Generated "mini-app" demos (code+minimap, heatmap, chart, chat).
        eprintln!("── Mini apps (visual only) ──");
        for demo in demos::generated_demos() {
            let twp_cmd = demo_twp_cmd(demo.cols, demo.rows, &demo.scene);
            // Render into a window a little larger than the widget so nothing
            // is clipped at the edges.
            let (win_c, win_r) = (demo.cols + 6, demo.rows + 4);
            if run_showcase(demo.name, demo.category, twp_cmd, win_c, win_r) {
                pass += 1;
            } else {
                fail += 1;
            }
        }
    } // end run_showcase scope

    // Native-vs-TWP comparisons: capture a native terminal command and a TWP
    // widget in *separate* windows (they can't share one — placeholder images
    // and printed text don't coexist on screen), shown side by side.
    eprintln!("── Native vs term() across terminal themes (side by side) ──");
    for cmp in demos::comparison_demos() {
        eprint!("  {}: ", cmp.name);
        // Install the theme one-off via kitty `--override color0..15` plus the
        // matching fg/bg. The native swatches read these palette slots directly;
        // the proxy queries them over OSC so its `term()` colours match.
        let palette: Vec<String> = cmp
            .theme
            .ansi
            .iter()
            .enumerate()
            .map(|(i, c)| format!("color{i}={c}"))
            .collect();
        let native_cfg = CaptureConfig {
            output: cfg.results_dir.join(format!("kitty_{}.png", cmp.name)),
            display: display.clone(),
            proxy: None,
            font: cfg.font.clone(),
            font_size: cfg.font_size.clone(),
            cols: cmp.native_cols,
            rows: cmp.native_rows,
            bg: cmp.theme.bg.to_string(),
            fg: cmp.theme.fg.to_string(),
            palette: palette.clone(),
            class: "twp-screenshot".to_string(),
            timeout: 15,
            command: vec![cmp.native_cmd.clone()],
        };
        let twp_cfg = CaptureConfig {
            output: cfg.results_dir.join(format!("twp_{}.png", cmp.name)),
            display: display.clone(),
            proxy: Some(cfg.proxy_path.clone()),
            font: cfg.font.clone(),
            font_size: cfg.font_size.clone(),
            cols: cmp.twp_cols + 6,
            rows: cmp.twp_rows + 4,
            bg: cmp.theme.bg.to_string(),
            fg: cmp.theme.fg.to_string(),
            palette,
            class: "twp-screenshot".to_string(),
            timeout: 15,
            command: vec![demo_twp_cmd(cmp.twp_cols, cmp.twp_rows, &cmp.twp_scene)],
        };
        let native_png = capture_one(&native_cfg).ok();
        let twp_png = capture_one(&twp_cfg).ok();
        let ok = native_png.is_some() && twp_png.is_some();
        eprintln!("{}", if ok { "RENDERED" } else { "FAIL" });
        if ok {
            pass += 1;
        } else {
            fail += 1;
        }
        entries.push(TestEntry {
            name: format!("{} — {}", cmp.name, cmp.label),
            result: if ok {
                CompareResult {
                    status: TestStatus::Pass,
                    matches: 0,
                    total: 0,
                    mismatches: vec![],
                }
            } else {
                CompareResult {
                    status: TestStatus::Fail("render failed".into()),
                    matches: 0,
                    total: 0,
                    mismatches: vec![],
                }
            },
            native_png,
            twp_png,
            category: cmp.category.to_string(),
            native_label: format!("Kitty native — {} (ANSI 48;5;n)", cmp.label),
        });
    }

    // Theme-derived demos: the same widget rendered once per terminal theme
    // (palette installed one-off), every colour derived from term()/color-mix.
    eprintln!("── Theme-derived widgets (term() + color-mix) ──");
    for td in demos::themed_demos() {
        eprint!("  {}: ", td.name);
        let palette: Vec<String> = td
            .theme
            .ansi
            .iter()
            .enumerate()
            .map(|(i, c)| format!("color{i}={c}"))
            .collect();
        let cfg_td = CaptureConfig {
            output: cfg.results_dir.join(format!("twp_{}.png", td.name)),
            display: display.clone(),
            proxy: Some(cfg.proxy_path.clone()),
            font: cfg.font.clone(),
            font_size: cfg.font_size.clone(),
            cols: td.cols + 6,
            rows: td.rows + 4,
            bg: td.theme.bg.to_string(),
            fg: td.theme.fg.to_string(),
            palette,
            class: "twp-screenshot".to_string(),
            timeout: 15,
            command: vec![demo_twp_cmd(td.cols, td.rows, &td.scene)],
        };
        let twp_png = capture_one(&cfg_td).ok();
        let ok = twp_png.is_some();
        eprintln!("{}", if ok { "RENDERED" } else { "FAIL" });
        if ok {
            pass += 1;
        } else {
            fail += 1;
        }
        entries.push(TestEntry {
            name: format!("{} — {}", td.name, td.label),
            result: if ok {
                CompareResult {
                    status: TestStatus::Pass,
                    matches: 0,
                    total: 0,
                    mismatches: vec![],
                }
            } else {
                CompareResult {
                    status: TestStatus::Fail("render failed".into()),
                    matches: 0,
                    total: 0,
                    mismatches: vec![],
                }
            },
            native_png: None,
            twp_png,
            category: td.category.to_string(),
            native_label: String::new(),
        });
    }

    let total = pass + fail + skip;
    eprintln!();
    eprintln!("==========================");
    eprintln!("Results: {pass} passed, {fail} failed, {skip} skipped (of {total})");

    if let Some(ref report_path) = cfg.report_path {
        let font_info = format!("Font: {} @ {}pt", cfg.font, cfg.font_size);
        if let Err(e) = report::generate_html(&entries, &font_info, report_path) {
            eprintln!("Failed to write report: {e}");
        } else {
            eprintln!("Report:  file://{}", report_path.display());
        }
    }

    drop(xvfb);

    if fail > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

// ── CLI ───────────────────────────────────────────────────────────

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.first().map(|s| s.as_str()) == Some("test") {
        return main_test(&args[1..]);
    }

    let cfg = match parse_capture_args(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };

    let xvfb = match XvfbSession::ensure(&cfg.display) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("twp-screenshot: {e}");
            return ExitCode::FAILURE;
        }
    };

    let result = match capture_one(&cfg) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("twp-screenshot: {e}");
            ExitCode::FAILURE
        }
    };

    drop(xvfb);
    result
}

fn main_test(args: &[String]) -> ExitCode {
    let mut display = ":55".to_string();
    let mut report_path = None;
    let mut font = String::new();
    let mut font_size = String::new();

    // Read font from kitty config
    if let Ok(contents) = fs::read_to_string(dirs_from_home(".config/kitty/kitty.conf")) {
        for line in contents.lines() {
            if let Some(rest) = line.strip_prefix("font_family") {
                if font.is_empty() {
                    font = rest.trim().to_string();
                }
            }
            if let Some(rest) = line.strip_prefix("font_size") {
                if font_size.is_empty() {
                    font_size = rest.trim().to_string();
                }
            }
        }
    }
    if font.is_empty() {
        font = "monospace".to_string();
    }
    if font_size.is_empty() {
        font_size = "16".to_string();
    }

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--display" => {
                i += 1;
                display = args[i].clone();
            }
            "--report" => {
                i += 1;
                report_path = Some(PathBuf::from(&args[i]));
            }
            "--font" => {
                i += 1;
                font = args[i].clone();
            }
            "--font-size" => {
                i += 1;
                font_size = args[i].clone();
            }
            _ => {}
        }
        i += 1;
    }

    let exe_dir = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let proxy_path = exe_dir.join("twp-proxy").to_string_lossy().to_string();

    eprintln!("TWP Visual Comparison Test");
    eprintln!("==========================");
    eprintln!("Font: {font} @ {font_size}pt");
    eprintln!();

    let results_dir = PathBuf::from("/tmp/twp-visual-test");

    run_tests(&TestConfig {
        display,
        proxy_path,
        font,
        font_size,
        report_path,
        results_dir,
    })
}

fn dirs_from_home(suffix: &str) -> PathBuf {
    env::var("HOME")
        .map(|h| PathBuf::from(h).join(suffix))
        .unwrap_or_else(|_| PathBuf::from(suffix))
}

fn parse_capture_args(args: &[String]) -> Result<CaptureConfig, String> {
    let mut output = None;
    let mut display = ":55".to_string();
    let mut proxy = None;
    let mut font = "monospace".to_string();
    let mut font_size = "16".to_string();
    let mut cols = 60u32;
    let mut rows = 10u32;
    let mut bg = "#0a1e24".to_string();
    let mut fg = "#ecefc1".to_string();
    let mut class = "twp-screenshot".to_string();
    let mut timeout = 15u64;
    let mut command = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--" => {
                command = args[i + 1..].to_vec();
                break;
            }
            "--output" | "-o" => {
                i += 1;
                output = Some(PathBuf::from(&args[i]));
            }
            "--display" => {
                i += 1;
                display = args[i].clone();
            }
            "--proxy" => {
                i += 1;
                proxy = Some(args[i].clone());
            }
            "--font" => {
                i += 1;
                font = args[i].clone();
            }
            "--font-size" => {
                i += 1;
                font_size = args[i].clone();
            }
            "--cols" => {
                i += 1;
                cols = args[i].parse().map_err(|_| "invalid --cols")?;
            }
            "--rows" => {
                i += 1;
                rows = args[i].parse().map_err(|_| "invalid --rows")?;
            }
            "--bg" => {
                i += 1;
                bg = args[i].clone();
            }
            "--fg" => {
                i += 1;
                fg = args[i].clone();
            }
            "--class" => {
                i += 1;
                class = args[i].clone();
            }
            "--timeout" => {
                i += 1;
                timeout = args[i].parse().map_err(|_| "invalid --timeout")?;
            }
            "--help" | "-h" => return Err(USAGE.to_string()),
            other => return Err(format!("unknown argument: {other}\n{USAGE}")),
        }
        i += 1;
    }

    let output = output.ok_or_else(|| format!("--output is required\n{USAGE}"))?;
    if command.is_empty() {
        return Err(format!("no command specified after --\n{USAGE}"));
    }

    Ok(CaptureConfig {
        output,
        display,
        proxy,
        font,
        font_size,
        cols,
        rows,
        bg,
        fg,
        palette: Vec::new(),
        class,
        timeout,
        command,
    })
}

const USAGE: &str = "Usage: twp-screenshot [test | capture options]

Subcommands:
  test                     Run visual comparison tests
    --report <PATH>        Generate HTML report
    --display <DISP>       X display (default: :55)
    --font <FAMILY>        Font family
    --font-size <N>        Font size in pt

  (no subcommand)          Capture a single screenshot
    --output, -o <PATH>    Output PNG path (required)
    --display <DISP>       X display (default: :55)
    --proxy <PATH>         Run through twp-proxy
    --font <FAMILY>        Font family (default: monospace)
    --font-size <N>        Font size in pt (default: 16)
    --cols <N>             Terminal width in cells (default: 60)
    --rows <N>             Terminal height in cells (default: 10)
    --bg <COLOR>           Background color (default: #0a1e24)
    --fg <COLOR>           Foreground color (default: #ecefc1)
    --class <CLASS>        X11 window class (default: twp-screenshot)
    --timeout <SECS>       Max wait seconds (default: 15)
    -- <COMMAND>...        Command to run in kitty";
