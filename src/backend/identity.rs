use libp2p::identity::Keypair;
use std::fs;
use std::path::Path;
use std::io::{self, Write, Read};

pub fn load_identity(path: &Path) -> Result<Keypair, Box<dyn std::error::Error>> {
    if path.exists() {
        let mut file = fs::File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        
        // Try decoding as protobuf (standard libp2p format)
        match Keypair::from_protobuf_encoding(&bytes) {
            Ok(kp) => Ok(kp),
            Err(_) => {
                // Fallback or error handling if format changes
                Err("Failed to decode keypair".into())
            }
        }
    } else {
        Err("Identity file not found".into())
    }
}

pub fn save_identity(path: &Path, keypair: &Keypair) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = keypair.to_protobuf_encoding()?;
    let mut file = fs::File::create(path)?;
    file.write_all(&bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_save_and_load_identity() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("identity.test");
        
        let original_keypair = Keypair::generate_ed25519();
        save_identity(&file_path, &original_keypair).expect("Failed to save identity");
        
        let loaded_keypair = load_identity(&file_path).expect("Failed to load identity");
        
        assert_eq!(
            original_keypair.public(),
            loaded_keypair.public(),
            "Loaded public key should match original"
        );
    }
}
