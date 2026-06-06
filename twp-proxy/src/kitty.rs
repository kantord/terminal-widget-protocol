// Kitty Graphics Protocol emission with Unicode placeholders.
//
// Two surfaces:
//   * transmit_image — wraps a PNG into one or more `ESC _ G ... ESC \` APC
//     sequences with U=1 (virtual placement) and q=2 (suppress responses).
//   * placeholder_cells — emits a c×r grid of U+10EEEE cells whose
//     foreground color encodes the image ID and whose combining diacritics
//     identify the row and (first cell of each row) column. Subsequent
//     cells inherit the column by left-to-right auto-increment.
//
// References:
//   https://sw.kovidgoyal.net/kitty/graphics-protocol/
//   https://sw.kovidgoyal.net/kitty/graphics-protocol/#unicode-placeholders

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

include!(concat!(env!("OUT_DIR"), "/diacritics.rs"));

// One Kitty Graphics chunk carries at most 4096 base64-encoded bytes.
const CHUNK: usize = 4096;
const PLACEHOLDER: char = '\u{10EEEE}';

/// Build the bytes of the Kitty Graphics transmit sequence(s) for `png_bytes`
/// at image id `id`, virtually placed into a `cols × rows` cell box.
pub fn transmit_image(id: u32, png_bytes: &[u8], cols: u32, rows: u32) -> Vec<u8> {
    let encoded = STANDARD.encode(png_bytes);
    let mut out = Vec::with_capacity(encoded.len() + 64);

    if encoded.len() <= CHUNK {
        out.extend_from_slice(b"\x1b_G");
        out.extend_from_slice(format!("a=T,f=100,i={id},U=1,c={cols},r={rows},q=2").as_bytes());
        out.push(b';');
        out.extend_from_slice(encoded.as_bytes());
        out.extend_from_slice(b"\x1b\\");
        return out;
    }

    // Chunked transmission: m=1 on all but the last chunk, m=0 to terminate.
    let bytes = encoded.as_bytes();
    let mut offset = 0;
    let mut first = true;
    while offset < bytes.len() {
        let end = (offset + CHUNK).min(bytes.len());
        let is_last = end == bytes.len();
        out.extend_from_slice(b"\x1b_G");
        if first {
            out.extend_from_slice(
                format!(
                    "a=T,f=100,i={id},U=1,c={cols},r={rows},q=2,m={}",
                    if is_last { 0 } else { 1 }
                )
                .as_bytes(),
            );
            first = false;
        } else {
            out.extend_from_slice(format!("m={}", if is_last { 0 } else { 1 }).as_bytes());
        }
        out.push(b';');
        out.extend_from_slice(&bytes[offset..end]);
        out.extend_from_slice(b"\x1b\\");
        offset = end;
    }
    out
}

/// Build the bytes that paint a `cols × rows` placeholder grid referring to
/// image `id`. Includes SGR escapes to set the fg color (the ID's low 24
/// bits) and restore it afterwards. Each row ends with `\r\n` so the image
/// occupies multiple lines in the scrollback buffer.
pub fn placeholder_cells(id: u32, cols: u32, rows: u32) -> Vec<u8> {
    let r = ((id >> 16) & 0xFF) as u8;
    let g = ((id >> 8) & 0xFF) as u8;
    let b = (id & 0xFF) as u8;

    let mut out = Vec::with_capacity(cols as usize * rows as usize * 6 + 32);

    for row_idx in 0..rows {
        // Each row re-asserts the fg color so a `\r\n` between rows can't
        // strand subsequent cells without an active color.
        out.extend_from_slice(format!("\x1b[38;2;{r};{g};{b}m").as_bytes());

        let row_diacritic = DIACRITICS[row_idx as usize];

        // First cell carries row + col(0) diacritics; later cells carry only
        // the row diacritic and let the column auto-increment from the left
        // neighbour (which has the same foreground color).
        push_char(&mut out, PLACEHOLDER);
        push_char(&mut out, row_diacritic);
        push_char(&mut out, DIACRITICS[0]);

        for _ in 1..cols {
            push_char(&mut out, PLACEHOLDER);
            push_char(&mut out, row_diacritic);
        }

        // Reset fg so following text in the buffer isn't colored.
        out.extend_from_slice(b"\x1b[39m");

        if row_idx + 1 < rows {
            out.extend_from_slice(b"\r\n");
        }
    }

    out
}

fn push_char(out: &mut Vec<u8>, c: char) {
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    out.extend_from_slice(s.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diacritics_table_loaded() {
        assert!(DIACRITICS.len() >= 256);
        assert_eq!(DIACRITICS[0], '\u{0305}');
        assert_eq!(DIACRITICS[1], '\u{030D}');
        assert_eq!(DIACRITICS[2], '\u{030E}');
    }

    #[test]
    fn transmit_is_one_chunk_for_small_payload() {
        let bytes = transmit_image(0x123456, b"hello", 4, 2);
        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(s.starts_with("\x1b_G"));
        assert!(s.contains("a=T"));
        assert!(s.contains("f=100"));
        assert!(s.contains("i=1193046"));
        assert!(s.contains("U=1"));
        assert!(s.contains("c=4"));
        assert!(s.contains("r=2"));
        assert!(s.contains("q=2"));
        assert!(s.ends_with("\x1b\\"));
        assert!(!s.contains("m="));
    }

    #[test]
    fn transmit_chunks_large_payload() {
        let big = vec![0xAAu8; CHUNK * 3];
        let bytes = transmit_image(1, &big, 1, 1);
        let s = std::str::from_utf8(&bytes).unwrap();
        // Multiple APC sequences, the last one with m=0
        assert!(s.matches("\x1b_G").count() >= 2);
        assert!(s.contains("m=1"));
        assert!(s.contains("m=0"));
    }

    #[test]
    fn placeholder_encodes_id_in_fg_color() {
        let bytes = placeholder_cells(0x123456, 2, 1);
        let s = std::str::from_utf8(&bytes).unwrap();
        // 0x12=18, 0x34=52, 0x56=86
        assert!(s.contains("\x1b[38;2;18;52;86m"));
        // Two placeholder cells, ending with fg reset, no trailing newline.
        assert!(s.contains("\x1b[39m"));
        assert!(!s.ends_with("\r\n"));
    }

    #[test]
    fn placeholder_emits_one_newline_per_row_boundary() {
        let bytes = placeholder_cells(1, 3, 3);
        let s = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(s.matches("\r\n").count(), 2);
    }
}
