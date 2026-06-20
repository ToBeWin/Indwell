use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use hmac::{Hmac, Mac};
use indwell_channel::ChannelPolicy;
use indwell_core::{AuthContext, RiskLevel, ToolDescriptor};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};
use thiserror::Error;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    Allow,
    RequireOwnerAuth,
    RequireConfirmation,
    Deny { reason: String },
}

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("forbidden tool: {0}")]
    ForbiddenTool(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingChallenge {
    pub session_id: String,
    pub code: String,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairedDevice {
    pub device_id: String,
    pub label: String,
    pub public_key_hash: String,
    pub public_key: Vec<u8>,
    pub paired_at_ms: u64,
    pub last_seen_at_ms: Option<u64>,
    pub revoked_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredSecret {
    pub key_ref: String,
    pub fingerprint: String,
    pub stored_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedSecret {
    pub key_ref: String,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassphraseChallenge {
    pub challenge_id: String,
    pub phrase: String,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthSession {
    pub session_id: String,
    pub device_id: String,
    pub subject_id: String,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmationGrant {
    pub grant_id: String,
    pub subject_id: String,
    pub allowed_tool: String,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub consumed_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedRequest {
    pub device_id: String,
    pub timestamp_ms: u64,
    pub nonce: String,
    pub method: String,
    pub path: String,
    pub body_sha256: String,
}

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("pairing challenge expired")]
    PairingExpired,
    #[error("pairing challenge code mismatch")]
    PairingCodeMismatch,
    #[error("pairing challenge not found")]
    PairingNotFound,
    #[error("passphrase challenge expired")]
    PassphraseExpired,
    #[error("passphrase challenge mismatch")]
    PassphraseMismatch,
    #[error("passphrase challenge not found")]
    PassphraseNotFound,
    #[error("secret ref must not be empty")]
    EmptySecretRef,
    #[error("secret encryption failed")]
    SecretEncryptionFailed,
    #[error("secret decryption failed")]
    SecretDecryptionFailed,
    #[error("secret key must be 32 bytes")]
    InvalidSecretKey,
    #[error("secret store io error: {0}")]
    SecretStoreIo(String),
    #[error("secret store json error: {0}")]
    SecretStoreJson(String),
    #[error("pairing public key must be 32 bytes")]
    InvalidPairingPublicKey,
    #[error("pairing signature must be 64 bytes")]
    InvalidPairingSignature,
    #[error("pairing signature verification failed")]
    PairingSignatureInvalid,
    #[error("paired device not found")]
    PairedDeviceNotFound,
    #[error("paired device has been revoked")]
    PairedDeviceRevoked,
    #[error("auth session not found")]
    AuthSessionNotFound,
    #[error("auth session expired")]
    AuthSessionExpired,
    #[error("auth session token is malformed")]
    AuthSessionMalformed,
    #[error("auth session token signature mismatch")]
    AuthSessionSignatureMismatch,
    #[error("signed request device mismatch")]
    SignedRequestDeviceMismatch,
    #[error("signed request is outside the allowed clock skew")]
    SignedRequestStale,
    #[error("signed request signature verification failed")]
    SignedRequestSignatureInvalid,
    #[error("confirmation grant not found")]
    ConfirmationGrantNotFound,
    #[error("confirmation grant expired")]
    ConfirmationGrantExpired,
    #[error("confirmation grant already consumed")]
    ConfirmationGrantConsumed,
    #[error("confirmation grant subject mismatch")]
    ConfirmationGrantSubjectMismatch,
    #[error("confirmation grant tool mismatch")]
    ConfirmationGrantToolMismatch,
}

#[derive(Debug, Default, Clone)]
pub struct PolicyEngine;

impl PolicyEngine {
    pub fn evaluate_tool(
        &self,
        tool: &ToolDescriptor,
        auth: &AuthContext,
        channel: &ChannelPolicy,
    ) -> PolicyDecision {
        if tool.risk == RiskLevel::Forbidden {
            return PolicyDecision::Deny {
                reason: "forbidden tool".to_string(),
            };
        }

        if !channel_allows_tool(tool, channel) {
            return PolicyDecision::Deny {
                reason: format!(
                    "channel {:?} is not allowed to use {}",
                    channel.channel, tool.name
                ),
            };
        }

        if tool.requires_owner && !auth.owner_authenticated {
            return PolicyDecision::RequireOwnerAuth;
        }

        if tool.requires_confirmation
            || (tool.risk >= RiskLevel::High && channel.requires_confirmation_for_high)
        {
            if !auth.has_strong_factor() {
                return PolicyDecision::RequireConfirmation;
            }
        }

        PolicyDecision::Allow
    }
}

#[derive(Debug, Default, Clone)]
pub struct PairingManager {
    challenges: BTreeMap<String, PairingChallenge>,
    paired_devices: BTreeMap<String, PairedDevice>,
}

#[derive(Debug, Default, Clone)]
pub struct PassphraseChallengeManager {
    challenges: BTreeMap<String, PassphraseChallenge>,
    phrase_cursor: usize,
}

#[derive(Debug, Clone)]
pub struct SessionTokenManager {
    signing_key: [u8; 32],
    sessions: BTreeMap<String, AuthSession>,
}

#[derive(Debug, Default, Clone)]
pub struct ConfirmationGrantManager {
    grants: BTreeMap<String, ConfirmationGrant>,
}

impl ConfirmationGrantManager {
    pub fn from_grants(grants: impl IntoIterator<Item = ConfirmationGrant>) -> Self {
        Self {
            grants: grants
                .into_iter()
                .map(|grant| (grant.grant_id.clone(), grant))
                .collect(),
        }
    }

    pub fn grants(&self) -> Vec<ConfirmationGrant> {
        self.grants.values().cloned().collect()
    }

    pub fn issue(
        &mut self,
        subject_id: impl Into<String>,
        allowed_tool: impl Into<String>,
        now_ms: u64,
        ttl_ms: u64,
    ) -> ConfirmationGrant {
        let grant = ConfirmationGrant {
            grant_id: Uuid::new_v4().to_string(),
            subject_id: subject_id.into(),
            allowed_tool: allowed_tool.into(),
            issued_at_ms: now_ms,
            expires_at_ms: now_ms.saturating_add(ttl_ms),
            consumed_at_ms: None,
        };
        self.grants.insert(grant.grant_id.clone(), grant.clone());
        grant
    }

    pub fn consume(
        &mut self,
        grant_id: &str,
        subject_id: &str,
        tool: &str,
        now_ms: u64,
    ) -> Result<ConfirmationGrant, SecurityError> {
        let grant = self
            .grants
            .get_mut(grant_id)
            .ok_or(SecurityError::ConfirmationGrantNotFound)?;
        if grant.consumed_at_ms.is_some() {
            return Err(SecurityError::ConfirmationGrantConsumed);
        }
        if now_ms > grant.expires_at_ms {
            return Err(SecurityError::ConfirmationGrantExpired);
        }
        if grant.subject_id != subject_id {
            return Err(SecurityError::ConfirmationGrantSubjectMismatch);
        }
        if grant.allowed_tool != tool {
            return Err(SecurityError::ConfirmationGrantToolMismatch);
        }
        grant.consumed_at_ms = Some(now_ms);
        Ok(grant.clone())
    }
}

#[derive(Debug, Clone)]
pub struct JsonConfirmationGrantStore {
    path: PathBuf,
}

impl JsonConfirmationGrantStore {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, SecurityError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(secret_store_io)?;
        }
        Ok(Self { path })
    }

    pub fn load(&self) -> Result<Vec<ConfirmationGrant>, SecurityError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let bytes = fs::read(&self.path).map_err(secret_store_io)?;
        serde_json::from_slice(&bytes).map_err(secret_store_json)
    }

    pub fn save(&self, grants: &[ConfirmationGrant]) -> Result<(), SecurityError> {
        let tmp_path = self.path.with_extension("json.tmp");
        let mut file = fs::File::create(&tmp_path).map_err(secret_store_io)?;
        serde_json::to_writer_pretty(&mut file, grants).map_err(secret_store_json)?;
        writeln!(file).map_err(secret_store_io)?;
        file.flush().map_err(secret_store_io)?;
        fs::rename(tmp_path, &self.path).map_err(secret_store_io)?;
        Ok(())
    }
}

impl SessionTokenManager {
    pub fn new(signing_key: [u8; 32]) -> Self {
        Self {
            signing_key,
            sessions: BTreeMap::new(),
        }
    }

    pub fn issue(
        &mut self,
        device: &PairedDevice,
        subject_id: impl Into<String>,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<(AuthSession, String), SecurityError> {
        if device.revoked_at_ms.is_some() {
            return Err(SecurityError::PairedDeviceRevoked);
        }
        let session = AuthSession {
            session_id: Uuid::new_v4().to_string(),
            device_id: device.device_id.clone(),
            subject_id: subject_id.into(),
            issued_at_ms: now_ms,
            expires_at_ms: now_ms.saturating_add(ttl_ms),
        };
        let token = self.sign_session(&session);
        self.sessions
            .insert(session.session_id.clone(), session.clone());
        Ok((session, token))
    }

    pub fn verify(&self, token: &str, now_ms: u64) -> Result<AuthSession, SecurityError> {
        let (session_id, signature) = token
            .split_once('.')
            .ok_or(SecurityError::AuthSessionMalformed)?;
        let session = self
            .sessions
            .get(session_id)
            .ok_or(SecurityError::AuthSessionNotFound)?;
        if now_ms > session.expires_at_ms {
            return Err(SecurityError::AuthSessionExpired);
        }
        if self.sign_session(session) != format!("{session_id}.{signature}") {
            return Err(SecurityError::AuthSessionSignatureMismatch);
        }
        Ok(session.clone())
    }

    pub fn revoke(&mut self, session_id: &str) -> bool {
        self.sessions.remove(session_id).is_some()
    }

    fn sign_session(&self, session: &AuthSession) -> String {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(&self.signing_key)
            .expect("HMAC-SHA256 accepts fixed-length keys");
        mac.update(session_signature_payload(session).as_bytes());
        let signature = mac.finalize().into_bytes();
        format!("{}.{}", session.session_id, to_hex(&signature))
    }
}

pub fn session_signature_payload(session: &AuthSession) -> String {
    format!(
        "indwell-session-v1\nsession_id={}\ndevice_id={}\nsubject_id={}\nissued_at_ms={}\nexpires_at_ms={}\n",
        session.session_id,
        session.device_id,
        session.subject_id,
        session.issued_at_ms,
        session.expires_at_ms,
    )
}

impl PassphraseChallengeManager {
    pub fn issue(&mut self, now_ms: u64, ttl_ms: u64) -> PassphraseChallenge {
        let phrase = CONFIRMATION_PHRASES[self.phrase_cursor % CONFIRMATION_PHRASES.len()];
        self.phrase_cursor = self.phrase_cursor.saturating_add(1);
        let challenge = PassphraseChallenge {
            challenge_id: Uuid::new_v4().to_string(),
            phrase: phrase.to_string(),
            expires_at_ms: now_ms.saturating_add(ttl_ms),
        };
        self.challenges
            .insert(challenge.challenge_id.clone(), challenge.clone());
        challenge
    }

    pub fn verify(
        &mut self,
        challenge_id: &str,
        spoken_phrase: &str,
        now_ms: u64,
    ) -> Result<(), SecurityError> {
        let challenge = self
            .challenges
            .remove(challenge_id)
            .ok_or(SecurityError::PassphraseNotFound)?;
        if now_ms > challenge.expires_at_ms {
            return Err(SecurityError::PassphraseExpired);
        }
        if normalize_phrase(&challenge.phrase) != normalize_phrase(spoken_phrase) {
            return Err(SecurityError::PassphraseMismatch);
        }
        Ok(())
    }
}

const CONFIRMATION_PHRASES: &[&str] = &["blue moon", "quiet light", "silver river", "morning star"];

impl PairingManager {
    pub fn from_paired_devices(devices: impl IntoIterator<Item = PairedDevice>) -> Self {
        Self {
            challenges: BTreeMap::new(),
            paired_devices: devices
                .into_iter()
                .map(|device| (device.device_id.clone(), device))
                .collect(),
        }
    }

    pub fn issue_challenge(&mut self, now_ms: u64, ttl_ms: u64) -> PairingChallenge {
        let session_id = Uuid::new_v4().to_string();
        let code = session_id
            .chars()
            .filter(|ch| ch.is_ascii_hexdigit())
            .take(6)
            .collect::<String>()
            .to_uppercase();
        let challenge = PairingChallenge {
            session_id: session_id.clone(),
            code,
            expires_at_ms: now_ms.saturating_add(ttl_ms),
        };
        self.challenges.insert(session_id, challenge.clone());
        challenge
    }

    pub fn complete_pairing(
        &mut self,
        session_id: &str,
        code: &str,
        label: impl Into<String>,
        public_key: &[u8],
        now_ms: u64,
    ) -> Result<PairedDevice, SecurityError> {
        if public_key.len() != 32 {
            return Err(SecurityError::InvalidPairingPublicKey);
        }
        let challenge = self
            .challenges
            .remove(session_id)
            .ok_or(SecurityError::PairingNotFound)?;
        if now_ms > challenge.expires_at_ms {
            return Err(SecurityError::PairingExpired);
        }
        if !challenge.code.eq_ignore_ascii_case(code.trim()) {
            return Err(SecurityError::PairingCodeMismatch);
        }

        let paired = PairedDevice {
            device_id: Uuid::new_v4().to_string(),
            label: label.into(),
            public_key_hash: fingerprint(public_key),
            public_key: public_key.to_vec(),
            paired_at_ms: now_ms,
            last_seen_at_ms: Some(now_ms),
            revoked_at_ms: None,
        };
        self.paired_devices
            .insert(paired.device_id.clone(), paired.clone());
        Ok(paired)
    }

    pub fn complete_pairing_signed(
        &mut self,
        session_id: &str,
        code: &str,
        label: impl Into<String>,
        public_key: &[u8],
        signature: &[u8],
        now_ms: u64,
    ) -> Result<PairedDevice, SecurityError> {
        let label = label.into();
        verify_pairing_signature(session_id, code, &label, public_key, signature)?;
        self.complete_pairing(session_id, code, label, public_key, now_ms)
    }

    pub fn paired_devices(&self) -> Vec<PairedDevice> {
        self.paired_devices.values().cloned().collect()
    }

    pub fn mark_seen(
        &mut self,
        device_id: &str,
        now_ms: u64,
    ) -> Result<PairedDevice, SecurityError> {
        let paired = self
            .paired_devices
            .get_mut(device_id)
            .ok_or(SecurityError::PairedDeviceNotFound)?;
        if paired.revoked_at_ms.is_some() {
            return Err(SecurityError::PairedDeviceRevoked);
        }
        paired.last_seen_at_ms = Some(now_ms);
        Ok(paired.clone())
    }

    pub fn revoke(&mut self, device_id: &str, now_ms: u64) -> Result<PairedDevice, SecurityError> {
        let paired = self
            .paired_devices
            .get_mut(device_id)
            .ok_or(SecurityError::PairedDeviceNotFound)?;
        paired.revoked_at_ms = Some(now_ms);
        Ok(paired.clone())
    }
}

#[derive(Debug, Clone)]
pub struct JsonPairedDeviceStore {
    path: PathBuf,
}

impl JsonPairedDeviceStore {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, SecurityError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(secret_store_io)?;
        }
        Ok(Self { path })
    }

    pub fn load(&self) -> Result<Vec<PairedDevice>, SecurityError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let bytes = fs::read(&self.path).map_err(secret_store_io)?;
        serde_json::from_slice(&bytes).map_err(secret_store_json)
    }

    pub fn save(&self, devices: &[PairedDevice]) -> Result<(), SecurityError> {
        let tmp_path = self.path.with_extension("json.tmp");
        let mut file = fs::File::create(&tmp_path).map_err(secret_store_io)?;
        serde_json::to_writer_pretty(&mut file, devices).map_err(secret_store_json)?;
        writeln!(file).map_err(secret_store_io)?;
        file.flush().map_err(secret_store_io)?;
        fs::rename(tmp_path, &self.path).map_err(secret_store_io)?;
        Ok(())
    }
}

pub fn pairing_signature_payload(
    session_id: &str,
    code: &str,
    label: &str,
    public_key: &[u8],
) -> String {
    format!(
        "indwell-pairing-v1\nsession_id={}\ncode={}\nlabel={}\npublic_key_sha256={}\n",
        session_id.trim(),
        code.trim().to_uppercase(),
        label.trim(),
        hex_sha256(public_key),
    )
}

pub fn verify_pairing_signature(
    session_id: &str,
    code: &str,
    label: &str,
    public_key: &[u8],
    signature: &[u8],
) -> Result<(), SecurityError> {
    let public_key: [u8; 32] = public_key
        .try_into()
        .map_err(|_| SecurityError::InvalidPairingPublicKey)?;
    let signature: [u8; 64] = signature
        .try_into()
        .map_err(|_| SecurityError::InvalidPairingSignature)?;
    let verifying_key = VerifyingKey::from_bytes(&public_key)
        .map_err(|_| SecurityError::InvalidPairingPublicKey)?;
    let signature = Signature::from_bytes(&signature);
    verifying_key
        .verify(
            pairing_signature_payload(session_id, code, label, &public_key).as_bytes(),
            &signature,
        )
        .map_err(|_| SecurityError::PairingSignatureInvalid)
}

pub fn signed_request_payload(request: &SignedRequest) -> String {
    format!(
        "indwell-request-v1\ndevice_id={}\ntimestamp_ms={}\nnonce={}\nmethod={}\npath={}\nbody_sha256={}\n",
        request.device_id.trim(),
        request.timestamp_ms,
        request.nonce.trim(),
        request.method.trim().to_ascii_uppercase(),
        request.path.trim(),
        request.body_sha256.trim().to_ascii_lowercase(),
    )
}

pub fn verify_signed_request(
    device: &PairedDevice,
    request: &SignedRequest,
    signature: &[u8],
    now_ms: u64,
    max_clock_skew_ms: u64,
) -> Result<(), SecurityError> {
    if device.revoked_at_ms.is_some() {
        return Err(SecurityError::PairedDeviceRevoked);
    }
    if request.device_id != device.device_id {
        return Err(SecurityError::SignedRequestDeviceMismatch);
    }
    let drift = now_ms.abs_diff(request.timestamp_ms);
    if drift > max_clock_skew_ms {
        return Err(SecurityError::SignedRequestStale);
    }
    let public_key: [u8; 32] = device
        .public_key
        .as_slice()
        .try_into()
        .map_err(|_| SecurityError::InvalidPairingPublicKey)?;
    let signature: [u8; 64] = signature
        .try_into()
        .map_err(|_| SecurityError::InvalidPairingSignature)?;
    let verifying_key = VerifyingKey::from_bytes(&public_key)
        .map_err(|_| SecurityError::InvalidPairingPublicKey)?;
    let signature = Signature::from_bytes(&signature);
    verifying_key
        .verify(signed_request_payload(request).as_bytes(), &signature)
        .map_err(|_| SecurityError::SignedRequestSignatureInvalid)
}

#[derive(Debug, Default, Clone)]
pub struct InMemorySecretStore {
    secrets: BTreeMap<String, Vec<u8>>,
    metadata: BTreeMap<String, StoredSecret>,
}

impl InMemorySecretStore {
    pub fn put(
        &mut self,
        key_ref: impl Into<String>,
        secret: impl AsRef<[u8]>,
        now_ms: u64,
    ) -> Result<StoredSecret, SecurityError> {
        let key_ref = key_ref.into();
        if key_ref.trim().is_empty() {
            return Err(SecurityError::EmptySecretRef);
        }
        let bytes = secret.as_ref().to_vec();
        let stored = StoredSecret {
            key_ref: key_ref.clone(),
            fingerprint: fingerprint(&bytes),
            stored_at_ms: now_ms,
        };
        self.secrets.insert(key_ref.clone(), bytes);
        self.metadata.insert(key_ref, stored.clone());
        Ok(stored)
    }

    pub fn get(&self, key_ref: &str) -> Option<&[u8]> {
        self.secrets.get(key_ref).map(Vec::as_slice)
    }

    pub fn describe(&self, key_ref: &str) -> Option<&StoredSecret> {
        self.metadata.get(key_ref)
    }

    pub fn delete(&mut self, key_ref: &str) -> bool {
        self.metadata.remove(key_ref);
        self.secrets.remove(key_ref).is_some()
    }
}

#[derive(Debug, Clone)]
pub struct FileSealedSecretStore {
    root: PathBuf,
    key_bytes: [u8; 32],
}

impl FileSealedSecretStore {
    pub fn new(root: impl Into<PathBuf>, key_bytes: [u8; 32]) -> Result<Self, SecurityError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(secret_store_io)?;
        Ok(Self { root, key_bytes })
    }

    pub fn put(
        &self,
        key_ref: impl Into<String>,
        secret: impl AsRef<[u8]>,
        now_ms: u64,
    ) -> Result<StoredSecret, SecurityError> {
        let key_ref = key_ref.into();
        if key_ref.trim().is_empty() {
            return Err(SecurityError::EmptySecretRef);
        }
        let nonce = derive_secret_nonce(&key_ref, secret.as_ref(), now_ms);
        let sealed = seal_secret(&key_ref, secret.as_ref(), &self.key_bytes, &nonce)?;
        let path = self.path_for(&key_ref)?;
        let tmp_path = path.with_extension("json.tmp");
        let mut file = fs::File::create(&tmp_path).map_err(secret_store_io)?;
        serde_json::to_writer_pretty(&mut file, &sealed).map_err(secret_store_json)?;
        writeln!(file).map_err(secret_store_io)?;
        file.flush().map_err(secret_store_io)?;
        fs::rename(tmp_path, path).map_err(secret_store_io)?;
        Ok(StoredSecret {
            key_ref,
            fingerprint: sealed.fingerprint,
            stored_at_ms: now_ms,
        })
    }

    pub fn get(&self, key_ref: &str) -> Result<Option<Vec<u8>>, SecurityError> {
        let path = self.path_for(key_ref)?;
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(path).map_err(secret_store_io)?;
        let sealed: SealedSecret = serde_json::from_slice(&bytes).map_err(secret_store_json)?;
        Ok(Some(open_secret(&sealed, &self.key_bytes)?))
    }

    pub fn describe(&self, key_ref: &str) -> Result<Option<StoredSecret>, SecurityError> {
        let path = self.path_for(key_ref)?;
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(path).map_err(secret_store_io)?;
        let sealed: SealedSecret = serde_json::from_slice(&bytes).map_err(secret_store_json)?;
        Ok(Some(StoredSecret {
            key_ref: sealed.key_ref,
            fingerprint: sealed.fingerprint,
            stored_at_ms: 0,
        }))
    }

    pub fn delete(&self, key_ref: &str) -> Result<bool, SecurityError> {
        let path = self.path_for(key_ref)?;
        if !path.exists() {
            return Ok(false);
        }
        fs::remove_file(path).map_err(secret_store_io)?;
        Ok(true)
    }

    fn path_for(&self, key_ref: &str) -> Result<PathBuf, SecurityError> {
        if key_ref.trim().is_empty() {
            return Err(SecurityError::EmptySecretRef);
        }
        let digest = Sha256::digest(key_ref.as_bytes());
        let name = digest
            .iter()
            .take(16)
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        Ok(self.root.join(format!("{name}.json")))
    }
}

