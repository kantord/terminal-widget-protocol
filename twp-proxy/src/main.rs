use std::env;
use std::io::{self, Read, Write};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::thread;

use nix::libc;
use nix::sys::signal::{SigSet, Signal};
use nix::sys::termios::{self, SetArg, Termios};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};

use twp_proxy::{cache, expand, kitty, parser, protocol, render};

// Default cell footprint when the payload header omits c=COLS,ROWS.
// Most Phase 1 widgets are wide-and-short.
const DEFAULT_COLS: u32 = 20;
const DEFAULT_ROWS: u32 = 4;

fn handle_twp(cache: &mut cache::Cache, payload: &[u8], out: &mut Vec<u8>) {
    let parsed = match parse_twp_payload(payload) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("twp-proxy: bad payload: {e}");
            return;
        }
    };

    let image_id = cache::image_id_for(&parsed.json_bytes);
    if cache.mark_transmitted(image_id) {
        let value: protocol::Payload = match serde_json::from_slice(&parsed.json_bytes) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("twp-proxy: JSON parse error: {e}");
                return;
            }
        };
        let Some(scene) = value.scene else {
            // Defs-only payload: registration is a no-op in this implementation
            // because templates are resolved per-payload (Phase 1 spec allows
            // either approach).
            return;
        };
        let expanded = expand::expand(scene, &value.defs);
        let png = render::render_to_png(&expanded, parsed.cols, parsed.rows);
        out.extend_from_slice(&kitty::transmit_image(
            image_id,
            &png,
            parsed.cols,
            parsed.rows,
        ));
    }
    out.extend_from_slice(&kitty::placeholder_cells(image_id, parsed.cols, parsed.rows));
}

struct ParsedTwp<'a> {
    cols: u32,
    rows: u32,
    json_bytes: &'a [u8],
}

/// Splits a `twp;` APC payload into a single comma-separated header section
/// and a JSON body, divided by the first `;`.
///
/// Header keys recognized in Phase 1:
///   * `v=1` — protocol version (required by spec but tolerated when missing)
///   * `c=N` — cell columns
///   * `r=N` — cell rows
/// Unknown keys are reserved for future versions and silently ignored.
fn parse_twp_payload(payload: &[u8]) -> Result<ParsedTwp<'_>, String> {
    let mut cols = DEFAULT_COLS;
    let mut rows = DEFAULT_ROWS;

    let (header_bytes, body_bytes) = match payload.iter().position(|&b| b == b';') {
        Some(idx) => (&payload[..idx], &payload[idx + 1..]),
        None => (&[][..], payload),
    };

    let header = std::str::from_utf8(header_bytes).map_err(|_| "non-UTF8 header".to_string())?;
    for kv in header.split(',') {
        let kv = kv.trim();
        if kv.is_empty() {
            continue;
        }
        let (k, v) = kv
            .split_once('=')
            .ok_or_else(|| format!("header pair missing `=`: `{kv}`"))?;
        match k {
            "v" => {
                if v != "1" {
                    return Err(format!("unsupported protocol version: {v}"));
                }
            }
            "c" => cols = v.parse().map_err(|e| format!("bad c=`{v}`: {e}"))?,
            "r" => rows = v.parse().map_err(|e| format!("bad r=`{v}`: {e}"))?,
            _ => {} // reserved
        }
    }

    Ok(ParsedTwp {
        cols,
        rows,
        json_bytes: body_bytes,
    })
}

fn current_winsize() -> libc::winsize {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let rc = unsafe {
        libc::ioctl(
            libc::STDIN_FILENO,
            libc::TIOCGWINSZ,
            &mut ws as *mut libc::winsize,
        )
    };
    if rc != 0 || ws.ws_col == 0 || ws.ws_row == 0 {
        ws.ws_col = 80;
        ws.ws_row = 24;
    }
    ws
}

fn pty_size_from_winsize(ws: &libc::winsize) -> PtySize {
    PtySize {
        rows: ws.ws_row,
        cols: ws.ws_col,
        pixel_width: ws.ws_xpixel,
        pixel_height: ws.ws_ypixel,
    }
}

