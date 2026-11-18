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
