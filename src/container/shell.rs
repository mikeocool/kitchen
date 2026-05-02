use bollard::Docker;
use bollard::exec::{CreateExecOptions, ResizeExecOptions, StartExecResults};
use eyre::{Result, eyre};
use futures_util::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::kitchen::KitchenConfig;

pub async fn shell(docker: &Docker, kitchen: &KitchenConfig) -> Result<i32> {
    let container_name = kitchen.container_name();

    // TODO abstract this
    let running = docker
        .inspect_container(&container_name, None)
        .await
        .ok()
        .and_then(|info| info.state)
        .and_then(|s| s.running)
        .unwrap_or(false);

    if !running {
        return Err(eyre!("Container {container_name} is not running"));
    }

    // Create an exec instance with a TTY and all three streams attached.
    // TODO this doesnt seem to run default shell
    let exec = docker
        .create_exec(
            &container_name,
            CreateExecOptions {
                cmd: Some(vec![
                    "sh",
                    "-c",
                    "exec $(getent passwd $(whoami) | cut -d: -f7)",
                ]),
                attach_stdin: Some(true),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                tty: Some(true),
                ..Default::default()
            },
        )
        .await
        .expect("failed to create exec instance");

    // Start the exec in attached (non-detach) mode.
    // start_exec with detach:false returns Attached{output, input}.
    let StartExecResults::Attached {
        mut output,
        mut input,
    } = docker
        .start_exec(&exec.id, None)
        .await
        .expect("failed to start exec")
    else {
        unreachable!("non-detach start_exec returned Detached")
    };

    // Switch the local terminal to raw mode so keystrokes pass through
    // unmodified rather than being line-buffered by the OS.
    crossterm::terminal::enable_raw_mode().expect("failed to enable raw mode");

    // Sync the container PTY size to the current terminal size immediately.
    if let Ok((cols, rows)) = crossterm::terminal::size() {
        let _ = docker
            .resize_exec(
                &exec.id,
                ResizeExecOptions {
                    height: rows,
                    width: cols,
                },
            )
            .await;
    }

    // Clone handles needed by the background tasks.
    let docker_resize = docker.clone();
    let exec_id_resize = exec.id.clone();

    // Task 1: read raw bytes from local stdin and forward them to the exec input.
    let stdin_task = tokio::spawn(async move {
        let mut stdin = tokio::io::stdin();
        let mut buf = [0u8; 256];

        loop {
            match stdin.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if input.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Task 2: read output from the exec stream and write it to local stdout
    // With tty:true the decoder emits LogOutput::Console, which implements
    // AsRef<[u8]>, so we forward raw bytes without interpreting the variant.
    let output_task = tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(Ok(chunk)) = output.next().await {
            if stdout.write_all(chunk.as_ref()).await.is_err() || stdout.flush().await.is_err() {
                break;
            }
        }
    });

    // Task 3: listen for SIGWINCH and forward terminal size changes to the PTY.
    // Using a signal instead of crossterm's EventStream avoids competing with
    // Task 1 for stdin bytes.
    let resize_task = tokio::spawn(async move {
        let mut sigwinch =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::window_change())
                .expect("failed to register SIGWINCH handler");

        while sigwinch.recv().await.is_some() {
            if let Ok((cols, rows)) = crossterm::terminal::size() {
                let _ = docker_resize
                    .resize_exec(
                        &exec_id_resize,
                        ResizeExecOptions {
                            height: rows,
                            width: cols,
                        },
                    )
                    .await;
            }
        }
    });

    // Wait for the shell to exit (the output stream closes).
    output_task.await.ok();
    stdin_task.abort();
    resize_task.abort();

    let _ = crossterm::terminal::disable_raw_mode();
    // Move to a clean line so the host shell prompt appears without needing Enter.
    let _ = std::io::Write::write_all(&mut std::io::stdout(), b"\r\n");
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let exit_code = docker
        .inspect_exec(&exec.id)
        .await
        .ok()
        .and_then(|info| info.exit_code)
        .unwrap_or(0) as i32;

    Ok(exit_code)
}
