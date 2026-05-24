use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod config;
mod core;
mod gui;
mod tui;

#[derive(Parser, Debug)]
#[command(name = "explorerust", about = "A modern dual-interface file explorer for Linux")]
pub struct Args {
    /// Launch in TUI (terminal) mode
    #[arg(long, short)]
    pub tui: bool,

    /// Starting directory
    pub path: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let start_path = args
        .path
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")));

    if args.tui {
        tui::run(start_path)
    } else {
        gui::run(start_path)
    }
}
