mod bus;
mod cli;
mod config;
mod db;
mod log;
mod server;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::Arc;
use tracing::info;

use crate::config::Config;
use crate::db::Database;
use crate::server::AppState;

// ─────────────────────────────────────────────────────────────────────────────
//  CLI definition
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
#[command(
    name = "pixicode",
    about = "AI-powered development tool",
    version = env!("CARGO_PKG_VERSION"),
    long_about = None,
)]
struct Cli {
    /// Print logs to stdout
    #[arg(long, global = true)]
    print_logs: bool,

    /// Log level: debug, info, warn, error
    #[arg(long, global = true, default_value = "info", env = "PIXICODE_LOG_LEVEL")]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Start an interactive session
    Run {
        /// Working directory
        #[arg(long, env = "PIXICODE_CWD")]
        cwd: Option<std::path::PathBuf>,
        /// Model to use (default: ollama model)
        #[arg(long, env = "PIXICODE_MODEL")]
        model: Option<String>,
    },
    /// Start the HTTP server
    Serve {
        /// Port to listen on (default 4096 to match Node backend for migration)
        #[arg(long, default_value = "4096", env = "PIXICODE_PORT")]
        port: u16,
        /// Host to bind
        #[arg(long, default_value = "127.0.0.1", env = "PIXICODE_HOST")]
        host: String,
    },
    /// Manage Ollama models
    Ollama {
        #[command(subcommand)]
        action: OllamaCommands,
    },
    /// Manage authentication credentials
    Auth {
        #[command(subcommand)]
        action: AuthCommands,
    },
    /// List available AI models
    Models {
        /// Filter by provider
        #[arg(long)]
        provider: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Upgrade pixicode to the latest version
    Upgrade {
        /// Target version (default: latest)
        #[arg(long)]
        version: Option<String>,
    },
    /// Export a session
    Export {
        /// Session ID to export
        session_id: String,
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
        /// Export format: json, markdown
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Import a session
    Import {
        /// File to import
        file: std::path::PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum OllamaCommands {
    /// List available Ollama models
    List,
    /// Pull a model from Ollama
    Pull {
        /// Model name (e.g., llama3, mistral, edu-assistant)
        model: String,
    },
    /// Remove a model
    Remove {
        /// Model name
        model: String,
    },
    /// Show model info
    Show {
        /// Model name
        model: String,
    },
    /// Run a model interactively
    Run {
        /// Model name
        model: String,
        /// Optional prompt (if not provided, runs interactively)
        #[arg(trailing_var_arg = true)]
        prompt: Option<String>,
    },
    /// Check Ollama status
    Status,
}

#[derive(Debug, Subcommand)]
enum AuthCommands {
    /// Set a credential for a provider
    Set {
        /// Provider name (e.g., anthropic, openai)
        provider: String,
        /// API key or credential value
        value: String,
    },
    /// Remove a credential for a provider
    Remove {
        /// Provider name
        provider: String,
    },
    /// List configured providers
    List,
}

// ─────────────────────────────────────────────────────────────────────────────
//  Signal handling
// ─────────────────────────────────────────────────────────────────────────────

fn setup_signals(shutdown_tx: tokio::sync::broadcast::Sender<()>) {
    // SIGINT / Ctrl-C
    let tx1 = shutdown_tx.clone();
    ctrlc::set_handler(move || {
        tracing::warn!("Received SIGINT — shutting down");
        let _ = tx1.send(());
    })
    .expect("Failed to set SIGINT handler");

    // SIGTERM / SIGHUP via signal-hook on Unix
    #[cfg(unix)]
    {
        use signal_hook::consts::signal::{SIGHUP, SIGTERM};
        use signal_hook::iterator::Signals;
        let tx2 = shutdown_tx.clone();
        std::thread::spawn(move || {
            let mut signals = Signals::new([SIGHUP, SIGTERM]).expect("Failed to create signal iterator");
            for sig in signals.forever() {
                tracing::warn!("Received signal {} — shutting down", sig);
                let _ = tx2.send(());
                break;
            }
        });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Entry point
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialise logging early
    crate::log::init(&cli.log_level, cli.print_logs)?;

    info!(version = env!("CARGO_PKG_VERSION"), "pixicode starting");

    // Shutdown broadcast channel
    let (shutdown_tx, _shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);
    setup_signals(shutdown_tx.clone());

    // Load config
    let config = Config::load().await?;
    info!("Configuration loaded");

    // Open database (auto-migrates + handles opencode.db → pixicode.db)
    let db = Database::open(config.data_dir()).await?;
    info!("Database ready");

    // Dispatch sub-commands
    match cli.command {
        Commands::Run { cwd, model } => {
            let _cwd = cwd.unwrap_or_else(|| std::env::current_dir().unwrap());
            let _model = model.or_else(|| std::env::var("PIXICODE_MODEL").ok());
            
            // Use Ollama by default if no model specified
            if _model.is_none() {
                println!("🦙 Using Ollama as default provider");
                println!("   Set PIXICODE_MODEL or use --model to override");
            }
            
            info!("run command (TUI) — not yet implemented");
            // TODO: launch TUI
        }

        Commands::Serve { host, port } => {
            let state = Arc::new(AppState::new(config, db, shutdown_tx.subscribe()));
            let addr = format!("{}:{}", host, port);
            info!(addr, "Starting HTTP server");
            server::run(state, addr, shutdown_tx.subscribe()).await?;
        }

        Commands::Ollama { action } => {
            match action {
                OllamaCommands::List => {
                    match cli::ollama_cmd::list_models().await {
                        Ok(models) => {
                            println!("🦙 Available Ollama models:\n");
                            for model in models {
                                println!("  • {} ({})", 
                                    model.name, 
                                    cli::ollama_cmd::format_size(model.size));
                            }
                            println!("\n💡 Use 'pixicode run --model <name>' to use a model");
                        }
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            println!("\n💡 Make sure Ollama is running: ollama serve");
                        }
                    }
                }
                OllamaCommands::Pull { model } => {
                    if let Err(e) = cli::ollama_cmd::pull_model(&model).await {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                OllamaCommands::Remove { model } => {
                    if let Err(e) = cli::ollama_cmd::remove_model(&model).await {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                OllamaCommands::Show { model } => {
                    if let Err(e) = cli::ollama_cmd::show_model(&model).await {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                OllamaCommands::Run { model, prompt } => {
                    if let Err(e) = cli::ollama_cmd::run_model(&model, prompt.as_deref()).await {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                OllamaCommands::Status => {
                    match cli::ollama_cmd::check_ollama().await {
                        Ok(_) => {
                            println!("\n✅ Ollama is ready to use with Pixicode!");
                            println!("   Run 'pixicode ollama list' to see available models");
                        }
                        Err(e) => {
                            eprintln!("❌ {}", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
        }

        Commands::Auth { action } => {
            let mgr = pixicode_core::providers::auth::CredentialManager::keyring();
            match action {
                AuthCommands::Set { provider, value } => {
                    let key = format!("{}_api_key", provider.to_lowercase());
                    mgr.set(&key, &value).await;
                    println!("Stored credential for '{provider}' in OS keychain");
                }
                AuthCommands::Remove { provider } => {
                    let key = format!("{}_api_key", provider.to_lowercase());
                    mgr.remove(&key).await;
                    println!("Removed credential for '{provider}'");
                }
                AuthCommands::List => {
                    let keys: Vec<String> = mgr.list().await;
                    let providers: Vec<String> = keys
                        .iter()
                        .filter_map(|k| k.strip_suffix("_api_key").map(str::to_string))
                        .collect();
                    if providers.is_empty() {
                        println!("No stored credentials (use 'pixicode auth set <provider> <key>')");
                    } else {
                        println!("Stored providers: {}", providers.join(", "));
                    }
                }
            }
        }

        Commands::Models { provider, json } => {
            info!(?provider, "Listing models");
            // TODO: query provider registry
            println!("[]");
        }

        Commands::Upgrade { version } => {
            let target = version.as_deref().unwrap_or("latest");
            info!(target, "Upgrade requested");
            println!("Upgrade to {target}: not yet implemented");
        }

        Commands::Export { session_id, output, format } => {
            info!(session_id, format, "Exporting session");
            let out = match format.as_str() {
                "markdown" | "md" => crate::db::session_io::export_session_markdown(&db, &session_id)?,
                _ => crate::db::session_io::export_session_json(&db, &session_id)?,
            };
            if let Some(p) = output {
                tokio::fs::write(&p, &out).await?;
                info!(path = %p.display(), "Exported");
            } else {
                println!("{}", out);
            }
        }

        Commands::Import { file } => {
            info!(?file, "Importing session");
            let id = crate::db::session_io::import_session_from_path(&db, file.as_path())?;
            info!(session_id = %id, "Imported");
            println!("Imported session {}", id);
        }
    }

    Ok(())
}
