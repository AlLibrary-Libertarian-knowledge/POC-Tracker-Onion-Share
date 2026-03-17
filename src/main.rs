#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod config;
mod crypto;
mod gui;
mod link;
mod server;
mod share;
mod tor;
mod wizard; // mantido para installer + TERMS_TEXT

use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "onion_poc",
    version,
    about = "🧅 onion-poc — Sem argumentos: abre a interface gráfica."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Compartilha um arquivo (modo CLI headless).
    Share {
        #[arg(long)]
        file: PathBuf,
        #[arg(long, default_value_t = 256 * 1024)]
        chunk_size: usize,
        #[arg(long)]
        key: Option<String>,
        #[arg(long, default_value = "tor")]
        tor_path: String,
    },
    /// Baixa um arquivo compartilhado (modo CLI headless).
    Join {
        #[arg(long)]
        link: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "tor")]
        tor_path: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        // ── CLI headless ──────────────────────────────────────────────────
        Some(Command::Share {
            file,
            chunk_size,
            key,
            tor_path,
        }) => {
            init_tracing();
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            let file = file.canonicalize().context("invalid --file")?;
            let _ = key;
            let share = share::Share::new(file, chunk_size)?;
            rt.block_on(server::run_share_server(share, tor_path))?;
        }

        Some(Command::Join {
            link,
            out,
            tor_path,
        }) => {
            init_tracing();
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            let link = link::ShareLink::parse(&link)?;
            rt.block_on(server::run_join_client(link, out, tor_path))?;
        }

        // ── GUI padrão (sem argumentos) ───────────────────────────────────
        None => {
            gui::run()?;
        }
    }

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();
}