/// Query the terminal palette via OSC 4 (indices 0–15), OSC 10 (fg) and
/// OSC 11 (bg), install it via `render::set_palette`, and return any
/// non-response bytes read (stray input) so they can be replayed to the child.
/// Falls back to the default palette if the terminal doesn't answer.
fn query_palette() -> Vec<u8> {
    use std::time::{Duration, Instant};

    if std::env::var("TWP_NO_QUERY").is_ok() {
        return Vec::new();
    }

    let stdin = io::stdin();
    // Temporarily make reads time out (VMIN=0, VTIME=1 → 0.1s) so we don't
    // block forever on a terminal that never answers.
    let saved = termios::tcgetattr(&stdin).ok();
    if let Some(s) = &saved {
        let mut t = s.clone();
        t.control_chars[libc::VMIN as usize] = 0;
        t.control_chars[libc::VTIME as usize] = 1;
        let _ = termios::tcsetattr(&stdin, SetArg::TCSANOW, &t);
    }

    // Emit the queries.
    let mut q = Vec::new();
    for i in 0..16u8 {
        q.extend_from_slice(format!("\x1b]4;{i};?\x07").as_bytes());
    }
    q.extend_from_slice(b"\x1b]10;?\x07\x1b]11;?\x07");
    {
        let mut out = io::stdout();
        let _ = out.write_all(&q);
        let _ = out.flush();
    }

    // Read replies until we have all 18 (16 + fg + bg) or hit the deadline.
    let deadline = Instant::now() + Duration::from_millis(300);
    let mut acc: Vec<u8> = Vec::new();
    {
        let mut handle = stdin.lock();
        let mut buf = [0u8; 4096];
        while Instant::now() < deadline {
            match handle.read(&mut buf) {
                Ok(0) => {
                    if !acc.is_empty() {
                        break;
                    }
                }
                Ok(n) => acc.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
            if acc.windows(4).filter(|w| *w == b"rgb:").count() >= 18 {
                break;
            }
        }
    }

    if let Some(s) = &saved {
        let _ = termios::tcsetattr(&io::stdin(), SetArg::TCSANOW, s);
    }

    let (palette, leftover) = parse_osc_palette(&acc);
    render::set_palette(palette);
    leftover
}

/// Split accumulated terminal input into OSC color responses (folded into a
/// palette) and everything else (returned as leftover).
fn parse_osc_palette(data: &[u8]) -> (render::Palette, Vec<u8>) {
    let def = render::default_palette();
    let mut base: [[u8; 3]; 16] = def.ansi[..16].try_into().unwrap();
    let mut fg = def.fg;
    let mut bg = def.bg;
    let mut leftover = Vec::new();

    let mut i = 0;
    while i < data.len() {
        if data[i] == 0x1b && data.get(i + 1) == Some(&b']') {
            let body_start = i + 2;
            let mut j = body_start;
            let mut term_len = 0;
            while j < data.len() {
                if data[j] == 0x07 {
                    term_len = 1;
                    break;
                }
                if data[j] == 0x1b && data.get(j + 1) == Some(&b'\\') {
                    term_len = 2;
                    break;
                }
                j += 1;
            }
            if term_len == 0 {
                leftover.extend_from_slice(&data[i..]);
                break;
            }
            parse_osc_body(&data[body_start..j], &mut base, &mut fg, &mut bg);
            i = j + term_len;
        } else {
            leftover.push(data[i]);
            i += 1;
        }
    }
    (render::palette_from_base(base, fg, bg), leftover)
}

fn parse_osc_body(body: &[u8], base: &mut [[u8; 3]; 16], fg: &mut [u8; 3], bg: &mut [u8; 3]) {
    let Ok(s) = std::str::from_utf8(body) else { return };
    if let Some(rest) = s.strip_prefix("4;") {
        if let Some((idx, rgb)) = rest.split_once(';') {
            if let (Ok(i), Some(c)) = (idx.parse::<usize>(), parse_osc_rgb(rgb)) {
                if i < 16 {
                    base[i] = c;
                }
            }
        }
    } else if let Some(rgb) = s.strip_prefix("10;") {
        if let Some(c) = parse_osc_rgb(rgb) {
            *fg = c;
        }
    } else if let Some(rgb) = s.strip_prefix("11;") {
        if let Some(c) = parse_osc_rgb(rgb) {
            *bg = c;
        }
    }
}

/// Parse `rgb:RRRR/GGGG/BBBB` (or 1–4 hex digits per channel), scaling each to
/// 8 bits.
fn parse_osc_rgb(s: &str) -> Option<[u8; 3]> {
    let s = s.strip_prefix("rgb:")?;
    let mut it = s.split('/');
    let chan = |h: &str| -> Option<u8> {
        let v = u32::from_str_radix(h, 16).ok()?;
        let max = (1u32 << (4 * h.len() as u32)) - 1;
        Some(((v * 255 + max / 2) / max) as u8)
    };
    Some([chan(it.next()?)?, chan(it.next()?)?, chan(it.next()?)?])
}

fn enter_raw_mode() -> Option<Termios> {
    let stdin = io::stdin();
    let original = termios::tcgetattr(&stdin).ok()?;
    let mut raw = original.clone();
    termios::cfmakeraw(&mut raw);
    termios::tcsetattr(&stdin, SetArg::TCSANOW, &raw).ok()?;
    Some(original)
}

fn restore_termios(original: &Termios) {
    let stdin = io::stdin();
    let _ = termios::tcsetattr(&stdin, SetArg::TCSANOW, original);
}

fn run() -> io::Result<i32> {
    let mut args = env::args().skip(1);
    let shell = args
        .next()
        .or_else(|| env::var("SHELL").ok())
        .unwrap_or_else(|| "/bin/bash".to_string());
    let extra_args: Vec<String> = args.collect();

    // Block SIGWINCH on the main thread; threads spawned below inherit the
    // blocked mask, and a dedicated watcher will synchronously sigwait() it.
    let mut winch_set = SigSet::empty();
    winch_set.add(Signal::SIGWINCH);
    winch_set
        .thread_block()
        .map_err(|e| io::Error::other(format!("sigprocmask: {e}")))?;

    let pty_system = native_pty_system();
    let ws = current_winsize();

    // Derive per-cell pixel dimensions from the terminal's reported
    // pixel size. Kitty populates ws_xpixel/ws_ypixel; terminals that
    // don't will leave them 0 and render.rs falls back to defaults.
    if ws.ws_col > 0 && ws.ws_row > 0 && ws.ws_xpixel > 0 && ws.ws_ypixel > 0 {
        let px_col = ws.ws_xpixel as u32 / ws.ws_col as u32;
        let px_row = ws.ws_ypixel as u32 / ws.ws_row as u32;
        render::set_cell_pixels(px_col, px_row);
    }

    let pair = pty_system
        .openpty(pty_size_from_winsize(&ws))
        .map_err(|e| io::Error::other(format!("openpty: {e}")))?;

    let mut cmd = CommandBuilder::new(&shell);
    for arg in &extra_args {
        cmd.arg(arg);
    }
    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| io::Error::other(format!("spawn: {e}")))?;

    // Release the slave fd in the parent so the master sees EOF on child exit.
    drop(pair.slave);

    let master = pair.master;
    let writer = master
        .take_writer()
        .map_err(|e| io::Error::other(format!("take_writer: {e}")))?;
    let reader = master
        .try_clone_reader()
        .map_err(|e| io::Error::other(format!("try_clone_reader: {e}")))?;
    let master: Arc<Mutex<Box<dyn MasterPty + Send>>> = Arc::new(Mutex::new(master));

    let original_termios = enter_raw_mode();

    // Query the terminal's colour palette (OSC 4/10/11) so widgets can use
    // `term(fg)`/`term(bg)`/`term(N)`. Done before the I/O threads start, while
    // we still have exclusive access to stdin/stdout. Any non-response bytes
    // (stray user input) are returned and replayed to the child.
    let stdin_leftover = query_palette();

    // stdin -> PTY master
    {
        let mut writer = writer;
        let leftover = stdin_leftover;
        thread::spawn(move || {
            if !leftover.is_empty() {
                let _ = writer.write_all(&leftover);
                let _ = writer.flush();
            }
            let mut buf = [0u8; 4096];
            let stdin = io::stdin();
            let mut stdin = stdin.lock();
            loop {
                match stdin.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if writer.write_all(&buf[..n]).is_err() {
                            break;
                        }
                        let _ = writer.flush();
                    }
                    Err(_) => break,
                }
            }
        });
    }

    // PTY master -> filter -> stdout. Hold the handle so we can join it on
    // exit; otherwise the process can race ahead of pending output (e.g.
    // when the child runs `printf '\x1b_twp;foo\x1b\\' && exit` and the
    // bytes are still buffered in the master when wait() returns).
    let reader_handle = {
        let mut reader = reader;
        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            let mut out = Vec::with_capacity(8192);
            let mut filter = parser::Filter::new();
            let mut cache = cache::Cache::new();
            let stdout = io::stdout();
            let mut stdout = stdout.lock();
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        out.clear();
                        filter.process(&buf[..n], &mut out, |payload, out| {
                            handle_twp(&mut cache, payload, out);
                        });
                        if stdout.write_all(&out).is_err() {
                            break;
                        }
                        let _ = stdout.flush();
                    }
                    Err(_) => break,
                }
            }
        })
    };

    // SIGWINCH watcher: forward host terminal resizes to the PTY.
    {
        let master = Arc::clone(&master);
        thread::spawn(move || {
            let mut set = SigSet::empty();
            set.add(Signal::SIGWINCH);
            loop {
                match set.wait() {
                    Ok(_) => {
                        let ws = current_winsize();
                        if let Ok(m) = master.lock() {
                            let _ = m.resize(pty_size_from_winsize(&ws));
                        }
                    }
                    Err(nix::errno::Errno::EINTR) => continue,
                    Err(_) => break,
                }
            }
        });
    }

    let status = child
        .wait()
        .map_err(|e| io::Error::other(format!("wait: {e}")))?;

    // Drain any output the child wrote before exiting. The slave fd is
    // released when the child exits, so the master.read() inside the
    // reader thread will see EOF and exit shortly.
    let _ = reader_handle.join();

    if let Some(t) = &original_termios {
        restore_termios(t);
    }

    Ok(status.exit_code() as i32)
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code.clamp(0, 255) as u8),
        Err(e) => {
            eprintln!("twp-proxy: {e}");
            ExitCode::from(1)
        }
    }
}
