use crate::kitchen::Kitchen;

mod dotfiles;
mod tailscale;

pub async fn on_start(kitchen: &Kitchen) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running kitchen start hooks...");
    dotfiles::on_start(kitchen)?;
    // TODO mise, tailscale
    Ok(())
}

pub async fn poststart(kitchen: &Kitchen) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running kitchen poststart hooks...");

    tailscale::poststart(kitchen)?;

    Ok(())
}
