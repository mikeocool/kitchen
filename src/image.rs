use bollard::Docker;

use bytes;
use flate2;
use futures_util::stream::StreamExt;
use tar;

const DOCKERFILE: &[u8] = include_bytes!("../resources/Dockerfile");
const INIT_SH: &[u8] = include_bytes!("../resources/init.sh");

pub async fn build(image_tag: &str) {
    let tar_bytes = build_context_tar();
    let body = bollard::body_full(bytes::Bytes::from(tar_bytes));

    let opts = bollard::query_parameters::BuildImageOptionsBuilder::default()
        .dockerfile("Dockerfile")
        .t(&image_tag)
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

fn build_context_tar() -> Vec<u8> {
    let mut buf = Vec::new();
    let enc = flate2::write::GzEncoder::new(&mut buf, flate2::Compression::default());
    let mut ar = tar::Builder::new(enc);

    let mut hdr = tar::Header::new_gnu();
    hdr.set_size(DOCKERFILE.len() as u64);
    hdr.set_mode(0o644);
    hdr.set_cksum();
    ar.append_data(&mut hdr, "Dockerfile", DOCKERFILE).unwrap();

    let mut hdr = tar::Header::new_gnu();
    hdr.set_size(INIT_SH.len() as u64);
    hdr.set_mode(0o755);
    hdr.set_cksum();
    ar.append_data(&mut hdr, "init.sh", INIT_SH).unwrap();

    // copy this program into the build context
    // TODO this is nice for dev, but will break if there's a mismatch
    // between arch/os family on the host and the image
    let self_path = std::env::current_exe().expect("failed to get current exe path");
    let self_bytes = std::fs::read(&self_path).expect("failed to read current exe");
    let mut hdr = tar::Header::new_gnu();
    hdr.set_size(self_bytes.len() as u64);
    hdr.set_mode(0o755);
    hdr.set_cksum();
    ar.append_data(&mut hdr, "kitchen", self_bytes.as_slice())
        .unwrap();

    ar.into_inner().unwrap().finish().unwrap();
    buf
}
