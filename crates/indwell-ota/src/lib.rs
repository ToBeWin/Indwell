use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtaManifest {
    pub version: String,
    pub channel: String,
    pub target: String,
    pub firmware_url: String,
    pub sha256: String,
    pub signature: String,
    pub min_bootloader: Option<String>,
    pub memory_schema: Option<String>,
    pub notes: Vec<String>,
}

impl OtaManifest {
    pub fn host_sim_default() -> Self {
        let bytes = b"indwell-host-sim-firmware-placeholder";
        Self {
            version: "0.1.0-host-sim".to_string(),
            channel: "local".to_string(),
            target: "host-sim".to_string(),
            firmware_url: "https://github.com/indwell-os/indwell-os/releases".to_string(),
            sha256: hex_sha256(bytes),
            signature: "phase0-signature-required-before-real-ota".to_string(),
            min_bootloader: None,
            memory_schema: Some("memory-palace-jsonl-v1".to_string()),
            notes: vec![
                "Host simulator placeholder manifest.".to_string(),
                "Real firmware must use an Ed25519 signature before apply.".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtaVerificationReport {
    pub valid: bool,
    pub target: String,
    pub version: String,
    pub checks: Vec<OtaCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtaTrustStore {
    pub trusted_manifest_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtaApplyPlan {
    pub version: String,
    pub target: String,
    pub from_slot: OtaSlot,
    pub to_slot: OtaSlot,
    pub firmware_url: String,
    pub sha256: String,
    pub requires_confirmation: bool,
    pub rollback_supported: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OtaSlot {
    Factory,
    Ota0,
    Ota1,
}

impl OtaVerificationReport {
    pub fn failures(&self) -> Vec<&OtaCheck> {
        self.checks.iter().filter(|check| !check.passed).collect()
    }
}

impl OtaTrustStore {
    pub fn empty() -> Self {
        Self {
            trusted_manifest_keys: Vec::new(),
        }
    }

    pub fn with_keys(keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            trusted_manifest_keys: keys.into_iter().map(Into::into).collect(),
        }
    }

    pub fn verify_manifest_signature(&self, manifest: &OtaManifest) -> Result<(), OtaError> {
        if self.trusted_manifest_keys.is_empty() {
            return Err(OtaError::NoTrustedKeys);
        }

        let mut saw_invalid_key = false;
        for key in &self.trusted_manifest_keys {
            match verify_manifest_signature(manifest, key) {
                Ok(()) => return Ok(()),
                Err(OtaError::InvalidPublicKeyHex) => saw_invalid_key = true,
                Err(OtaError::SignatureVerificationFailed)
                | Err(OtaError::InvalidSignatureHex)
                | Err(OtaError::TargetMismatch { .. })
                | Err(OtaError::HashMismatch)
                | Err(OtaError::VerificationFailed(_))
                | Err(OtaError::AlreadyInstalled(_))
                | Err(OtaError::NoTrustedKeys) => {}
            }
        }

        if saw_invalid_key {
            Err(OtaError::InvalidPublicKeyHex)
        } else {
            Err(OtaError::SignatureVerificationFailed)
        }
    }

    pub fn verify_manifest(
        &self,
        manifest: &OtaManifest,
        expected_target: &str,
    ) -> OtaVerificationReport {
        let mut report = verify_manifest_shape(manifest, expected_target);
        let signature_check = match self.verify_manifest_signature(manifest) {
            Ok(()) => check(
                "trusted_signature",
                true,
                "manifest signature matched a trusted Ed25519 key".to_string(),
            ),
            Err(err) => check("trusted_signature", false, err.to_string()),
        };
        report.checks.push(signature_check);
        report.valid = report.checks.iter().all(|check| check.passed);
        report
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtaCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Error)]
pub enum OtaError {
    #[error("manifest target mismatch: expected {expected}, got {actual}")]
    TargetMismatch { expected: String, actual: String },
    #[error("firmware hash mismatch")]
    HashMismatch,
    #[error("manifest failed verification checks: {0}")]
    VerificationFailed(String),
    #[error("manifest version is already installed: {0}")]
    AlreadyInstalled(String),
    #[error("signature hex is invalid")]
    InvalidSignatureHex,
    #[error("public key hex is invalid")]
    InvalidPublicKeyHex,
    #[error("signature verification failed")]
    SignatureVerificationFailed,
    #[error("no trusted OTA public keys configured")]
    NoTrustedKeys,
}

pub fn verify_manifest_shape(
    manifest: &OtaManifest,
    expected_target: &str,
) -> OtaVerificationReport {
    let mut checks = Vec::new();
    checks.push(check(
        "target",
        manifest.target == expected_target,
        format!("expected {expected_target}, got {}", manifest.target),
    ));
    checks.push(check(
        "version",
        !manifest.version.trim().is_empty(),
        "version must be present".to_string(),
    ));
    checks.push(check(
        "firmware_url",
        manifest.firmware_url.starts_with("https://"),
        "firmware URL must use https".to_string(),
    ));
    checks.push(check(
        "sha256",
        is_hex_sha256(&manifest.sha256),
        "sha256 must be 64 lowercase or uppercase hex characters".to_string(),
    ));
    checks.push(check(
        "signature",
        !manifest.signature.trim().is_empty(),
        "signature must be present and must verify against a trusted Ed25519 key".to_string(),
    ));

    OtaVerificationReport {
        valid: checks.iter().all(|check| check.passed),
        target: manifest.target.clone(),
        version: manifest.version.clone(),
        checks,
    }
}

pub fn verify_firmware_hash(manifest: &OtaManifest, bytes: &[u8]) -> Result<(), OtaError> {
    if hex_sha256(bytes).eq_ignore_ascii_case(&manifest.sha256) {
        Ok(())
    } else {
        Err(OtaError::HashMismatch)
    }
}

pub fn verify_manifest_signature(
    manifest: &OtaManifest,
    public_key_hex: &str,
) -> Result<(), OtaError> {
    let public_key = hex_to_fixed::<32>(public_key_hex).ok_or(OtaError::InvalidPublicKeyHex)?;
    let signature = hex_to_fixed::<64>(&manifest.signature).ok_or(OtaError::InvalidSignatureHex)?;
    let verifying_key =
        VerifyingKey::from_bytes(&public_key).map_err(|_| OtaError::InvalidPublicKeyHex)?;
    let signature = Signature::from_bytes(&signature);
    verifying_key
        .verify(manifest_signature_payload(manifest).as_bytes(), &signature)
        .map_err(|_| OtaError::SignatureVerificationFailed)
}

pub fn plan_ota_apply(
    manifest: &OtaManifest,
    expected_target: &str,
    current_version: &str,
    current_slot: OtaSlot,
) -> Result<OtaApplyPlan, OtaError> {
    let report = verify_manifest_shape(manifest, expected_target);
    if !report.valid {
        let failed = report
            .failures()
            .into_iter()
            .map(|check| check.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(OtaError::VerificationFailed(failed));
    }
    if manifest.version == current_version {
        return Err(OtaError::AlreadyInstalled(manifest.version.clone()));
    }

    Ok(OtaApplyPlan {
        version: manifest.version.clone(),
        target: manifest.target.clone(),
        from_slot: current_slot,
        to_slot: next_slot(current_slot),
        firmware_url: manifest.firmware_url.clone(),
        sha256: manifest.sha256.clone(),
        requires_confirmation: true,
        rollback_supported: true,
    })
}

fn next_slot(slot: OtaSlot) -> OtaSlot {
    match slot {
        OtaSlot::Factory | OtaSlot::Ota1 => OtaSlot::Ota0,
        OtaSlot::Ota0 => OtaSlot::Ota1,
    }
}

pub fn manifest_signature_payload(manifest: &OtaManifest) -> String {
    format!(
        "version={}\nchannel={}\ntarget={}\nfirmware_url={}\nsha256={}\nmin_bootloader={}\nmemory_schema={}\nnotes={}\n",
        manifest.version,
        manifest.channel,
        manifest.target,
        manifest.firmware_url,
        manifest.sha256,
        manifest.min_bootloader.as_deref().unwrap_or(""),
        manifest.memory_schema.as_deref().unwrap_or(""),
        manifest.notes.join("|"),
    )
}

pub fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn is_hex_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn check(name: impl Into<String>, passed: bool, detail: String) -> OtaCheck {
    OtaCheck {
        name: name.into(),
        passed,
        detail,
    }
}

fn hex_to_fixed<const N: usize>(value: &str) -> Option<[u8; N]> {
    if value.len() != N * 2 {
        return None;
    }
    let mut out = [0_u8; N];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let hi = hex_value(chunk[0])?;
        let lo = hex_value(chunk[1])?;
        out[index] = (hi << 4) | lo;
    }
    Some(out)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Signer, SigningKey};

    use super::{
        bytes_to_hex, hex_sha256, manifest_signature_payload, plan_ota_apply, verify_firmware_hash,
        verify_manifest_shape, verify_manifest_signature, OtaError, OtaManifest, OtaSlot,
        OtaTrustStore,
    };

    #[test]
    fn default_manifest_passes_shape_checks() {
        let manifest = OtaManifest::host_sim_default();
        let report = verify_manifest_shape(&manifest, "host-sim");

        assert!(report.valid);
        assert!(report.failures().is_empty());
    }

    #[test]
    fn detects_wrong_target_and_missing_signature() {
        let mut manifest = OtaManifest::host_sim_default();
        manifest.target = "esp32s3".to_string();
        manifest.signature.clear();

        let report = verify_manifest_shape(&manifest, "host-sim");

        assert!(!report.valid);
        assert_eq!(report.failures().len(), 2);
    }

    #[test]
    fn verifies_firmware_hash_bytes() {
        let bytes = b"firmware";
        let mut manifest = OtaManifest::host_sim_default();
        manifest.sha256 = hex_sha256(bytes);

        assert!(verify_firmware_hash(&manifest, bytes).is_ok());
        assert!(verify_firmware_hash(&manifest, b"other firmware").is_err());
    }

    #[test]
    fn plans_apply_to_alternate_slot() {
        let mut manifest = OtaManifest::host_sim_default();
        manifest.version = "0.1.1".to_string();

        let plan = plan_ota_apply(&manifest, "host-sim", "0.1.0-host-sim", OtaSlot::Ota0)
            .expect("apply plan");

        assert_eq!(plan.to_slot, OtaSlot::Ota1);
        assert!(plan.requires_confirmation);
        assert!(plan.rollback_supported);
    }

    #[test]
    fn refuses_already_installed_version() {
        let manifest = OtaManifest::host_sim_default();
        let error =
            plan_ota_apply(&manifest, "host-sim", "0.1.0-host-sim", OtaSlot::Ota0).unwrap_err();

        assert!(matches!(error, OtaError::AlreadyInstalled(_)));
    }

    #[test]
    fn verifies_ed25519_manifest_signature() {
        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let mut manifest = OtaManifest::host_sim_default();
        manifest.signature = bytes_to_hex(
            &signing_key
                .sign(manifest_signature_payload(&manifest).as_bytes())
                .to_bytes(),
        );

        assert!(
            verify_manifest_signature(&manifest, &bytes_to_hex(verifying_key.as_bytes())).is_ok()
        );

        manifest.version = "tampered".to_string();
        assert!(matches!(
            verify_manifest_signature(&manifest, &bytes_to_hex(verifying_key.as_bytes())),
            Err(OtaError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn trust_store_adds_signature_check_to_manifest_verification() {
        let signing_key = SigningKey::from_bytes(&[8_u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let mut manifest = OtaManifest::host_sim_default();
        manifest.signature = bytes_to_hex(
            &signing_key
                .sign(manifest_signature_payload(&manifest).as_bytes())
                .to_bytes(),
        );
        let trust = OtaTrustStore::with_keys([bytes_to_hex(verifying_key.as_bytes())]);

        let report = trust.verify_manifest(&manifest, "host-sim");

        assert!(report.valid);
        assert!(report
            .checks
            .iter()
            .any(|check| check.name == "trusted_signature" && check.passed));

        let untrusted = OtaTrustStore::empty().verify_manifest(&manifest, "host-sim");
        assert!(!untrusted.valid);
    }
}