pub fn seal_secret(
    key_ref: impl Into<String>,
    secret: &[u8],
    key_bytes: &[u8],
    nonce_bytes: &[u8; 12],
) -> Result<SealedSecret, SecurityError> {
    if key_bytes.len() != 32 {
        return Err(SecurityError::InvalidSecretKey);
    }
    let key_ref = key_ref.into();
    if key_ref.trim().is_empty() {
        return Err(SecurityError::EmptySecretRef);
    }
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key_bytes));
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(nonce_bytes), secret)
        .map_err(|_| SecurityError::SecretEncryptionFailed)?;
    Ok(SealedSecret {
        key_ref,
        nonce: nonce_bytes.to_vec(),
        ciphertext,
        fingerprint: fingerprint(secret),
    })
}

pub fn open_secret(sealed: &SealedSecret, key_bytes: &[u8]) -> Result<Vec<u8>, SecurityError> {
    if key_bytes.len() != 32 || sealed.nonce.len() != 12 {
        return Err(SecurityError::InvalidSecretKey);
    }
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key_bytes));
    cipher
        .decrypt(Nonce::from_slice(&sealed.nonce), sealed.ciphertext.as_ref())
        .map_err(|_| SecurityError::SecretDecryptionFailed)
}

fn derive_secret_nonce(key_ref: &str, secret: &[u8], now_ms: u64) -> [u8; 12] {
    let mut hasher = Sha256::new();
    hasher.update(key_ref.as_bytes());
    hasher.update(now_ms.to_le_bytes());
    hasher.update(secret);
    let digest = hasher.finalize();
    let mut nonce = [0_u8; 12];
    nonce.copy_from_slice(&digest[..12]);
    nonce
}

