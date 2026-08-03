use std::io::{self, Read};

use sha2::{Digest, Sha256};
use world_domain::Sha256Digest;

/// Calculates the canonical SHA-256 digest of bytes.
pub fn sha256_digest(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::new(format!("{:x}", Sha256::digest(bytes)))
}

/// Calculates SHA-256 incrementally without loading an artifact into memory.
///
/// # Errors
///
/// Returns an I/O error when the reader cannot be consumed completely.
pub fn sha256_digest_reader(mut reader: impl Read) -> Result<Sha256Digest, io::Error> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Ok(Sha256Digest::new(format!("{:x}", hasher.finalize())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_lowercase_sha256() {
        let digest = sha256_digest(b"robokit");
        assert!(digest.is_well_formed());
        assert_eq!(
            digest.as_str(),
            "b298b81a7d92103720e71f80de21a2eabc662178ee51a67c1ca39bb870766de4"
        );
    }

    #[test]
    fn streaming_digest_matches_in_memory_digest() {
        assert_eq!(
            sha256_digest_reader(&b"robokit"[..]).expect("slice reader should succeed"),
            sha256_digest(b"robokit")
        );
    }
}
