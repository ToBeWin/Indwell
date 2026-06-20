use std::{fs, io, path::PathBuf};

use indwell_ota::{OtaManifest, OtaTrustStore, OtaVerificationReport};
use thiserror::Error;

#[derive(Debug)]
pub struct JsonOtaManifestStore {
    path: PathBuf,
    trust_path: PathBuf,
}

impl JsonOtaManifestStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, OtaManifestStoreError> {
        let path = root.into().join("manifest.json");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let store = Self {
            trust_path: path.with_file_name("trust_keys.json"),
            path,
        };
        if !store.path.exists() {
            store.save(&OtaManifest::host_sim_default())?;
        }
        Ok(store)
    }

    pub fn load(&self) -> Result<OtaManifest, OtaManifestStoreError> {
        let bytes = fs::read(&self.path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn save(&self, manifest: &OtaManifest) -> Result<(), OtaManifestStoreError> {
        let bytes = serde_json::to_vec_pretty(manifest)?;
        fs::write(&self.path, bytes)?;
        Ok(())
    }

    pub fn verify(
        &self,
        expected_target: &str,
    ) -> Result<OtaVerificationReport, OtaManifestStoreError> {
        let manifest = self.load()?;
        let trust = self.load_trust_store()?;
        Ok(trust.verify_manifest(&manifest, expected_target))
    }

    pub fn load_trust_store(&self) -> Result<OtaTrustStore, OtaManifestStoreError> {
        if !self.trust_path.exists() {
            return Ok(OtaTrustStore::empty());
        }
        let bytes = fs::read(&self.trust_path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn save_trust_store(&self, trust: &OtaTrustStore) -> Result<(), OtaManifestStoreError> {
        let bytes = serde_json::to_vec_pretty(trust)?;
        fs::write(&self.trust_path, bytes)?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum OtaManifestStoreError {
    #[error("ota manifest io error: {0}")]
    Io(#[from] io::Error),
    #[error("ota manifest json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ed25519_dalek::{Signer, SigningKey};
    use indwell_ota::{manifest_signature_payload, OtaTrustStore};

    use super::JsonOtaManifestStore;

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("indwell-ota-store-{name}-{}", std::process::id()))
    }

    #[test]
    fn creates_default_manifest_and_reports_missing_trusted_signature() {
        let root = temp_root("default");
        let _ = std::fs::remove_dir_all(&root);
        let store = JsonOtaManifestStore::new(&root).unwrap();

        let manifest = store.load().unwrap();
        assert_eq!(manifest.target, "host-sim");

        let report = store.verify("host-sim").unwrap();
        assert!(!report.valid);
        assert!(report
            .checks
            .iter()
            .any(|check| check.name == "trusted_signature" && !check.passed));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn verifies_against_trust_store_when_keys_are_configured() {
        let root = temp_root("trusted");
        let _ = std::fs::remove_dir_all(&root);
        let store = JsonOtaManifestStore::new(&root).unwrap();
        let signing_key = SigningKey::from_bytes(&[11_u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let mut manifest = store.load().unwrap();
        manifest.signature = hex(&signing_key
            .sign(manifest_signature_payload(&manifest).as_bytes())
            .to_bytes());
        store.save(&manifest).unwrap();
        store
            .save_trust_store(&OtaTrustStore::with_keys([hex(verifying_key.as_bytes())]))
            .unwrap();

        let report = store.verify("host-sim").unwrap();

        assert!(report.valid);
        assert!(report
            .checks
            .iter()
            .any(|check| check.name == "trusted_signature"));

        let _ = std::fs::remove_dir_all(root);
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
