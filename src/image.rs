use bollard::Docker;

use bytes;
use flate2;
use futures_util::stream::StreamExt;
use tar;

use crate::extensions::tailscale;
use crate::kitchen::KitchenConfig;

const DOCKERFILE: &[u8] = include_bytes!("../resources/Dockerfile");
const INIT_SH: &[u8] = include_bytes!("../resources/init.sh");

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
// TODO return error
pub async fn build(kitchen: &KitchenConfig) {
    let tar_bytes = build_context_tar(kitchen);
    let body = bollard::body_full(bytes::Bytes::from(tar_bytes));

    let opts = bollard::query_parameters::BuildImageOptionsBuilder::default()
        .dockerfile("Dockerfile")
        .t(&kitchen.container_name())
        .rm(true)
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
            Err(e) => eprintln!("Build error: {e}"),
        }
    }
}

fn build_context_tar(kitchen: &KitchenConfig) -> Vec<u8> {
    let self_path = std::env::current_exe().expect("failed to get current exe path");
    let self_bytes = std::fs::read(&self_path).expect("failed to read current exe");

    let mut files = vec![
        ContextFile::new("Dockerfile", DOCKERFILE),
        ContextFile::new("init.sh", INIT_SH).with_mode(0o755),
        // TODO this is nice for dev, but will break if there's a mismatch
        // between arch/os family on the host and the image
        ContextFile::new("kitchen", self_bytes).with_mode(0o755),
    ];

    files.extend(tailscale::image_context(kitchen));

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
    buf
}
