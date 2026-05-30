// Automated cell-alignment verification for the `mono` node type.
//
// These tests render known strings as mono widgets and inspect the
// resulting PNG pixel data to verify that:
//   * Each character's glyph falls within its expected cell column
//   * Characters don't bleed into adjacent cells
//   * Scaled text (scale=2, scale=3) occupies the right number of cells
//   * Different string lengths produce linearly-scaling positions
//
// No Kitty or display server needed — pure image-based verification.

#[cfg(test)]
mod tests {
    use crate::expand::expand;
    use crate::protocol::Payload;
    use crate::render::{px_per_col, px_per_row, render_to_png};
    use image::RgbaImage;
    use proptest::prelude::*;

    fn render_mono(text: &str, cols: u32, rows: u32, extra_style: &str) -> RgbaImage {
        let style = if extra_style.is_empty() {
            "\"color\":\"#000000\",\"background\":\"#ffffff\"".to_string()
        } else {
            format!("\"color\":\"#000000\",\"background\":\"#ffffff\",{extra_style}")
        };
        let json = format!(
            "{{\"S\":{{\"n\":\"mono\",\"t\":\"{text}\",\"s\":{{{style}}}}}}}"
        );
        let payload: Payload = serde_json::from_str(&json).unwrap();
        let scene = expand(payload.scene.unwrap(), &payload.defs);
        let png = render_to_png(&scene, cols, rows);
        image::load_from_memory(&png).unwrap().to_rgba8()
    }

    fn render_payload(json: &str, cols: u32, rows: u32) -> RgbaImage {
        let payload: Payload = serde_json::from_str(json).unwrap();
        let scene = expand(payload.scene.unwrap(), &payload.defs);
        let png = render_to_png(&scene, cols, rows);
        image::load_from_memory(&png).unwrap().to_rgba8()
    }

    /// For each cell column, count non-background pixels in that column.
    /// Returns a vec indexed by cell index, each entry = count of "ink" pixels.
    fn ink_per_cell(img: &RgbaImage, cell_w: u32, num_cells: u32) -> Vec<u32> {
        (0..num_cells)
            .map(|cell| {
                let x_start = cell * cell_w;
                let x_end = (x_start + cell_w).min(img.width());
                let mut count = 0u32;
                for x in x_start..x_end {
                    for y in 0..img.height() {
                        let p = img.get_pixel(x, y);
                        // Background is white (#ffffff). Ink is anything
                        // noticeably darker.
                        if p[0] < 200 || p[1] < 200 || p[2] < 200 {
                            count += 1;
                        }
                    }
                }
                count
            })
            .collect()
    }

    /// Compute the horizontal center-of-mass of ink pixels within a cell.
    /// Returns None if the cell has no ink.
    fn glyph_center_x(img: &RgbaImage, cell_start_x: u32, cell_w: u32) -> Option<f32> {
        let mut total_weight = 0f64;
        let mut total_x = 0f64;
        let x_end = (cell_start_x + cell_w).min(img.width());
        for x in cell_start_x..x_end {
            for y in 0..img.height() {
                let p = img.get_pixel(x, y);
                if p[0] < 200 || p[1] < 200 || p[2] < 200 {
                    total_weight += 1.0;
                    total_x += x as f64;
                }
            }
        }
        if total_weight == 0.0 {
            None
        } else {
            Some((total_x / total_weight) as f32)
        }
    }

    // ─── Tests ───────────────────────────────────────────────────────

    #[test]
    fn every_cell_has_ink() {
        let text = "ABCDEFGHIJ";
        let cols = text.len() as u32;
        let img = render_mono(text, cols, 1, "");
        let cell_w = px_per_col();
        let ink = ink_per_cell(&img, cell_w, cols);
        for (i, &count) in ink.iter().enumerate() {
            assert!(
                count > 0,
                "cell {i} (char '{}') has no ink pixels",
                text.chars().nth(i).unwrap()
            );
        }
    }

