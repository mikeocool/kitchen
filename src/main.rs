use std::path::PathBuf;

use bollard::Docker;
use clap::{Parser, Subcommand};
use eyre::Result;

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
    ContainerInstall,
    ContainerProvision,
    ContainerPoststart,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Up { workspace }) => up(workspace).await?,
        Some(Commands::Down { workspace }) => down(workspace).await?,
        Some(Commands::Build { workspace }) => build(workspace).await?,
        Some(Commands::Shell { workspace }) => shell(workspace).await?,
        Some(Commands::ContainerInstall) => container_install().await?,
        Some(Commands::ContainerProvision) => container_provision().await?,
        Some(Commands::ContainerPoststart) => container_poststart().await?,
        None => eyre::bail!("no subcommand provided. Use --help for usage."),
    }

    Ok(())
}

async fn build(workspace: &Option<PathBuf>) -> Result<()> {
    let kitchen = KitchenConfig::from_workspace(workspace)?;
    println!("Building {}...", kitchen.name);
    image::build(&kitchen).await?;
    Ok(())
}

async fn up(workspace: &Option<PathBuf>) -> Result<()> {
    let kitchen = KitchenConfig::from_workspace(workspace)?;
    let container_name = kitchen.container_name();
    let docker = Docker::connect_with_local_defaults()?;

    match docker.inspect_container(&container_name, None).await {
        Ok(info) => {
            let running = info.state.and_then(|s| s.running).unwrap_or(false);
            if running {
                println!("Container {container_name} is already running.");
            } else {
                println!("Container {container_name} exists but is not running.");
            }
            return Ok(());
        }
        Err(_) => {}
    }

    println!("Building {}...", kitchen.name);
    image::build(&kitchen).await?;
    container::run(&docker, &kitchen).await?;
    container::exec(
        &docker,
        &kitchen,
        vec!["/usr/local/bin/kitchen", "container-poststart"],
    )
    .await?;

    println!(
        "Kitchen: {} at {}",
        kitchen.name,
        kitchen.local_workspace_path.display()
    );
    Ok(())
}

async fn down(workspace: &Option<PathBuf>) -> Result<()> {
    let kitchen = KitchenConfig::from_workspace(workspace)?;
    let container_name = kitchen.container_name();
    let docker = Docker::connect_with_local_defaults()?;
    // TODO if running, run scripts to handle cleanup -- like disconnecting from tailnet (or just have signal handler?)
    container::remove(&docker, &container_name).await?;
    Ok(())
}

async fn shell(workspace: &Option<PathBuf>) -> Result<()> {
    let kitchen = KitchenConfig::from_workspace(workspace)?;
    let docker = Docker::connect_with_local_defaults()?;
    container::shell(&docker, &kitchen).await?;
    Ok(())
}

async fn container_install() -> Result<()> {
    // TODO stronger sentinel that we're in a kitch container
    let workspace_path = std::env::var("KITCHEN_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().expect("no workspace configured"));

    let kitchen = KitchenConfig::from_workspace(&Some(workspace_path))?;
    extensions::install(&kitchen).await?;
    Ok(())
}

async fn container_provision() -> Result<()> {
    // TODO stronger sentinel that we're in a kitch container
    let workspace_path = std::env::var("KITCHEN_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().expect("no workspace configured"));

    let kitchen = KitchenConfig::from_workspace(&Some(workspace_path))?;
    // TODO also do install
    extensions::onstart(&kitchen).await?;
    Ok(())
}

async fn container_poststart() -> Result<()> {
    let workspace_path = std::env::var("KITCHEN_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().expect("no workspace configured"));

    let kitchen = KitchenConfig::from_workspace(&Some(workspace_path))?;
    extensions::poststart(&kitchen).await?;
    Ok(())
}
