//! `cargo xtask verify` — fmt + clippy + workspace test (ADO-0).
//! `cargo xtask autostart install|uninstall|status` — ADO-6 (GSV-watchdog pattern).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

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

fn vbs_path(root: &Path) -> PathBuf {
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

fn vbs_body(exe: &Path, root: &Path) -> String {
    let exe = win_path(exe);
    let root = win_path(root);
    format!(
        "Set fso = CreateObject(\"Scripting.FileSystemObject\")\n\
Set sh = CreateObject(\"WScript.Shell\")\n\
exe = \"{exe}\"\n\
root = \"{root}\"\n\
log = root & \"\\data\\adoreble-watchdog.log\"\n\
Do\n\
  On Error Resume Next\n\
  Set http = CreateObject(\"WinHttp.WinHttpRequest.5.1\")\n\
  http.Open \"GET\", \"http://127.0.0.1:9770/health\", False\n\
  http.SetTimeouts 1500, 1500, 1500, 1500\n\
  http.Send\n\
  code = 0\n\
  If Not Err.Number = 0 Then code = 0 Else code = http.Status\n\
  Err.Clear\n\
  If code <> 200 Then\n\
    sh.Run \"\"\"\" & exe & \"\"\"\", 0, False\n\
    WScript.Sleep 10000\n\
  Else\n\
    WScript.Sleep 15000\n\
  End If\n\
Loop\n"
    )
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
            let Some(exe) = pick_api_exe(&root) else {
                eprintln!(
                    "adoreble-xtask autostart: adoreble-api.exe not found (target/release or target/debug)"
                );
                return ExitCode::FAILURE;
            };
            let _ = std::fs::create_dir_all(root.join("scripts"));
            let vbs = vbs_path(&root);
            if let Err(e) = std::fs::write(&vbs, vbs_body(&exe, &root)) {
                eprintln!("adoreble-xtask autostart: write {}: {e}", vbs.display());
                return ExitCode::FAILURE;
            }
            let wscript = r"C:\Windows\System32\wscript.exe";
            let tr = format!(
                "\"{wscript}\" \"{}\"",
                vbs.to_string_lossy().replace('/', "\\")
            );
            if try_schtasks(&tr) {
                println!("adoreble-autostart: schtasks AdorebleWiFi (ONLOGON)\nTR={tr}");
                return ExitCode::SUCCESS;
            }
            if try_hkcu_run(&tr) {
                println!("adoreble-autostart: HKCU Run AdorebleWiFi\nTR={tr}");
                return ExitCode::SUCCESS;
            }
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
            let _ = std::fs::remove_file(vbs_path(&root));
            println!("adoreble-autostart: uninstalled (task + Run key removed; vbs removed)");
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
        "autostart" => autostart(std::env::args().nth(2).as_deref().unwrap_or("status")),
        _ => {
            eprintln!(
                "unknown xtask task: {cmd} (expected: verify | autostart install|uninstall|status)"
            );
            ExitCode::FAILURE
        }
    }
}