    #[test]
    fn spaces_have_no_ink() {
        let text = "A B C";
        let cols = text.len() as u32;
        let img = render_mono(text, cols, 1, "");
        let cell_w = px_per_col();
        let ink = ink_per_cell(&img, cell_w, cols);
        // Cells 1 and 3 are spaces — should have zero or near-zero ink
        assert!(
            ink[1] == 0,
            "space cell 1 has {} ink pixels (expected 0)",
            ink[1]
        );
        assert!(
            ink[3] == 0,
            "space cell 3 has {} ink pixels (expected 0)",
            ink[3]
        );
    }

    #[test]
    fn glyph_centers_are_near_cell_centers() {
        let text = "MMMMMMMMMM";
        let cols = text.len() as u32;
        let img = render_mono(text, cols, 1, "");
        let cell_w = px_per_col();
        for i in 0..cols {
            let cell_start = i * cell_w;
            let expected_center = cell_start as f32 + cell_w as f32 / 2.0;
            if let Some(actual_center) = glyph_center_x(&img, cell_start, cell_w) {
                let drift = (actual_center - expected_center).abs();
                let max_drift = cell_w as f32 * 0.4;
                assert!(
                    drift < max_drift,
                    "cell {i}: glyph center {actual_center:.1} is {drift:.1}px from \
                     cell center {expected_center:.1} (max allowed: {max_drift:.1})"
                );
            }
        }
    }

    #[test]
    fn different_lengths_produce_proportional_widths() {
        let cell_w = px_per_col();
        for len in [5, 10, 20, 40] {
            let text: String = "X".repeat(len);
            let cols = len as u32;
            let img = render_mono(&text, cols, 1, "");
            let expected_w = cols * cell_w;
            assert_eq!(
                img.width(),
                expected_w,
                "text of length {len}: image width {} != expected {expected_w}",
                img.width()
            );
        }
    }

    #[test]
    fn scale2_doubles_cell_size() {
        let text = "ABC";
        let cell_w = px_per_col();
        // scale=1: 3 cells wide, 1 cell tall
        let img1 = render_mono(text, 3, 1, "");
        // scale=2: 6 cells wide, 2 cells tall
        let img2 = render_mono(text, 6, 2, "\"scale\":2");

        assert_eq!(img1.width(), 3 * cell_w);
        assert_eq!(img2.width(), 6 * cell_w);
        assert_eq!(img2.height(), 2 * px_per_row());

        // Each character in scale=2 should have ink in a 2-cell-wide column
        let ink2 = ink_per_cell(&img2, cell_w * 2, 3);
        for (i, &count) in ink2.iter().enumerate() {
            assert!(
                count > 0,
                "scale=2 cell {i} has no ink"
            );
        }
    }

    #[test]
    fn scale3_triples_cell_size() {
        let text = "AB";
        let cell_w = px_per_col();
        let img = render_mono(text, 6, 3, "\"scale\":3");
        assert_eq!(img.width(), 6 * cell_w);
        assert_eq!(img.height(), 3 * px_per_row());
    }

    #[test]
    fn char_width2_doubles_horizontal_only() {
        let text = "ABC";
        let cell_w = px_per_col();
        let img = render_mono(text, 6, 1, "\"char-width\":2");
        // 3 chars × 2 cells each = 6 cells wide, but only 1 cell tall
        assert_eq!(img.width(), 6 * cell_w);
        assert_eq!(img.height(), 1 * px_per_row());
    }

    #[test]
    fn subscale_produces_smaller_glyphs() {
        let text = "MMMM";
        let cols = 4u32;
        let cell_w = px_per_col();
        let img_full = render_mono(text, cols, 1, "");
        let img_half = render_mono(text, cols, 1, "\"subscale-n\":1,\"subscale-d\":2");

        let ink_full: u32 = ink_per_cell(&img_full, cell_w, cols).iter().sum();
        let ink_half: u32 = ink_per_cell(&img_half, cell_w, cols).iter().sum();

        // Half-scale glyphs should have noticeably fewer ink pixels
        assert!(
            ink_half < ink_full,
            "subscale 1/2 ink ({ink_half}) should be less than full ink ({ink_full})"
        );
    }

