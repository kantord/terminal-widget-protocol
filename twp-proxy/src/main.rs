mod cache;
mod kitty;
mod parser;
mod render;

use std::env;
use std::io::{self, Read, Write};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::thread;

use nix::libc;
use nix::sys::signal::{SigSet, Signal};
use nix::sys::termios::{self, SetArg, Termios};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};

// Cell dimensions of the rendered widget block. Hardcoded for hello-world.
const WIDGET_COLS: u32 = 20;
const WIDGET_ROWS: u32 = 10;

fn handle_twp(cache: &mut cache::Cache, payload: &[u8], out: &mut Vec<u8>) {
    let image_id = cache::image_id_for(payload);
    if cache.mark_transmitted(image_id) {
        let png = match payload {
            b"foo" => render::render_triangle(),
            b"bar" => render::render_circle(),
            other => {
                eprintln!(
                    "twp-proxy: unknown payload `{}`; ignoring",
                    String::from_utf8_lossy(other)
                );
                return;
            }
        };
        out.extend_from_slice(&kitty::transmit_image(
            image_id,
            &png,
            WIDGET_COLS,
            WIDGET_ROWS,
        ));
    }
    out.extend_from_slice(&kitty::placeholder_cells(image_id, WIDGET_COLS, WIDGET_ROWS));
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
    let shell = env::args()
        .nth(1)
        .or_else(|| env::var("SHELL").ok())
        .unwrap_or_else(|| "/bin/bash".to_string());

    // Block SIGWINCH on the main thread; threads spawned below inherit the
    // blocked mask, and a dedicated watcher will synchronously sigwait() it.
    let mut winch_set = SigSet::empty();
    winch_set.add(Signal::SIGWINCH);
    winch_set
        .thread_block()
        .map_err(|e| io::Error::other(format!("sigprocmask: {e}")))?;

    let pty_system = native_pty_system();
    let ws = current_winsize();
    let pair = pty_system
        .openpty(pty_size_from_winsize(&ws))
        .map_err(|e| io::Error::other(format!("openpty: {e}")))?;

    let cmd = CommandBuilder::new(&shell);
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

    // stdin -> PTY master
    {
        let mut writer = writer;
        thread::spawn(move || {
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
