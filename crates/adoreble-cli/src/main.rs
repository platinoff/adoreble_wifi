//! `adoreble-wifi` — control binary (ADO-0 scaffold).

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "adoreble-wifi", about = "Adoreble Wi-Fi control CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List Wi-Fi adapters and their capabilities (static probe for now).
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
            let list = adoreble_windows::probe();
            #[cfg(not(windows))]
            let list = adoreble_linux::probe();
            println!("{}", serde_json::to_string_pretty(&list)?);
        }
        Command::Status => {
            let url = std::env::var("ADOREBLE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:9770/health".to_string());
            let body = reqwest::get(&url).await?.text().await?;
            println!("{body}");
        }
        Command::Version => {
            println!(
                "adoreble-wifi {} (rustc {})\nlicense MIT · repository https://github.com/platinoff/adoreble_wifi",
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_RUST_VERSION")
            );
        }
    }
    Ok(())
}
