use clap::{Parser, Subcommand};
use xbot_core::{BUS_NAME, DAEMON_INTERFACE, OBJECT_PATH};

#[derive(Parser)]
#[command(
    name = "crux",
    about = "CruxOS command line",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Describe commands as JSON for tool discovery.
    Help {
        #[arg(long)]
        json: bool,
    },
    Xbot {
        #[command(subcommand)]
        command: XbotCommand,
    },
}

#[derive(Subcommand)]
enum XbotCommand {
    /// Show XBot daemon status.
    Status {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, value_name = "FORMAT", value_parser = ["json"])]
        progress: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Help { json: true } => {
            println!(
                "{}",
                serde_json::json!({
                    "schema": "crux.help/1",
                    "commands": [{
                        "name": "xbot status",
                        "description": "Show XBot daemon status",
                        "options": ["--json", "--dry-run", "--progress=json"]
                    }]
                })
            );
        }
        Command::Help { json: false } => {
            println!("Available commands: xbot status (use --json for machine-readable help)");
        }
        Command::Xbot {
            command:
                XbotCommand::Status {
                    json,
                    dry_run: _,
                    progress: _,
                },
        } => {
            let connection = zbus::Connection::session().await?;
            let proxy =
                zbus::Proxy::new(&connection, BUS_NAME, OBJECT_PATH, DAEMON_INTERFACE).await?;
            let payload: String = proxy.call("Status", &()).await?;
            if json {
                let value: serde_json::Value = serde_json::from_str(&payload)?;
                println!("{value}");
            } else {
                let value: serde_json::Value = serde_json::from_str(&payload)?;
                println!(
                    "XBot {} ({}): monitoring {}",
                    value["version"], value["phase"], value["monitoring"]
                );
            }
        }
    }
    Ok(())
}