    #[test]
    fn identical_strings_render_identically() {
        let text = "Hello, world!";
        let cols = text.len() as u32;
        let img1 = render_mono(text, cols, 1, "");
        let img2 = render_mono(text, cols, 1, "");
        assert_eq!(img1.as_raw(), img2.as_raw(), "identical inputs must produce identical output");
    }

    #[test]
    fn no_ink_bleeds_across_cell_boundary() {
        // Render narrow chars (pipes) and check that the rightmost pixel
        // column of each cell and the leftmost of the next are both
        // background-colored (the glyph shouldn't touch the boundary).
        let text = "| | | | | ";
        let cols = text.len() as u32;
        let img = render_mono(text, cols, 1, "");
        let cell_w = px_per_col();

        for cell in 0..cols.saturating_sub(1) {
            let boundary_x = (cell + 1) * cell_w;
            if boundary_x >= img.width() {
                break;
            }
            // Count ink at the exact boundary column
            let mut boundary_ink = 0u32;
            for y in 0..img.height() {
                let p = img.get_pixel(boundary_x, y);
                if p[0] < 200 || p[1] < 200 || p[2] < 200 {
                    boundary_ink += 1;
                }
            }
            // Some bleed is tolerable (antialiasing), but it shouldn't be
            // more than ~10% of the column height
            let max_bleed = img.height() / 10;
            assert!(
                boundary_ink <= max_bleed,
                "cell boundary at x={boundary_x} (between cells {cell} and {}): \
                 {boundary_ink} ink pixels (max allowed: {max_bleed})",
                cell + 1
            );
        }
    }

    // ─── CSS passthrough effects ────────────────────────────────────
    //
    // These effects (shadow, opacity, stroke) have no terminal equivalent,
    // so there's no native reference to compare against. Instead we assert
    // self-referential invariants: render with and without the effect and
    // check the output changed in the expected direction.

    /// Total "ink" — count of pixels noticeably darker than the white bg.
    fn total_ink(img: &RgbaImage) -> u32 {
        let mut n = 0u32;
        for p in img.pixels() {
            if p[0] < 200 || p[1] < 200 || p[2] < 200 {
                n += 1;
            }
        }
        n
    }

    /// Count of near-black pixels (strong, unfaded ink).
    fn dark_ink(img: &RgbaImage) -> u32 {
        let mut n = 0u32;
        for p in img.pixels() {
            if p[0] < 80 && p[1] < 80 && p[2] < 80 {
                n += 1;
            }
        }
        n
    }

    #[test]
    fn text_shadow_adds_ink() {
        // A hard, opaque, offset shadow paints pixels the plain glyph doesn't.
        let plain = render_mono("XX", 4, 1, "");
        let shadowed = render_mono("XX", 4, 1, "\"text-shadow\":\"3px 3px 0 #000000\"");
        let plain_ink = total_ink(&plain);
        let shadow_ink = total_ink(&shadowed);
        assert!(
            shadow_ink > plain_ink,
            "text-shadow should add ink: plain={plain_ink}, shadowed={shadow_ink}"
        );
    }

    #[test]
    fn opacity_fades_text() {
        // Low opacity blends black text toward the white bg, so strong
        // near-black pixels nearly disappear.
        let opaque = render_mono("MMMM", 4, 1, "");
        let faded = render_mono("MMMM", 4, 1, "\"opacity\":0.2");
        let opaque_dark = dark_ink(&opaque);
        let faded_dark = dark_ink(&faded);
        assert!(opaque_dark > 0, "opaque text should have dark ink");
        assert!(
            faded_dark < opaque_dark / 2,
            "opacity 0.2 should fade most dark ink: opaque={opaque_dark}, faded={faded_dark}"
        );
    }

    #[test]
    fn text_stroke_adds_ink() {
        // An outline stroke widens each glyph, adding ink.
        let plain = render_mono("OOOO", 6, 1, "");
        let stroked = render_mono(
            "OOOO",
            6,
            1,
            "\"-webkit-text-stroke\":\"2px #000000\"",
        );
        assert!(
            total_ink(&stroked) > total_ink(&plain),
            "stroke should add ink"
        );
    }

