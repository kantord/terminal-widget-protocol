use std::env;
use std::fs;
use std::path::PathBuf;

// Generates a Rust source file at $OUT_DIR/diacritics.rs that defines
// the list of combining-mark codepoints used by the Kitty Graphics
// Protocol's Unicode placeholder mechanism for row/column indexing.
//
// The table is taken verbatim from Kitty's published rowcolumn-diacritics.txt.
fn main() {
    let src = "rowcolumn-diacritics.txt";
    println!("cargo:rerun-if-changed={src}");

    let text = fs::read_to_string(src).expect("read diacritics file");
    let mut codepoints: Vec<u32> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let hex = line.split(';').next().expect("malformed line");
        let cp = u32::from_str_radix(hex, 16).expect("hex parse");
        codepoints.push(cp);
    }
    assert!(codepoints.len() >= 256, "need at least 256 diacritics");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = PathBuf::from(&out_dir).join("diacritics.rs");

    let mut out = String::new();
    out.push_str(&format!(
        "pub const DIACRITICS: [char; {}] = [\n",
        codepoints.len()
    ));
    for cp in &codepoints {
        out.push_str(&format!("    '\\u{{{:X}}}',\n", cp));
    }
    out.push_str("];\n");

    fs::write(&out_path, out).expect("write diacritics.rs");
}
