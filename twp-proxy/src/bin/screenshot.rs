use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn usage() -> &'static str {
    "Usage: twp-screenshot [OPTIONS] --output <PATH> -- <COMMAND>...

Launches kitty on a virtual X display, runs COMMAND inside it
(optionally through twp-proxy), waits for rendering, screenshots
the kitty window, and writes the PNG to OUTPUT.

Starts Xvfb automatically if no X server is running on the target
display. Stops it when done.

Options:
  --output, -o <PATH>   Output PNG path (required)
  --display <DISP>      X display to use (default: :55)
  --proxy <PATH>        Run command through twp-proxy at PATH
  --font <FAMILY>       Font family (default: monospace)
  --font-size <N>       Font size in pt (default: 16)
  --cols <N>            Terminal width in cells (default: 60)
  --rows <N>            Terminal height in cells (default: 10)
  --bg <COLOR>          Background color (default: #0a1e24)
  --fg <COLOR>          Foreground color (default: #ecefc1)
  --class <CLASS>       X11 window class (default: twp-screenshot)
  --timeout <SECS>      Max seconds to wait for render (default: 15)
  --                    Everything after this is the command to run"
}

struct Config {
    output: PathBuf,
    display: String,
    proxy: Option<String>,
    font: String,
    font_size: String,
    cols: u32,
    rows: u32,
    bg: String,
    fg: String,
    class: String,
    timeout: u64,
    command: Vec<String>,
}

fn parse_args() -> Result<Config, String> {
    let args: Vec<String> = env::args().skip(1).collect();
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
            "--help" | "-h" => return Err(usage().to_string()),
            other => return Err(format!("unknown argument: {other}\n{}", usage())),
        }
        i += 1;
    }

    let output = output.ok_or_else(|| format!("--output is required\n{}", usage()))?;
    if command.is_empty() {
        return Err(format!("no command specified after --\n{}", usage()));
    }

    Ok(Config {
        output,
        display,
        proxy,
        font,
        font_size,
        cols,
        rows,
        bg,
        fg,
        class,
        timeout,
        command,
    })
}

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

fn start_xvfb(display: &str) -> Result<Child, String> {
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
            return Ok(child);
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err("Xvfb started but display not available after 5s".to_string())
}

fn wait_for_file(path: &std::path::Path, timeout: Duration) -> bool {
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

fn capture_window(display: &str, wid: &str, output: &std::path::Path) -> bool {
    Command::new("import")
        .args(["-window", wid, output.to_str().unwrap_or("")])
        .env("DISPLAY", display)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn screenshot_is_nonempty(path: &std::path::Path) -> bool {
    fs::metadata(path).map(|m| m.len() > 500).unwrap_or(false)
}

fn main() -> ExitCode {
    let cfg = match parse_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };

    // Start Xvfb if needed
    let mut xvfb: Option<Child> = None;
    if !display_is_available(&cfg.display) {
        match start_xvfb(&cfg.display) {
            Ok(child) => xvfb = Some(child),
            Err(e) => {
                eprintln!("twp-screenshot: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    let result = run_screenshot(&cfg);

    // Clean up Xvfb if we started it
    if let Some(ref mut child) = xvfb {
        let _ = child.kill();
        let _ = child.wait();
    }

    result
}

fn run_screenshot(cfg: &Config) -> ExitCode {
    let sig_file = {
        let mut p = env::temp_dir();
        p.push(format!("twp-screenshot-sig-{}", std::process::id()));
        p
    };
    let _ = fs::remove_file(&sig_file);

    let inner_script = format!(
        "printf '\\x1b[?25l\\x1b[2J\\x1b[H'; sleep 0.3; {}; touch {}; sleep 120",
        cfg.command.join(" "),
        sig_file.display()
    );

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

    if let Some(ref proxy) = cfg.proxy {
        kitty_args.push(proxy.clone());
    }
    kitty_args.extend(["bash".to_string(), "-c".to_string(), inner_script]);

    let mut kitty = match Command::new("kitty")
        .args(&kitty_args)
        .env("DISPLAY", &cfg.display)
        .env("LIBGL_ALWAYS_SOFTWARE", "1")
        .env("GALLIUM_DRIVER", "llvmpipe")
        .env("KITTY_DISABLE_WAYLAND", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("twp-screenshot: failed to launch kitty: {e}");
            let _ = fs::remove_file(&sig_file);
            return ExitCode::FAILURE;
        }
    };

    let timeout = Duration::from_secs(cfg.timeout);
    if !wait_for_file(&sig_file, timeout) {
        eprintln!("twp-screenshot: timed out waiting for render");
        let _ = kitty.kill();
        let _ = kitty.wait();
        let _ = fs::remove_file(&sig_file);
        return ExitCode::FAILURE;
    }

    let capture_timeout = Duration::from_secs(10);
    let capture_start = Instant::now();
    let mut captured = false;

    while capture_start.elapsed() < capture_timeout {
        thread::sleep(Duration::from_secs(1));
        if let Some(wid) = find_window(&cfg.display, &cfg.class) {
            if capture_window(&cfg.display, &wid, &cfg.output) {
                if screenshot_is_nonempty(&cfg.output) {
                    captured = true;
                    break;
                }
            }
        }
    }

    let _ = kitty.kill();
    let _ = kitty.wait();
    let _ = fs::remove_file(&sig_file);

    if !captured {
        eprintln!("twp-screenshot: failed to capture screenshot");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
