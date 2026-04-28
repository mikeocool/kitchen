use std::collections::HashMap;

use bollard::Docker;
use bollard::errors::Error as BollardError;
use bollard::models::{
    ContainerCreateBody, EndpointSettings, HostConfig, Mount, MountBindOptions, MountTypeEnum,
    NetworkCreateRequest, NetworkingConfig,
};
use bollard::query_parameters::{
    CreateContainerOptionsBuilder, LogsOptionsBuilder, RemoveContainerOptionsBuilder,
};
use futures_util::StreamExt;

use crate::kitchen::Kitchen;
mod shell;
pub use shell::shell;

const READY_SENTINEL: &str = "Kitchen is ready to cook";

async fn ensure_network(docker: &Docker, network: &str) -> Result<(), BollardError> {
    match docker.inspect_network(network, None).await {
        Ok(_) => return Ok(()),
        Err(BollardError::DockerResponseServerError {
            status_code: 404, ..
        }) => {}
        Err(e) => return Err(e),
    }
    docker
        .create_network(NetworkCreateRequest {
            name: network.to_string(),
            ..Default::default()
        })
        .await?;
    Ok(())
}

pub async fn run(docker: &Docker, kitchen: &Kitchen) -> Result<(), bollard::errors::Error> {
    let container_name = kitchen.container_name();

    let network = kitchen
        .config
        .as_ref()
        .and_then(|c| c.container.as_ref())
        .and_then(|c| c.network.as_deref());

    if let Some(network) = network {
        ensure_network(docker, network).await?;
    }

    let options = CreateContainerOptionsBuilder::default()
        .name(&container_name)
        .build();

    let networking_config = network.map(|n| NetworkingConfig {
        endpoints_config: Some(HashMap::from([(
            n.to_string(),
            EndpointSettings::default(),
        )])),
    });

    let body = ContainerCreateBody {
        image: Some(container_name.clone()),
        hostname: Some(container_name.clone()),
        env: Some(vec![kitchen.kitchen_workspace_env()]),
        networking_config,
        host_config: Some(HostConfig {
            mounts: Some(vec![
                Mount {
                    typ: Some(MountTypeEnum::BIND),
                    source: Some(kitchen.workspace_host_path()),
                    target: Some(kitchen.container_workspace_path()),
                    bind_options: Some(MountBindOptions {
                        create_mountpoint: Some(false),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                Mount {
                    typ: Some(MountTypeEnum::BIND),
                    source: Some("/var/run/docker.sock".to_string()),
                    target: Some("/var/run/docker.sock".to_string()),
                    bind_options: Some(MountBindOptions {
                        create_mountpoint: Some(false),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ]),
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
