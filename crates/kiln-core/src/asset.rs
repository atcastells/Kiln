//! The contract of a built asset (`asset.json`): what it is, what it was
//! made from, what came out and with which tools.

use sha2::{Digest, Sha256};

/// Lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn sha256_file(path: &std::path::Path) -> Result<String, String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("no se puede leer {}: {e}", path.display()))?;
    Ok(sha256_hex(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_of_abc() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_file_reads_and_reports_missing() {
        let p = std::env::current_exe().unwrap();
        assert_eq!(sha256_file(&p).unwrap().len(), 64);
        let err = sha256_file(std::path::Path::new("/no/such/file")).unwrap_err();
        assert!(err.starts_with("no se puede leer /no/such/file"));
    }
}