fn secret_store_io(err: io::Error) -> SecurityError {
    SecurityError::SecretStoreIo(err.to_string())
}

fn secret_store_json(err: serde_json::Error) -> SecurityError {
    SecurityError::SecretStoreJson(err.to_string())
}

fn fingerprint(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    to_hex(&digest)
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn normalize_phrase(input: &str) -> String {
    input
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn channel_allows_tool(tool: &ToolDescriptor, channel: &ChannelPolicy) -> bool {
    match tool.name.as_str() {
        "device.camera.capture" => channel.allow_camera,
        "memory.search" => channel.allow_memory_read,
        "memory.write_candidate" => channel.allow_memory_write,
        "memory.delete" => channel.allow_memory_write && channel.allow_memory_read,
        "system.status" => channel.allow_system_status,
        "system.update.check" | "system.update.apply" => channel.allow_ota,
        name if name.starts_with("device.sensor.") => channel.allow_sensor_read,
        name if name.starts_with("system.config.") => channel.allow_system_config,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Signer, SigningKey};
    use indwell_channel::{ChannelKind, ChannelPolicy};
    use indwell_core::{AuthContext, RiskLevel, ToolDescriptor};

    use super::{
        open_secret, pairing_signature_payload, seal_secret, ConfirmationGrantManager,
        FileSealedSecretStore, InMemorySecretStore, JsonConfirmationGrantStore,
        JsonPairedDeviceStore, PairedDevice, PairingManager, PassphraseChallengeManager,
        PolicyDecision, PolicyEngine, SecurityError, SignedRequest,
    };

    #[test]
    fn public_chat_channel_cannot_capture_camera() {
        let engine = PolicyEngine;
        let channel = ChannelPolicy::default_for(ChannelKind::Telegram);
        let tool = ToolDescriptor::new("device.camera.capture", "capture", RiskLevel::Medium);
        let decision = engine.evaluate_tool(&tool, &AuthContext::anonymous(), &channel);

        assert!(matches!(decision, PolicyDecision::Deny { .. }));
    }

    #[test]
    fn local_owner_can_search_memory() {
        let engine = PolicyEngine;
        let channel = ChannelPolicy::default_for(ChannelKind::LocalPwa);
        let tool = ToolDescriptor::new("memory.search", "search", RiskLevel::Safe);
        let auth = AuthContext::owner("owner", vec![]);

        assert_eq!(
            engine.evaluate_tool(&tool, &auth, &channel),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn pairing_challenge_creates_paired_device() {
        let mut manager = PairingManager::default();
        let challenge = manager.issue_challenge(1000, 30_000);
        let public_key = [7_u8; 32];

        let paired = manager
            .complete_pairing(
                &challenge.session_id,
                &challenge.code,
                "Bingo phone",
                &public_key,
                2000,
            )
            .unwrap();

        assert_eq!(paired.label, "Bingo phone");
        assert_eq!(paired.public_key, public_key);
        assert!(paired.revoked_at_ms.is_none());
        assert_eq!(manager.paired_devices().len(), 1);
    }

    #[test]
    fn pairing_rejects_invalid_public_key() {
        let mut manager = PairingManager::default();
        let challenge = manager.issue_challenge(1000, 30_000);

        let error = manager
            .complete_pairing(
                &challenge.session_id,
                &challenge.code,
                "Bingo phone",
                b"not-an-ed25519-public-key",
                2000,
            )
            .unwrap_err();

        assert!(matches!(error, SecurityError::InvalidPairingPublicKey));
    }

    #[test]
    fn pairing_challenge_accepts_signed_phone_key_proof() {
        let signing_key = SigningKey::from_bytes(&[12_u8; 32]);
        let public_key = signing_key.verifying_key().to_bytes();
        let mut manager = PairingManager::default();
        let challenge = manager.issue_challenge(1000, 30_000);
        let label = "Bingo phone";
        let signature = signing_key
            .sign(
                pairing_signature_payload(
                    &challenge.session_id,
                    &challenge.code,
                    label,
                    &public_key,
                )
                .as_bytes(),
            )
            .to_bytes();

        let paired = manager
            .complete_pairing_signed(
                &challenge.session_id,
                &challenge.code,
                label,
                &public_key,
                &signature,
                2000,
            )
            .unwrap();

        assert_eq!(paired.public_key, public_key);
        assert_eq!(
            manager
                .mark_seen(&paired.device_id, 3000)
                .unwrap()
                .last_seen_at_ms,
            Some(3000)
        );
        assert_eq!(
            manager
                .revoke(&paired.device_id, 4000)
                .unwrap()
                .revoked_at_ms,
            Some(4000)
        );
        assert!(matches!(
            manager.mark_seen(&paired.device_id, 5000),
            Err(SecurityError::PairedDeviceRevoked)
        ));
    }

    #[test]
    fn pairing_challenge_rejects_tampered_signature_payload() {
        let signing_key = SigningKey::from_bytes(&[13_u8; 32]);
        let public_key = signing_key.verifying_key().to_bytes();
        let mut manager = PairingManager::default();
        let challenge = manager.issue_challenge(1000, 30_000);
        let signature = signing_key
            .sign(
                pairing_signature_payload(
                    &challenge.session_id,
                    &challenge.code,
                    "Different phone",
                    &public_key,
                )
                .as_bytes(),
            )
            .to_bytes();

        let error = manager
            .complete_pairing_signed(
                &challenge.session_id,
                &challenge.code,
                "Bingo phone",
                &public_key,
                &signature,
                2000,
            )
            .unwrap_err();

        assert!(matches!(error, SecurityError::PairingSignatureInvalid));
    }

    #[test]
    fn pairing_rejects_wrong_code() {
        let mut manager = PairingManager::default();
        let challenge = manager.issue_challenge(1000, 30_000);
        let public_key = [7_u8; 32];

        let error = manager
            .complete_pairing(
                &challenge.session_id,
                "WRONG",
                "Bingo phone",
                &public_key,
                2000,
            )
            .unwrap_err();

        assert!(matches!(error, SecurityError::PairingCodeMismatch));
    }

    #[test]
    fn secret_store_returns_metadata_without_exposing_value() {
        let mut store = InMemorySecretStore::default();
        let stored = store.put("key_llm_main", b"secret-value", 42).unwrap();

        assert_eq!(stored.key_ref, "key_llm_main");
        assert_eq!(
            store.describe("key_llm_main").unwrap().fingerprint.len(),
            16
        );
        assert_eq!(store.get("key_llm_main").unwrap(), b"secret-value");
        assert!(store.delete("key_llm_main"));
    }

    #[test]
    fn passphrase_challenge_accepts_normalized_phrase_once() {
        let mut manager = PassphraseChallengeManager::default();
        let challenge = manager.issue(1000, 30_000);

        manager
            .verify(&challenge.challenge_id, "Blue   Moon!", 2000)
            .unwrap();

        let error = manager
            .verify(&challenge.challenge_id, "Blue Moon", 3000)
            .unwrap_err();
        assert!(matches!(error, SecurityError::PassphraseNotFound));
    }

    #[test]
    fn passphrase_challenge_rejects_mismatch() {
        let mut manager = PassphraseChallengeManager::default();
        let challenge = manager.issue(1000, 30_000);

        let error = manager
            .verify(&challenge.challenge_id, "wrong phrase", 2000)
            .unwrap_err();

        assert!(matches!(error, SecurityError::PassphraseMismatch));
    }

    #[test]
    fn seals_and_opens_secret_with_chacha20poly1305() {
        let key = [9_u8; 32];
        let nonce = [4_u8; 12];
        let sealed = seal_secret("key_llm_main", b"secret-value", &key, &nonce).unwrap();

        assert_ne!(sealed.ciphertext, b"secret-value");
        assert_eq!(open_secret(&sealed, &key).unwrap(), b"secret-value");
        assert!(open_secret(&sealed, &[1_u8; 32]).is_err());
    }

    #[test]
    fn file_secret_store_persists_only_sealed_bytes() {
        let root =
            std::env::temp_dir().join(format!("indwell-secret-store-{}", uuid::Uuid::new_v4()));
        let store = FileSealedSecretStore::new(&root, [3_u8; 32]).unwrap();
        let stored = store.put("key_llm_main", b"secret-value", 1000).unwrap();

        assert_eq!(stored.key_ref, "key_llm_main");
        assert_eq!(store.get("key_llm_main").unwrap().unwrap(), b"secret-value");
        let raw_files = std::fs::read_dir(&root)
            .unwrap()
            .map(|entry| std::fs::read(entry.unwrap().path()).unwrap())
            .collect::<Vec<_>>();
        assert!(raw_files
            .iter()
            .all(|bytes| !String::from_utf8_lossy(bytes).contains("secret-value")));
        assert!(store.describe("key_llm_main").unwrap().is_some());
        assert!(store.delete("key_llm_main").unwrap());
        assert!(store.get("key_llm_main").unwrap().is_none());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn json_paired_device_store_round_trips_authorized_devices() {
        let root =
            std::env::temp_dir().join(format!("indwell-paired-devices-{}", uuid::Uuid::new_v4()));
        let store = JsonPairedDeviceStore::new(root.join("pairing/devices.json")).unwrap();
        let mut manager = PairingManager::default();
        let challenge = manager.issue_challenge(1000, 30_000);
        let public_key = [8_u8; 32];
        let paired = manager
            .complete_pairing(
                &challenge.session_id,
                &challenge.code,
                "Bingo phone",
                &public_key,
                2000,
            )
            .unwrap();

        store.save(&manager.paired_devices()).unwrap();
        let loaded = store.load().unwrap();
        let restored = PairingManager::from_paired_devices(loaded);

        assert_eq!(restored.paired_devices().len(), 1);
        assert_eq!(restored.paired_devices()[0].device_id, paired.device_id);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn session_token_manager_issues_verifies_and_revokes_tokens() {
        let device = PairedDevice {
            device_id: "phone-1".to_string(),
            label: "Bingo phone".to_string(),
            public_key_hash: "hash".to_string(),
            public_key: vec![1, 2, 3],
            paired_at_ms: 1000,
            last_seen_at_ms: Some(1000),
            revoked_at_ms: None,
        };
        let mut manager = super::SessionTokenManager::new([5_u8; 32]);
        let (session, token) = manager.issue(&device, "owner", 2000, 60_000).unwrap();

        let verified = manager.verify(&token, 3000).unwrap();

        assert_eq!(verified.session_id, session.session_id);
        assert_eq!(verified.device_id, "phone-1");
        assert!(manager.revoke(&session.session_id));
        assert!(matches!(
            manager.verify(&token, 3000),
            Err(SecurityError::AuthSessionNotFound)
        ));
    }

    #[test]
    fn session_token_manager_rejects_tampered_or_expired_tokens() {
        let device = PairedDevice {
            device_id: "phone-1".to_string(),
            label: "Bingo phone".to_string(),
            public_key_hash: "hash".to_string(),
            public_key: vec![1, 2, 3],
            paired_at_ms: 1000,
            last_seen_at_ms: Some(1000),
            revoked_at_ms: None,
        };
        let mut manager = super::SessionTokenManager::new([5_u8; 32]);
        let (_session, token) = manager.issue(&device, "owner", 2000, 10).unwrap();

        assert!(matches!(
            manager.verify(&format!("{token}00"), 2005),
            Err(SecurityError::AuthSessionSignatureMismatch)
        ));
        assert!(matches!(
            manager.verify(&token, 3000),
            Err(SecurityError::AuthSessionExpired)
        ));
    }

    #[test]
    fn signed_request_verification_accepts_paired_device_signature() {
        let signing_key = SigningKey::from_bytes(&[21_u8; 32]);
        let device = PairedDevice {
            device_id: "phone-1".to_string(),
            label: "Bingo phone".to_string(),
            public_key_hash: "hash".to_string(),
            public_key: signing_key.verifying_key().to_bytes().to_vec(),
            paired_at_ms: 1000,
            last_seen_at_ms: Some(1000),
            revoked_at_ms: None,
        };
        let request = SignedRequest {
            device_id: "phone-1".to_string(),
            timestamp_ms: 2000,
            nonce: "nonce-1".to_string(),
            method: "post".to_string(),
            path: "/v1/auth/session".to_string(),
            body_sha256: "ABCD".to_string(),
        };
        let signature = signing_key
            .sign(super::signed_request_payload(&request).as_bytes())
            .to_bytes();

        super::verify_signed_request(&device, &request, &signature, 2100, 5_000).unwrap();
    }

    #[test]
    fn signed_request_verification_rejects_tamper_and_stale_requests() {
        let signing_key = SigningKey::from_bytes(&[22_u8; 32]);
        let device = PairedDevice {
            device_id: "phone-1".to_string(),
            label: "Bingo phone".to_string(),
            public_key_hash: "hash".to_string(),
            public_key: signing_key.verifying_key().to_bytes().to_vec(),
            paired_at_ms: 1000,
            last_seen_at_ms: Some(1000),
            revoked_at_ms: None,
        };
        let request = SignedRequest {
            device_id: "phone-1".to_string(),
            timestamp_ms: 2000,
            nonce: "nonce-1".to_string(),
            method: "POST".to_string(),
            path: "/v1/auth/session".to_string(),
            body_sha256: "abcd".to_string(),
        };
        let mut tampered = request.clone();
        tampered.path = "/v1/secrets/key".to_string();
        let signature = signing_key
            .sign(super::signed_request_payload(&request).as_bytes())
            .to_bytes();

        assert!(matches!(
            super::verify_signed_request(&device, &tampered, &signature, 2100, 5_000),
            Err(SecurityError::SignedRequestSignatureInvalid)
        ));
        assert!(matches!(
            super::verify_signed_request(&device, &request, &signature, 20_000, 5_000),
            Err(SecurityError::SignedRequestStale)
        ));
    }

    #[test]
    fn confirmation_grant_is_single_use_and_tool_scoped() {
        let mut manager = ConfirmationGrantManager::default();
        let grant = manager.issue("owner", "system.update.apply", 1000, 30_000);

        assert!(matches!(
            manager.consume(&grant.grant_id, "owner", "memory.delete", 2000),
            Err(SecurityError::ConfirmationGrantToolMismatch)
        ));
        let consumed = manager
            .consume(&grant.grant_id, "owner", "system.update.apply", 3000)
            .unwrap();
        assert_eq!(consumed.consumed_at_ms, Some(3000));
        assert!(matches!(
            manager.consume(&grant.grant_id, "owner", "system.update.apply", 4000),
            Err(SecurityError::ConfirmationGrantConsumed)
        ));
    }

    #[test]
    fn confirmation_grant_rejects_expired_or_wrong_subject() {
        let mut manager = ConfirmationGrantManager::default();
        let grant = manager.issue("owner", "system.update.apply", 1000, 10);

        assert!(matches!(
            manager.consume(&grant.grant_id, "other", "system.update.apply", 1005),
            Err(SecurityError::ConfirmationGrantSubjectMismatch)
        ));
        assert!(matches!(
            manager.consume(&grant.grant_id, "owner", "system.update.apply", 2000),
            Err(SecurityError::ConfirmationGrantExpired)
        ));
    }

    #[test]
    fn json_confirmation_grant_store_round_trips_consumed_grants() {
        let root = std::env::temp_dir().join(format!(
            "indwell-confirmation-grants-{}",
            uuid::Uuid::new_v4()
        ));
        let store = JsonConfirmationGrantStore::new(root.join("auth/grants.json")).unwrap();
        let mut manager = ConfirmationGrantManager::default();
        let grant = manager.issue("owner", "system.update.apply", 1000, 30_000);
        manager
            .consume(&grant.grant_id, "owner", "system.update.apply", 2000)
            .unwrap();

        store.save(&manager.grants()).unwrap();
        let loaded = store.load().unwrap();
        let restored = ConfirmationGrantManager::from_grants(loaded);
        let restored_grant = restored
            .grants()
            .into_iter()
            .find(|candidate| candidate.grant_id == grant.grant_id)
            .expect("grant should be restored");

        assert_eq!(restored_grant.consumed_at_ms, Some(2000));

        let _ = std::fs::remove_dir_all(root);
    }
}
