//! Content-addressable cache — real implementation lands in Task 4.
//!
//! Task 2 only needs `sha256_hex` for `Source::cache_key`.

use sha2::{Digest, Sha256};

pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}
