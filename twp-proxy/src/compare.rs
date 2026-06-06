use image::RgbaImage;

pub struct CompareResult {
    pub status: TestStatus,
    pub matches: u32,
    pub total: u32,
    pub mismatches: Vec<String>,
}

pub enum TestStatus {
    Pass,
    Fail(String),
    Skip(String),
}

impl CompareResult {
    pub fn summary(&self) -> String {
        match &self.status {
            TestStatus::Pass => format!("PASS {}/{}", self.matches, self.total),
            TestStatus::Fail(detail) => format!("FAIL {detail}"),
            TestStatus::Skip(reason) => format!("SKIP ({reason})"),
        }
    }

    pub fn is_pass(&self) -> bool {
        matches!(self.status, TestStatus::Pass)
    }
}

pub fn cell_fill(img: &RgbaImage, ncols: u32) -> (Vec<bool>, u32) {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 || ncols == 0 {
        return (vec![false; ncols as usize], 0);
    }

    let bg = img.get_pixel(w - 1, 0);
    let bg_f = [bg[0] as f64, bg[1] as f64, bg[2] as f64];

    let mut col_dev = vec![0.0f64; w as usize];
    for x in 0..w {
        let mut dev = 0.0f64;
        for y in 0..h {
            let p = img.get_pixel(x, y);
            let dr = p[0] as f64 - bg_f[0];
            let dg = p[1] as f64 - bg_f[1];
            let db = p[2] as f64 - bg_f[2];
            dev += (dr * dr + dg * dg + db * db).sqrt();
        }
        col_dev[x as usize] = dev;
    }

    let max_dev = col_dev.iter().cloned().fold(0.0f64, f64::max);
    if max_dev < 1.0 {
        return (vec![false; ncols as usize], 0);
    }
    let ink_th = max_dev * 0.02;

    let ink_cols: Vec<usize> = col_dev
        .iter()
        .enumerate()
        .filter(|(_, d)| **d > ink_th)
        .map(|(i, _)| i)
        .collect();

    if ink_cols.len() < 2 {
        return (vec![false; ncols as usize], 0);
    }

    let x0 = ink_cols[0];
    let x1 = ink_cols[ink_cols.len() - 1] + 1;
    let cw = (x1 - x0) / ncols as usize;
    if cw == 0 {
        return (vec![false; ncols as usize], 0);
    }

    let mut inks = Vec::with_capacity(ncols as usize);
    for i in 0..ncols as usize {
        let cx0 = x0 + i * cw;
        let cx1 = (cx0 + cw).min(w as usize);
        if cx0 >= w as usize {
            inks.push(0.0);
            continue;
        }
        let mut ink = 0.0f64;
        for x in cx0..cx1 {
            for y in 0..h {
                let p = img.get_pixel(x as u32, y);
                let dr = p[0] as f64 - bg_f[0];
                let dg = p[1] as f64 - bg_f[1];
                let db = p[2] as f64 - bg_f[2];
                ink += (dr * dr + dg * dg + db * db).sqrt();
            }
        }
        inks.push(ink);
    }

    let mut nonzero: Vec<f64> = inks.iter().cloned().filter(|&v| v > 0.0).collect();
    if nonzero.is_empty() {
        return (vec![false; ncols as usize], 0);
    }
    nonzero.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = nonzero[nonzero.len() / 2];
    let threshold = median * 0.20;

    let filled: Vec<bool> = inks.iter().map(|&v| v > threshold).collect();
    let count = filled.iter().filter(|&&f| f).count() as u32;
    (filled, count)
}

pub fn compare_images(native: &RgbaImage, twp: &RgbaImage, text: &str, cols: u32) -> CompareResult {
    let (kit_cells, kit_n) = cell_fill(native, cols);
    let (twp_cells, twp_n) = cell_fill(twp, cols);

    let expected_filled = text
        .chars()
        .take(cols as usize)
        .filter(|&c| c != ' ')
        .count() as u32;
    let min_ink = (expected_filled / 2).max(1);

    if kit_n < min_ink {
        return CompareResult {
            status: TestStatus::Fail(format!("kitty blank ({kit_n}/{expected_filled})")),
            matches: 0,
            total: cols,
            mismatches: vec![],
        };
    }
    if twp_n < min_ink {
        return CompareResult {
            status: TestStatus::Fail(format!("twp blank ({twp_n}/{expected_filled})")),
            matches: 0,
            total: cols,
            mismatches: vec![],
        };
    }

    let matches = kit_cells
        .iter()
        .zip(twp_cells.iter())
        .filter(|(k, t)| k == t)
        .count() as u32;

    let mismatches: Vec<String> = kit_cells
        .iter()
        .zip(twp_cells.iter())
        .enumerate()
        .filter(|(_, (k, t))| k != t)
        .take(5)
        .map(|(i, _)| {
            let ch = text.chars().nth(i).unwrap_or('?');
            format!("{i}:{ch}")
        })
        .collect();

    let status = if matches == cols {
        TestStatus::Pass
    } else {
        let mut detail = format!("{matches}/{cols}");
        if !mismatches.is_empty() {
            detail.push_str(" mm=");
            detail.push_str(&mismatches.join(","));
        }
        TestStatus::Fail(detail)
    };

    CompareResult {
        status,
        matches,
        total: cols,
        mismatches,
    }
}
