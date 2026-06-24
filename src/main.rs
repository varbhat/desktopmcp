use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod atspi;
mod dbus;
mod portal;
mod server;
mod session;
mod tools;
mod pipewire_capture;

#[derive(Parser, Debug)]
#[command(name = "desktopmcp")]
#[command(about = "Desktop MCP - MCP server for the Linux desktop", long_about = None)]
struct Args {
    /// Transport mode
    #[arg(short, long, value_enum, default_value = "stdio")]
    transport: Transport,

    /// HTTP server bind address (only for http transport)
    #[arg(long, default_value = "127.0.0.1:3000")]
    bind: String,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum Transport {
    /// Standard input/output (for local MCP clients)
    Stdio,
    /// HTTP server with SSE (for remote MCP clients)
    Http,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // For stdio mode, logs must go to stderr (stdout is for JSON-RPC)
    // For HTTP mode, logs can go to stdout
    let writer = if matches!(args.transport, Transport::Stdio) {
        tracing_subscriber::fmt::writer::BoxMakeWriter::new(std::io::stderr)
    } else {
        tracing_subscriber::fmt::writer::BoxMakeWriter::new(std::io::stdout)
    };

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("desktopmcp={}", args.log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(writer))
        .init();

    tracing::info!("Starting Desktop MCP server");
    tracing::info!("Transport mode: {:?}", args.transport);

    match args.transport {
        Transport::Stdio => {
            server::run_stdio().await?;
        }
        Transport::Http => {
            server::run_http(&args.bind).await?;
        }
    }

    Ok(())
}
