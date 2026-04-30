use std::collections::HashMap;

use bollard::Docker;
use bollard::errors::Error as BollardError;
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::models::{
    ContainerCreateBody, EndpointSettings, HostConfig, Mount, MountBindOptions, MountTypeEnum,
    NetworkCreateRequest, NetworkingConfig, VolumeCreateRequest,
};
use bollard::query_parameters::{
    CreateContainerOptionsBuilder, LogsOptionsBuilder, RemoveContainerOptionsBuilder,
};
use futures_util::StreamExt;

use crate::kitchen::KitchenConfig;
mod shell;
pub use shell::shell;

const READY_SENTINEL: &str = "Kitchen is ready to cook";

async fn ensure_volume(docker: &Docker, name: &str) -> Result<(), BollardError> {
    match docker.inspect_volume(name).await {
        Ok(_) => return Ok(()),
        Err(BollardError::DockerResponseServerError {
            status_code: 404, ..
        }) => {}
        Err(e) => return Err(e),
    }
    docker
        .create_volume(VolumeCreateRequest {
            name: Some(name.to_string()),
            ..Default::default()
        })
        .await?;
    Ok(())
}

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

pub async fn run(docker: &Docker, kitchen: &KitchenConfig) -> Result<(), bollard::errors::Error> {
    let container_name = kitchen.container_name();

    let network = kitchen.container.network.as_ref();

    if let Some(network) = network {
        ensure_network(docker, network).await?;
    }

    let tailscale_volume = format!("{container_name}-tailscale");
    ensure_volume(docker, &tailscale_volume).await?;

    let options = CreateContainerOptionsBuilder::default()
        .name(&container_name)
        .build();

    let networking_config = network.map(|n| NetworkingConfig {
        endpoints_config: Some(HashMap::from([(
            n.to_string(),
            EndpointSettings::default(),
        )])),
    });

    let mut mounts = vec![
        Mount {
            typ: Some(MountTypeEnum::BIND),
            source: Some(kitchen.container.host_workspace_path.clone()),
            target: Some(kitchen.container_workspace_path.clone()),
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
        Mount {
            typ: Some(MountTypeEnum::VOLUME),
            source: Some(tailscale_volume),
            target: Some("/var/lib/tailscale".to_string()),
            ..Default::default()
        },
    ];

    mounts.extend(kitchen.container.additional_mounts.clone());

    let body = ContainerCreateBody {
        image: Some(container_name.clone()),
        hostname: Some(container_name.clone()),
        env: Some(vec![kitchen.kitchen_workspace_env()]),
        networking_config,
        host_config: Some(HostConfig {
            mounts: Some(mounts),
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

pub async fn exec(
    docker: &Docker,
    kitchen: &KitchenConfig,
    cmd: Vec<impl Into<String>>,
) -> Result<i64, BollardError> {
    let container_name = kitchen.container_name();
    let cmd: Vec<String> = cmd.into_iter().map(Into::into).collect();

    let exec = docker
        .create_exec(
            &container_name,
            CreateExecOptions {
                cmd: Some(cmd),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                tty: Some(false),
                ..Default::default()
            },
        )
        .await?;

    let start_result = docker.start_exec(&exec.id, None).await?;

    if let StartExecResults::Attached { mut output, .. } = start_result {
        while let Some(result) = output.next().await {
            match result {
                Ok(log) => print!("{log}"),
                Err(e) => return Err(e),
            }
        }
    }

    let inspect = docker.inspect_exec(&exec.id).await?;
    Ok(inspect.exit_code.unwrap_or(0))
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
