//! `cargo xtask verify` — fmt + clippy + workspace test (ADO-0).
//! `cargo xtask autostart install|uninstall|status` — ADO-6 (GSV-watchdog pattern).
//! `cargo xtask watch` — silent Rust watcher (replaces the old WSH/VBS launcher).
//!
//! Owner rule (2026-11-25): no WScript popup windows in a web-oriented
//! ecosystem. The watcher logs failures to `data/adoreble-watchdog.log` so
//! agents/models can read and fix them; it never shows a system dialog.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// CREATE_NO_WINDOW — no console flash from the respawned API.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn run(cmd: &str, args: &[&str]) {
    let status = Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .status()
        .unwrap_or_else(|e| panic!("spawn {cmd}: {e}"));
    assert!(
        status.success(),
        "`{cmd} {}' failed with {status}",
        args.join(" ")
    );
}

fn repo_root() -> PathBuf {
    std::env::current_dir().expect("cwd")
}

/// Legacy VBS launcher, kept only so `uninstall` can remove files created by
/// older versions. New installs never write or execute a VBS file.
fn legacy_vbs_path(root: &Path) -> PathBuf {
    root.join("scripts").join("adoreble-watch.vbs")
}

fn pick_api_exe(root: &Path) -> Option<PathBuf> {
    for rel in [
        "target/release/adoreble-api.exe",
        "target/debug/adoreble-api.exe",
    ] {
        let p = root.join(rel);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn win_path(p: &Path) -> String {
    p.to_string_lossy().replace('/', "\\")
}

fn watcher_exe() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("S:\\rust\\adoreble_wifi\\target\\debug\\xtask.exe"))
}

fn log_line(root: &Path, msg: &str) {
    let path = root.join("data").join("adoreble-watchdog.log");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "{secs} {msg}");
    }
}

/// Read the first HTTP response bytes; `200` only on a well-formed status line.
fn status_ok(head: &[u8]) -> bool {
    head.starts_with(b"HTTP/1.1 200") || head.starts_with(b"HTTP/1.0 200")
}

fn health_ok() -> bool {
    let addr = match "127.0.0.1:9770".parse() {
        Ok(a) => a,
        Err(_) => return false,
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(1500)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(1500)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(1500)));
    if stream
        .write_all(b"GET /health HTTP/1.1\r\nHost: 127.0.0.1:9770\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut head = [0u8; 64];
    match stream.read(&mut head) {
        Ok(n) => status_ok(&head[..n]),
        Err(_) => false,
    }
}

fn spawn_hidden(exe: &Path) {
    let mut cmd = Command::new(exe);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    match cmd.spawn() {
        Ok(_) => (),
        Err(e) => eprintln!("adoreble-watch: spawn {} failed: {e}", exe.display()),
    }
}

/// Silent loop: probe the loopback API, respawn it when down, log everything.
fn watch() -> ExitCode {
    let root = repo_root();
    let Some(exe) = pick_api_exe(&root) else {
        log_line(&root, "watch: adoreble-api.exe not found; watcher stopped");
        return ExitCode::FAILURE;
    };
    log_line(&root, &format!("watch: start exe={}", win_path(&exe)));
    loop {
        if health_ok() {
            std::thread::sleep(Duration::from_secs(15));
        } else {
            log_line(&root, "health != 200; respawn adoreble-api");
            spawn_hidden(&exe);
            std::thread::sleep(Duration::from_secs(10));
        }
    }
}

fn try_schtasks(tr: &str) -> bool {
    let st = Command::new(r"C:\Windows\System32\schtasks.exe")
        .args([
            "/create",
            "/sc",
            "onlogon",
            "/tn",
            "AdorebleWiFi",
            "/tr",
            tr,
            "/f",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    matches!(st, Ok(s) if s.success())
}

fn try_hkcu_run(tr: &str) -> bool {
    let st = Command::new(r"C:\Windows\System32\reg.exe")
        .args([
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "AdorebleWiFi",
            "/t",
            "REG_SZ",
            "/d",
            tr,
            "/f",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    matches!(st, Ok(s) if s.success())
}

fn autostart(verb: &str) -> ExitCode {
    let root = repo_root();
    match verb {
        "install" => {
            // No VBS / WScript: launch the Rust watcher directly, hidden.
            let exe = watcher_exe();
            let tr = format!("\"{}\" watch", win_path(&exe));
            // Remove any legacy WSH launcher left by older versions.
            let legacy = legacy_vbs_path(&root);
            if legacy.is_file() {
                let _ = std::fs::remove_file(&legacy);
                log_line(&root, "autostart: removed legacy VBS launcher");
            }
            if try_schtasks(&tr) {
                println!("adoreble-autostart: schtasks AdorebleWiFi (ONLOGON)\nTR={tr}");
                return ExitCode::SUCCESS;
            }
            if try_hkcu_run(&tr) {
                println!("adoreble-autostart: HKCU Run AdorebleWiFi\nTR={tr}");
                return ExitCode::SUCCESS;
            }
            log_line(
                &root,
                "autostart: could not persist (need schtasks or reg.exe)",
            );
            eprintln!("adoreble-autostart: could not persist (need schtasks or reg.exe)");
            ExitCode::FAILURE
        }
        "uninstall" => {
            let _ = Command::new(r"C:\Windows\System32\schtasks.exe")
                .args(["/delete", "/tn", "AdorebleWiFi", "/f"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            let _ = Command::new(r"C:\Windows\System32\reg.exe")
                .args([
                    "delete",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "/v",
                    "AdorebleWiFi",
                    "/f",
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            let _ = std::fs::remove_file(legacy_vbs_path(&root));
            println!(
                "adoreble-autostart: uninstalled (task + Run key removed; legacy VBS removed)"
            );
            ExitCode::SUCCESS
        }
        "status" => {
            let sch = Command::new(r"C:\Windows\System32\schtasks.exe")
                .args(["/query", "/tn", "AdorebleWiFi"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            let hkcu = Command::new(r"C:\Windows\System32\reg.exe")
                .args([
                    "query",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "/v",
                    "AdorebleWiFi",
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            let sch_ok = matches!(sch, Ok(s) if s.success());
            let hkcu_ok = matches!(hkcu, Ok(s) if s.success());
            println!(
                "schtasks: {}",
                if sch_ok { "installed" } else { "not installed" }
            );
            println!(
                "HKCU Run: {}",
                if hkcu_ok {
                    "installed"
                } else {
                    "not installed"
                }
            );
            if sch_ok || hkcu_ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        _ => {
            eprintln!("unknown autostart verb: {verb} (expected: install|uninstall|status)");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let cmd = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "verify".to_string());
    match cmd.as_str() {
        "verify" => {
            run("cargo", &["fmt", "--all", "--", "--check"]);
            run(
                "cargo",
                &[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ],
            );
            run("cargo", &["test", "--workspace"]);
            println!("verify: ok");
            ExitCode::SUCCESS
        }
        "watch" => watch(),
        "autostart" => autostart(std::env::args().nth(2).as_deref().unwrap_or("status")),
        _ => {
            eprintln!(
                "unknown xtask task: {cmd} (expected: verify | watch | autostart install|uninstall|status)"
            );
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_status_parser_accepts_only_200() {
        assert!(status_ok(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n"));
        assert!(status_ok(b"HTTP/1.0 200 OK\r\n\r\n"));
        assert!(!status_ok(b"HTTP/1.1 500 Internal Server Error\r\n\r\n"));
        assert!(!status_ok(b"garbage"));
        assert!(!status_ok(b""));
    }
}
