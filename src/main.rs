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
    /// Debug repository cache
    DebugCache,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Get database path from environment or use default
    let db_path = std::env::var("DOCKEROPS_DB_PATH")
        .unwrap_or_else(|_| {
            let home_dir = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            format!("{}/.dockerops/dockerops.db", home_dir)
        });

    // Create .dockerops directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let database_url = format!("sqlite:{}", db_path);

    // Only initialize database for commands that need it
    match &cli.command {
        Commands::Watch { url } => {
            let db = database::Database::new(&database_url).await?;
            let commands = commands::Commands::new(db);
            commands.watch(url).await?;
        }
        Commands::Reconcile => {
            let db = database::Database::new(&database_url).await?;
            let commands = commands::Commands::new(db);
            commands.reconcile().await?;
        }
        Commands::Stop => {
            let db = database::Database::new(&database_url).await?;
            let commands = commands::Commands::new(db);
            commands.stop().await?;
        }
        Commands::Version => {
            // Version command doesn't need database
            commands::Commands::show_version();
        }
        Commands::DebugCache => {
            let db = database::Database::new(&database_url).await?;
            let commands = commands::Commands::new(db);
            commands.debug_cache().await?;
        }
    }

    Ok(())
} 