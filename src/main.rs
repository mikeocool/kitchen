use bollard::Docker;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use kitchen::KitchenConfig;

mod config;
mod container;
mod extensions;
mod image;
mod kitchen;



#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
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
    ContainerPoststart,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Up { workspace }) => up(&workspace).await,
        Some(Commands::Down { workspace }) => down(&workspace).await,
        Some(Commands::Build { workspace }) => build(&workspace).await,
        Some(Commands::Shell { workspace }) => shell(&workspace).await,
        Some(Commands::ContainerProvision) => container_provision().await,
        Some(Commands::ContainerPoststart) => container_poststart().await,
        None => {
            eprintln!("Error: no subcommand provided. Use --help for usage.");
            std::process::exit(1);
        }
    }
}

async fn build(workspace: &Option<PathBuf>) {
    let kitchen = KitchenConfig::from_workspace(&workspace).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        std::process::exit(1);
    });
    println!("Building {}...", kitchen.name);

    image::build(&kitchen).await;
}

async fn up(workspace: &Option<PathBuf>) {
    let kitchen = KitchenConfig::from_workspace(&workspace).unwrap_or_else(|e| {
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

    image::build(&kitchen).await;
    container::run(&docker, &kitchen)
        .await
        .expect("failed to start containeo");

    if let Err(e) = container::exec(
        &docker,
        &kitchen,
        vec!["/usr/local/bin/kitchen", "container-poststart"],
    )
    .await
    {
        eprintln!("Error running poststart: {e}");
        std::process::exit(1);
    }

    println!(
        "Kitchen: {} at {}",
        kitchen.name,
        kitchen.local_workspace_path.display()
    );
}

async fn down(workspace: &Option<PathBuf>) {
    let kitchen = KitchenConfig::from_workspace(&workspace).unwrap_or_else(|e| {
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
    let kitchen = KitchenConfig::from_workspace(workspace).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        std::process::exit(1);
    });

    let docker = Docker::connect_with_local_defaults().expect("failed to connect to Docker");

    if let Err(e) = container::shell(&docker, &kitchen).await {
        eprint!("Error: {e}");
        std::process::exit(1);
    }
}

async fn container_provision() {
    // TODO stronger sentinel that we're in a kitch container
    let workspace_path = std::env::var("KITCHEN_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().expect("no workspace configured"));

    let kitchen = KitchenConfig::from_workspace(&Some(workspace_path)).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        std::process::exit(1);
    });

    // TODO also do install

    if let Err(e) = extensions::onstart(&kitchen).await {
        eprint!("Error: {e}");
        std::process::exit(1);
    }
}

async fn container_poststart() {
    let workspace_path = std::env::var("KITCHEN_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().expect("no workspace configured"));

    let kitchen = KitchenConfig::from_workspace(&Some(workspace_path)).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        std::process::exit(1);
    });

    if let Err(e) = extensions::poststart(&kitchen).await {
        eprint!("Error: {e}");
        std::process::exit(1);
    }
}
