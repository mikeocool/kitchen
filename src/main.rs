use bollard::Docker;
use clap::{Parser, Subcommand};
use kitchen::Kitchen;
use std::path::PathBuf;

mod config;
mod container;
mod image;
mod kitchen;
mod provision;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    name: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Up { workspace: Option<PathBuf> },
    Down { workspace: Option<PathBuf> },
    Build { workspace: Option<PathBuf> },
    Shell { workspace: Option<PathBuf> },
    ContainerProvision,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Some(name) = cli.name.as_deref() {
        println!("Value for name: {name}")
    }

    match &cli.command {
        Some(Commands::Up { workspace }) => up(&workspace).await,
        Some(Commands::Down { workspace }) => down(&workspace).await,
        Some(Commands::Build { workspace }) => build(&workspace).await,
        Some(Commands::Shell { workspace }) => shell(&workspace).await,
        Some(Commands::ContainerProvision) => container_provision().await,
        None => {}
    }
}

fn get_kitchen(workspace: &Option<PathBuf>) -> Result<Kitchen, Box<dyn std::error::Error>> {
    let workspace_path = match workspace {
        Some(ws) => std::fs::canonicalize(ws).unwrap_or_else(|_| ws.clone()),
        None => std::env::current_dir().expect("failed to get current directory"),
    };

    let name = workspace_path
        .file_name()
        .expect("workspace path has no final component")
        .to_string_lossy()
        .into_owned();

    let config = config::load(&workspace_path)?;

    Ok(Kitchen {
        workspace_path,
        name,
        config,
    })
}

async fn build(workspace: &Option<PathBuf>) {
    let kitchen = get_kitchen(&workspace).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        std::process::exit(1);
    });
    println!("Building {}...", kitchen.name);

    image::build(&kitchen.container_name()).await;
}

async fn up(workspace: &Option<PathBuf>) {
    let kitchen = get_kitchen(&workspace).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        std::process::exit(1);
    });

    let container_name = kitchen.container_name();

    let docker = Docker::connect_with_local_defaults().expect("failed to connect to Docker");

    match docker.inspect_container(&container_name, None).await {
        Ok(info) => {
            let running = info.state.and_then(|s| s.running).unwrap_or(false);
            if running {
                println!("Container {container_name} is already running.");
                return;
            } else {
                println!("Container {container_name} exists but is not running.");
                return;
            }
        }
        Err(_) => {}
    }

    image::build(&kitchen.container_name()).await;
    container::run(&docker, &kitchen)
        .await
        .expect("failed to start containeo");

    println!(
        "Kitchen: {} at {}",
        kitchen.name,
        kitchen.workspace_path.display()
    );
}

async fn down(workspace: &Option<PathBuf>) {
    let kitchen = get_kitchen(&workspace).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        std::process::exit(1);
    });
    let container_name = kitchen.container_name();
    let docker = Docker::connect_with_local_defaults().expect("failed to connect to Docker");
    // TODO if running, run scripts to handle cleanup -- like disconnecting from tailnet (or just have signal handler?)
    if let Err(e) = container::remove(&docker, &container_name).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn shell(workspace: &Option<PathBuf>) {
    let kitchen = get_kitchen(workspace).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        std::process::exit(1);
    });

    let docker = Docker::connect_with_local_defaults().expect("failed to connect to Docker");

    match container::shell(&docker, &kitchen).await {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprint!("Error: {e}");
            std::process::exit(1);
        }
    }
}

async fn container_provision() {
    // TODO stronger sentinel that we're in a kitch container
    let workspace_path = std::env::var("KITCHEN_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().expect("no workspace configured"));

    let kitchen = get_kitchen(&Some(workspace_path)).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        std::process::exit(1);
    });

    if let Err(e) = provision::run(&kitchen).await {
        eprint!("Error: {e}");
        std::process::exit(1);
    }
}
