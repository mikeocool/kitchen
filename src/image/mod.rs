use bollard::Docker;

use bytes;
use eyre::Result;
use flate2;
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use tar;

use crate::kitchen::KitchenConfig;

pub mod containerfile;
pub use containerfile::Containerfile;

const INIT_SH: &[u8] = include_bytes!("../../resources/init.sh");
const KTICHEN_PKG: &[u8] = include_bytes!("../../resources/lib/kitchen-pkg");

pub struct ContextFile {
    pub path: String,
    pub contents: Vec<u8>,
    pub mode: u32,
}

impl ContextFile {
    pub fn new(path: impl Into<String>, contents: impl Into<Vec<u8>>) -> Self {
        Self {
            path: path.into(),
            contents: contents.into(),
            mode: 0o644,
        }
    }

    pub fn with_mode(mut self, mode: u32) -> Self {
        self.mode = mode;
        self
    }
}

pub async fn build(kitchen: &KitchenConfig) -> Result<()> {
    let tar_bytes = build_context_tar(kitchen)?;
    let body = bollard::body_full(bytes::Bytes::from(tar_bytes));

    let mut buildargs = HashMap::new();
    buildargs.insert("KITCHEN_WORKSPACE", kitchen.container_workspace_path_str());

    let opts = bollard::query_parameters::BuildImageOptionsBuilder::default()
        .dockerfile("Dockerfile")
        .t(&kitchen.container_name())
        .rm(true)
        .buildargs(&buildargs)
        .build();

    // TODO share this
    let docker = Docker::connect_with_local_defaults().expect("failed to connect to Docker");
    let mut stream = docker.build_image(opts, None, Some(body));
    while let Some(result) = stream.next().await {
        match result {
            Ok(info) => {
                if let Some(msg) = info.stream {
                    print!("{}", msg);
                }
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

fn build_containerfile(kitchen: &KitchenConfig) -> Result<String> {
    let mut system_packages = vec![
        "sudo",
        "curl",
        "git",
        "ca-certificates", // might needs to run update-ca-certificates (debian) or update-ca-trust (redhat)
        "zsh",             // TODO configure useer's shell in config and install based on that
        "vim",
        // TODO these are debian specific
        "build-essential", // redhat: dnf groupinstall -y "Development Tools" alpine: build-base
        "pkg-config",
        "openssh-client", // openssh-clients on redhat -- needed for git ssh clones
        "locales",
    ];
    system_packages.extend(kitchen.system_packages.iter().map(|s| s.as_str()));
    let pkg_args = system_packages.join(" ");

    let mut containerfile = Containerfile::new()
        .from(&kitchen.container.base_image)
        .arg("KITCHEN_WORKSPACE", "/workspace/default")
        .run("mkdir -p /usr/lib/kitchen/")
        .copy("kitchen-pkg", "/usr/lib/kitchen/kitchen-pkg")
        .run("chmod +x /usr/lib/kitchen/kitchen-pkg")
        .run(&format!("/usr/lib/kitchen/kitchen-pkg {}", pkg_args))
        //fixes terminal wonkiness (but will presumably need to be different for non-us users)
        //maybe do the locale-get in the init script with the user's LANG?
        //will be different on other distros
        .run(r#"echo "en_US.UTF-8 UTF-8" >> /etc/locale.gen && locale-gen"#)
        .run("mkdir -p /etc/kitchen/daemons/")
        .run(
            r#"useradd -m -s /bin/zsh k \
            && usermod -aG sudo k \
            && echo "k ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/k"#,
        );

    // TODO SHELL ["/bin/bash", "-o", "pipefail", "-c"]

    for ext in &kitchen.extensions {
        if let Some(instructions) = ext.image_instructions(kitchen)? {
            containerfile = containerfile.extend(&instructions);
        }
    }

    // from https://mise.jdx.dev/mise-cookbook/docker.html
    Ok(containerfile
        .run("mkdir -p /usr/local/bin/")
        .copy("kitchen", "/usr/local/bin/kitchen")
        .run("mkdir -p ${KITCHEN_WORKSPACE}")
        .env("KITCHEN_WORKSPACE", "${KITCHEN_WORKSPACE}")
        .run("/usr/local/bin/kitchen container-install")
        .copy("init.sh", "/init.sh")
        .run("chmod +x init.sh")
        .user("k")
        .entrypoint(&["/init.sh"])
        .build())
}

fn build_context_tar(kitchen: &KitchenConfig) -> Result<Vec<u8>> {
    let self_path = std::env::current_exe().expect("failed to get current exe path");
    let self_bytes = std::fs::read(&self_path).expect("failed to read current exe");

    let mut files = vec![
        ContextFile::new("Dockerfile", build_containerfile(&kitchen)?),
        ContextFile::new("init.sh", INIT_SH).with_mode(0o755),
        ContextFile::new("kitchen-pkg", KTICHEN_PKG).with_mode(0o755),
        // TODO this is nice for dev, but will break if there's a mismatch
        // between arch/os family on the host and the image
        ContextFile::new("kitchen", self_bytes).with_mode(0o755),
    ];

    for ext in &kitchen.extensions {
        files.extend(ext.image_context(kitchen)?);
    }

    let mut buf = Vec::new();
    let enc = flate2::write::GzEncoder::new(&mut buf, flate2::Compression::default());
    let mut ar = tar::Builder::new(enc);

    for file in &files {
        let mut hdr = tar::Header::new_gnu();
        hdr.set_size(file.contents.len() as u64);
        hdr.set_mode(file.mode);
        hdr.set_cksum();
        ar.append_data(&mut hdr, &file.path, file.contents.as_slice())
            .unwrap();
    }

    ar.into_inner().unwrap().finish().unwrap();
    Ok(buf)
}
