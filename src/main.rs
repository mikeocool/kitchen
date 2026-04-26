use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    name: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Up {
        #[arg(short, long)]
        workspace: Option<PathBuf>
    }
}

fn main() {
    let cli = Cli::parse();

    if let Some(name) = cli.name.as_deref() {
        println!("Value for name: {name}")
    }

    match &cli.command {
        Some(Commands::Up { workspace }) => {
            up(&workspace)
        }
        None => {}
    }
}

fn up(workspace: &Option<PathBuf>) {
    if let Some(ws) = workspace.as_deref() {
        println!("Workspace: {}", ws.display())
    } else {
        println!("No workspace specified")
    }
}
