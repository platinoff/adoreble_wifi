//! `adoreble-wifi` — control binary (ADO-1: capability probes).

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "adoreble-wifi", about = "Adoreble Wi-Fi control CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List Wi-Fi adapters + drivers + tethering capability (JSON).
    Scan,
    /// Probe GSV hub candidates (read-only HTTP GET of /api/health).
    ProbeHub {
        /// Override candidates (repeatable). Defaults to the LAN-first list.
        #[arg(long)]
        candidate: Vec<String>,
    },
    /// Show what fail-open release would do (never changes networking).
    Release {
        /// Profile to fall back to.
        #[arg(long, default_value = "BELL409")]
        to: String,
    },
    /// Print the loopback API health JSON to stdout.
    Status,
    /// Print plugin + toolchain versions.
    Version,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Scan => {
            #[cfg(windows)]
            {
                let ifaces = adoreble_windows::parse_interfaces(&capture_netsh(&[
                    "wlan",
                    "show",
                    "interfaces",
                ])?);
                let drivers =
                    adoreble_windows::parse_drivers(&capture_netsh(&["wlan", "show", "drivers"])?);
                let tethering = adoreble_windows::probe_tethering();
                let out = serde_json::json!({
                    "interfaces": ifaces,
                    "drivers": drivers,
                    "tethering": tethering,
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            }
            #[cfg(not(windows))]
            {
                println!("[]");
            }
        }
        Command::ProbeHub { candidate } => {
            let urls = if candidate.is_empty() {
                adoreble_core::default_hub_candidates()
            } else {
                candidate
            };
            let results = probe_hubs(&urls).await;
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        Command::Release { to } => {
            // ADO-2: dry-run only. Actually switching profiles is owner-gated (ADO-3).
            #[cfg(windows)]
            let from = capture_netsh(&["wlan", "show", "interfaces"])
                .ok()
                .and_then(|o| adoreble_windows::current_profile_from_interfaces(&o));
            #[cfg(not(windows))]
            let from: Option<String> = None;
            let plan = adoreble_core::plan_release(from.as_deref(), &to);
            let mut out = serde_json::to_value(&plan)?;
            out["dry_run"] = serde_json::json!(true);
            out["note"] = serde_json::json!("ADO-2: plan only — host networking is NOT changed");
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        Command::Status => {
            let url = std::env::var("ADOREBLE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:9770/health".to_string());
            let body = reqwest::get(&url).await?.text().await?;
            println!("{body}");
        }
        Command::Version => {
            println!(
                "adoreble-wifi {}\nlicense MIT · repository https://github.com/platinoff/adoreble_wifi",
                env!("CARGO_PKG_VERSION")
            );
        }
    }
    Ok(())
}

#[cfg(windows)]
fn capture_netsh(args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let out = std::process::Command::new("netsh")
        .args(args)
        .stdout(std::process::Stdio::piped())
        .output()?;
    if !out.status.success() {
        return Err(format!("netsh {} failed with {}", args.join(" "), out.status).into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Read-only GSV hub probe: GET each `/api/health` with a 2s timeout.
async fn probe_hubs(urls: &[String]) -> Vec<adoreble_core::HubProbeResult> {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return urls
                .iter()
                .map(|u| adoreble_core::HubProbeResult {
                    url: u.clone(),
                    ok: false,
                    latency_ms: 0,
                    detail: format!("client build: {e}"),
                })
                .collect();
        }
    };
    let mut out = Vec::with_capacity(urls.len());
    for url in urls {
        let start = std::time::Instant::now();
        let res = client.get(url).send().await;
        let latency_ms = start.elapsed().as_millis() as u64;
        match res {
            Ok(resp) => {
                let status = resp.status();
                out.push(adoreble_core::HubProbeResult {
                    url: url.clone(),
                    ok: status.is_success(),
                    latency_ms,
                    detail: format!("HTTP {status}"),
                });
            }
            Err(e) => {
                out.push(adoreble_core::HubProbeResult {
                    url: url.clone(),
                    ok: false,
                    latency_ms,
                    detail: format!("{e}"),
                });
            }
        }
    }
    out
}
