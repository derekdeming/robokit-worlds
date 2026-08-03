use sha2::{Digest, Sha256};
use world_domain::Sha256Digest;

/// Calculates the canonical SHA-256 digest of bytes.
pub fn sha256_digest(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::new(format!("{:x}", Sha256::digest(bytes)))
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
}
