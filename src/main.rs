use std::path::PathBuf;

use clap::{Parser, Subcommand};
use bollard::Docker;

struct Kitchen {
    workspace_path: PathBuf,
    name: String,
}

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
        workspace: Option<PathBuf>
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Some(name) = cli.name.as_deref() {
        println!("Value for name: {name}")
    }

    match &cli.command {
        Some(Commands::Up { workspace }) => {
            up(&workspace).await
        }
        None => {}
    }
}

async fn up(workspace: &Option<PathBuf>) {
    let workspace_path = match workspace {
        Some(ws) => std::fs::canonicalize(ws).unwrap_or_else(|_| ws.clone()),
        None => std::env::current_dir().expect("failed to get current directory"),
    };

    let name = workspace_path
        .file_name()
        .expect("workspace path has no final component")
        .to_string_lossy()
        .into_owned();

    let kitchen = Kitchen { workspace_path, name };

    let container_name = format!("{}-kitchen", kitchen.name);

    let docker = Docker::connect_with_local_defaults()
        .expect("failed to connect to Docker");

    match docker.inspect_container(&container_name, None).await {
        Ok(info) => {
            let running = info
                .state
                .and_then(|s| s.running)
                .unwrap_or(false);
            if running {
                println!("Container {container_name} is already running.");
            } else {
                println!("Container {container_name} exists but is not running.");
            }
        }
        Err(_) => {
            println!("Container {container_name} does not exist");
        }
    }

    println!("Kitchen: {} at {}", kitchen.name, kitchen.workspace_path.display());
}
