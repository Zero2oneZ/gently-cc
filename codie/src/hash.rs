//! BLAKE3 content address in GRIM format: "sha256:{hex}" prefix for compat.

pub fn content_hash(data: &str) -> String {
    let h = blake3::hash(data.as_bytes());
    format!("sha256:{}", h.to_hex())
}
