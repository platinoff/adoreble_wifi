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
    /// Mobile Hotspot (WinRT Tethering) — ADO-3.
    Hotspot {
        #[command(subcommand)]
        action: HotspotAction,
    },
    /// Print the loopback API health JSON to stdout.
    Status,
    /// Print plugin + toolchain versions.
    Version,
}

#[derive(Subcommand)]
enum HotspotAction {
    /// Configure + start the hotspot (SSID/passphrase auto-generated).
    Start {
        /// Override SSID (default: from settings or generated `GSV-Peers-xxxx`).
        #[arg(long)]
        ssid: Option<String>,
        /// Override WPA2 passphrase (default: from settings or generated).
        /// Never logged or printed.
        #[arg(long)]
        passphrase: Option<String>,
    },
    /// Stop the hotspot (STA profile untouched).
    Stop,
    /// Show live hotspot status (JSON, redacted).
    Status,
    /// Generate `data/peer-join.html` with Wi-Fi + mesh-page QR codes.
    Qr {
        /// Output path (default: data/peer-join.html).
        #[arg(long, default_value = "data/peer-join.html")]
        out: String,
    },
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
        Command::Hotspot { action } => match action {
            HotspotAction::Start { ssid, passphrase } => {
                let data_dir = std::path::PathBuf::from(
                    std::env::var("ADOREBLE_DATA_DIR").unwrap_or_else(|_| "data".to_string()),
                );
                let path = data_dir.join("adoreble.json");
                let mut settings = adoreble_core::load_settings(&path)?;
                if let Some(s) = ssid {
                    settings.hotspot.ssid = s;
                }
                if let Some(p) = passphrase {
                    settings.hotspot.password = p;
                }
                let (ssid, passphrase) = adoreble_core::ensure_hotspot_secrets(&mut settings);
                adoreble_core::persist_settings(&path, &settings)?;
                eprintln!(
                    "settings saved to {} (enabled, mode=hotspot)",
                    path.display()
                );
                #[cfg(windows)]
                {
                    let status = adoreble_windows::hotspot_start(&ssid, &passphrase)?;
                    println!("{}", serde_json::to_string_pretty(&status)?);
                }
                #[cfg(not(windows))]
                {
                    let _ = (&ssid, &passphrase);
                    println!("{{\"ok\": false, \"error\": \"hotspot is Windows-only\"}}");
                }
            }
            HotspotAction::Stop => {
                #[cfg(windows)]
                {
                    let status = adoreble_windows::hotspot_stop()?;
                    println!("{}", serde_json::to_string_pretty(&status)?);
                }
                #[cfg(not(windows))]
                println!("{{\"ok\": false, \"error\": \"hotspot is Windows-only\"}}");
            }
            HotspotAction::Status => {
                #[cfg(windows)]
                {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&adoreble_windows::hotspot_status())?
                    );
                }
                #[cfg(not(windows))]
                println!("{{\"ok\": false, \"error\": \"hotspot is Windows-only\"}}");
            }
            HotspotAction::Qr { out } => {
                let data_dir = std::path::PathBuf::from(
                    std::env::var("ADOREBLE_DATA_DIR").unwrap_or_else(|_| "data".to_string()),
                );
                let path = data_dir.join("adoreble.json");
                let settings = adoreble_core::load_settings(&path)?;
                let ssid = settings.hotspot.ssid.clone();
                let passphrase = settings.hotspot.password.clone();
                if ssid.is_empty() || passphrase.is_empty() {
                    return Err(
                        "no hotspot secrets yet — run `adoreble-wifi hotspot start` first".into(),
                    );
                }
                #[cfg(windows)]
                let gateway = adoreble_windows::detect_gateway_ip()
                    .unwrap_or_else(|| "192.168.137.1".to_string());
                #[cfg(not(windows))]
                let gateway = "192.168.137.1".to_string();
                let html = render_peer_join_html(&ssid, &passphrase, &gateway);
                std::fs::create_dir_all(
                    std::path::Path::new(&out)
                        .parent()
                        .unwrap_or(std::path::Path::new(".")),
                )
                .ok();
                std::fs::write(&out, &html)?;
                println!("peer onboarding page written to {out}");
                println!(
                    "Wi-Fi SSID: {ssid} · gateway: {gateway} · mesh page: {}",
                    adoreble_core::mesh_url(&gateway)
                );
            }
        },
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
    use std::os::windows::process::CommandExt;
    // CREATE_NO_WINDOW — netsh must not flash a console window.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("netsh")
        .creation_flags(CREATE_NO_WINDOW)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .output()?;
    if !out.status.success() {
        return Err(format!("netsh {} failed with {}", args.join(" "), out.status).into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Build the self-contained peer onboarding page (QR codes inline as SVG).
fn render_peer_join_html(ssid: &str, passphrase: &str, gateway: &str) -> String {
    use qrcode::QrCode;
    let wifi_svg = QrCode::new(adoreble_core::wpa_qr_payload(ssid, passphrase))
        .map(|q| {
            q.render::<qrcode::render::svg::Color>()
                .min_dimensions(220, 220)
                .quiet_zone(true)
                .build()
        })
        .unwrap_or_else(|_| "<p>QR encode failed</p>".to_string());
    let mesh_url = adoreble_core::mesh_url(gateway);
    let mesh_svg = QrCode::new(&mesh_url)
        .map(|q| {
            q.render::<qrcode::render::svg::Color>()
                .min_dimensions(220, 220)
                .quiet_zone(true)
                .build()
        })
        .unwrap_or_else(|_| "<p>QR encode failed</p>".to_string());
    format!(
        r#"<!doctype html>
<html lang="uk">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>GSV mesh — підключення піра</title>
<style>
  :root {{ color-scheme: dark; }}
  body {{ background:#0b1020; color:#e5e9ff; font-family:ui-sans-serif,system-ui,sans-serif; margin:0; padding:24px; }}
  h1 {{ font-size:1.4rem; color:#a78bfa; }}
  .card {{ background:rgba(255,255,255,.05); border:1px solid rgba(255,255,255,.12); border-radius:16px; padding:20px; max-width:720px; margin:16px auto; }}
  .qr {{ display:flex; gap:24px; flex-wrap:wrap; justify-content:center; }}
  .qr figure {{ margin:0; text-align:center; }}
  figcaption {{ margin-top:8px; font-size:.9rem; color:#c4b5fd; }}
  code {{ background:#000; padding:2px 8px; border-radius:6px; color:#7dd3fc; }}
  details {{ margin-top:16px; }}
  ol li {{ margin:6px 0; }}
</style>
</head>
<body>
<div class="card">
  <h1>Adoreble Wi-Fi — пір GSV mesh</h1>
  <p>SSID хотспота: <code>{ssid}</code> · шлюз: <code>{gateway}</code></p>
  <div class="qr">
    <figure>{wifi_svg}<figcaption>1) Wi-Fi</figcaption></figure>
    <figure>{mesh_svg}<figcaption>2) Меш-сторінка</figcaption></figure>
  </div>
  <ol>
    <li>Відскануй <strong>Wi-Fi</strong> QR → телефон підключиться до <code>{ssid}</code>.</li>
    <li>Відскануй <strong>Меш-сторінку</strong> QR (або відкрий <code>{mesh_url}</code>) → пір сам зареєструється в хабі.</li>
    <li>Пір з'явиться в <code>/api/wgpu/mesh</code> протягом кількох секунд.</li>
  </ol>
  <details>
    <summary>Пароль (для ручного введення)</summary>
    <p><code>{passphrase}</code></p>
  </details>
  <p style="color:#94a3b8;font-size:.8rem">Пароль не зберігаємо у логи; файл містить його — видаліть зайве після онбордингу.</p>
</div>
</body>
</html>"#
    )
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
