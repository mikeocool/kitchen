use crate::kitchen::KitchenConfig;

pub mod dotfiles;
pub mod pitchfork;
pub mod tailscale;

pub async fn onstart(kitchen: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running kitchen start hooks...");

    dotfiles::onstart(kitchen)?;
    // TODO mise, tailscale
    pitchfork::onstart(kitchen)?;
    Ok(())
}

pub async fn poststart(kitchen: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running kitchen poststart hooks...");

    tailscale::poststart(kitchen)?;

    Ok(())
}
