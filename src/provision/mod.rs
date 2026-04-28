use crate::kitchen::Kitchen;

mod dotfiles;

pub async fn run(kitchen: &Kitchen) -> Result<(), Box<dyn std::error::Error>> {
    println!("Provisioning kitchen...");
    dotfiles::provision(kitchen)?;
    // TODO mise, tailscale
    Ok(())
}
