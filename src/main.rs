mod models;
mod database;
mod commands;

use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "dockerops")]
#[command(about = "A Docker Compose file watcher and manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Watch a GitHub repository for file changes
    Watch {
        /// GitHub repository URL to watch (e.g., https://github.com/user/repo)
        url: String,
    },
    /// Reconcile the database and show current state
    Reconcile,
    /// Stop the application
    Stop,
    /// Show version information
    Version,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Only initialize database for commands that need it
    match &cli.command {
        Commands::Watch { url } => {
            let database_url = "sqlite:dockerops.db";
            let db = database::Database::new(database_url).await?;
            let commands = commands::Commands::new(db);
            commands.watch(url).await?;
        }
        Commands::Reconcile => {
            let database_url = "sqlite:dockerops.db";
            let db = database::Database::new(database_url).await?;
            let commands = commands::Commands::new(db);
            commands.reconcile().await?;
        }
        Commands::Stop => {
            // Stop command doesn't need database
            let commands = commands::Commands::new(database::Database::new("sqlite:dockerops.db").await?);
            commands.stop().await?;
        }
        Commands::Version => {
            // Version command doesn't need database
            commands::Commands::show_version();
        }
    }

    Ok(())
} 