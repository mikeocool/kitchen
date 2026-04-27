use bollard::Docker;
use bollard::errors::Error as BollardError;
use bollard::models::{ContainerCreateBody, HostConfig};
use bollard::query_parameters::{
    CreateContainerOptionsBuilder, LogsOptionsBuilder, RemoveContainerOptionsBuilder,
};
use futures_util::StreamExt;

use crate::kitchen::Kitchen;

const READY_SENTINEL: &str = "Kitchen is ready to cook";

pub async fn run(docker: &Docker, kitchen: &Kitchen) -> Result<(), bollard::errors::Error> {
    let container_name = kitchen.container_name();

    let options = CreateContainerOptionsBuilder::default()
        .name(&container_name)
        .build();

    let body = ContainerCreateBody {
        image: Some(container_name.clone()),
        hostname: Some(container_name.clone()),
        env: Some(vec![kitchen.kitchen_workspace_env()]),
        host_config: Some(HostConfig {
            binds: Some(vec![kitchen.workspace_mount()]),
            ..Default::default()
        }),
        ..Default::default()
    };

    docker.create_container(Some(options), body).await?;
    docker.start_container(&container_name, None).await?;

    let log_options = LogsOptionsBuilder::default()
        .follow(true)
        .stdout(true)
        .stderr(true)
        .build();

    let mut stream = docker.logs(&container_name, Some(log_options));

    while let Some(result) = stream.next().await {
        match result {
            Ok(output) => {
                let line = output.to_string();
                print!("{line}");
                if line.contains(READY_SENTINEL) {
                    break;
                }
            }
            Err(e) => return Err(e),
        }
    }

    Ok({})
}

pub async fn remove(docker: &Docker, container_name: &str) -> Result<(), String> {
    let options = RemoveContainerOptionsBuilder::default().force(true).build();

    match docker.remove_container(container_name, Some(options)).await {
        Ok(_) => {
            println!("Removed container {container_name}");
            Ok(())
        }
        Err(BollardError::DockerResponseServerError {
            status_code: 404, ..
        }) => Err(format!("Container {container_name} does not exist")),
        Err(e) => Err(e.to_string()),
    }
}
