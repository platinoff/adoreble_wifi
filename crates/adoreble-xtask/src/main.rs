//! `cargo xtask verify` — fmt + clippy + workspace test (ADO-0).

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
        _ => {
            eprintln!("unknown xtask task: {cmd} (expected: verify)");
            ExitCode::FAILURE
        }
    }
}