    #[test]
    fn invalid_css_is_ignored() {
        // An unknown property must not panic and must not change rendering —
        // the protocol's forward-compat guarantee.
        let plain = render_mono("ABC", 3, 1, "");
        let bogus = render_mono("ABC", 3, 1, "\"totally-not-a-prop\":\"42deg\"");
        assert_eq!(
            plain.as_raw(),
            bogus.as_raw(),
            "unknown CSS property should be ignored, leaving render unchanged"
        );
    }

    // ─── img node (Kitty-compatible image source) ───────────────────

    #[test]
    fn img_raw_rgba_renders_pixels() {
        use base64::Engine;
        // A 2×2 solid-red RGBA buffer, transmitted directly (Kitty t=d, f=32).
        let raw: Vec<u8> = vec![
            255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
        ];
        let b64 = base64::engine::general_purpose::STANDARD.encode(&raw);
        let json = format!(
            "{{\"S\":{{\"n\":\"img\",\"s\":{{\"width\":30,\"height\":30}},\"img\":{{\"f\":32,\"s\":2,\"v\":2,\"d\":\"{b64}\"}}}}}}"
        );
        let img = render_payload(&json, 4, 4);
        let has_red = img.pixels().any(|p| p[0] > 200 && p[1] < 80 && p[2] < 80);
        assert!(has_red, "raw-RGBA img node should render red pixels");
    }

    #[test]
    fn img_missing_source_degrades_without_panic() {
        // No `d`/`path` — must not panic, just renders nothing for the node.
        let json = "{\"S\":{\"n\":\"img\",\"s\":{\"width\":20,\"height\":20}}}";
        let img = render_payload(json, 3, 3);
        assert!(img.width() > 0 && img.height() > 0);
    }

    // ─── Property-based tests ───────────────────────────────────────

    proptest! {
        #[test]
        fn mono_never_renders_blank(text in "[A-Za-z0-9]{1,30}") {
            let cols = text.len() as u32;
            let img = render_mono(&text, cols, 1, "");
            let total_ink: u32 = ink_per_cell(&img, px_per_col(), cols).iter().sum();
            prop_assert!(total_ink > 0, "text {:?} produced no ink", text);
        }

        #[test]
        fn non_space_cells_have_ink(text in "[A-Z ]{1,15}") {
            prop_assume!(text.chars().any(|c| c != ' '));
            let cols = text.len() as u32;
            let img = render_mono(&text, cols, 1, "");
            let ink = ink_per_cell(&img, px_per_col(), cols);
            for (i, ch) in text.chars().enumerate() {
                if ch != ' ' {
                    prop_assert!(ink[i] > 0, "cell {} (char '{}') has no ink", i, ch);
                }
            }
        }

        #[test]
        fn scale_multiplies_dimensions(scale in 1u32..=4, len in 1u32..=8) {
            let text: String = "X".repeat(len as usize);
            let cols = len * scale;
            let rows = scale;
            let extra = format!("\"scale\":{scale}");
            let img = render_mono(&text, cols, rows, &extra);
            prop_assert_eq!(img.width(), cols * px_per_col());
            prop_assert_eq!(img.height(), rows * px_per_row());
        }

        #[test]
        fn deterministic_output(text in "[A-Za-z0-9]{1,15}") {
            let cols = text.len() as u32;
            let img1 = render_mono(&text, cols, 1, "");
            let img2 = render_mono(&text, cols, 1, "");
            prop_assert_eq!(img1.as_raw(), img2.as_raw());
        }

        #[test]
        fn no_panic_on_sizing_params(
            scale in 1u32..=4,
            sub_n in 1u32..=4,
            sub_d in 1u32..=4,
            len in 1usize..=8,
        ) {
            prop_assume!(sub_n <= sub_d);
            let text: String = "A".repeat(len);
            let extra = format!(
                "\"scale\":{scale},\"subscale-n\":{sub_n},\"subscale-d\":{sub_d}"
            );
            let cols = len as u32 * scale;
            let rows = scale;
            let _ = render_mono(&text, cols, rows, &extra);
        }
    }
}
