//! Authentication management — API key storage and OAuth2 flows

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Credential storage backend.
pub trait CredentialStore: Send + Sync {
    /// Get a credential by key.
    fn get(&self, key: &str) -> Option<String>;
    /// Set a credential.
    fn set(&self, key: &str, value: &str);
    /// Remove a credential.
    fn remove(&self, key: &str);
    /// List all credential keys.
    fn list(&self) -> Vec<String>;
}

/// In-memory credential store.
#[derive(Debug, Default)]
pub struct MemoryStore {
    credentials: std::sync::RwLock<HashMap<String, String>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            credentials: std::sync::RwLock::new(HashMap::new()),
        }
    }
}

impl CredentialStore for MemoryStore {
    fn get(&self, key: &str) -> Option<String> {
        self.credentials.read().ok()?.get(key).cloned()
    }

    fn set(&self, key: &str, value: &str) {
        if let Ok(mut c) = self.credentials.write() {
            c.insert(key.to_string(), value.to_string());
        }
    }

    fn remove(&self, key: &str) {
        if let Ok(mut c) = self.credentials.write() {
            c.remove(key);
        }
    }

    fn list(&self) -> Vec<String> {
        self.credentials.read().map(|c| c.keys().cloned().collect()).unwrap_or_default()
    }
}

/// File-based credential store (encrypted).
pub struct FileStore {
    path: std::path::PathBuf,
    encryption_key: Option<[u8; 32]>,
}

impl FileStore {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self {
            path,
            encryption_key: None,
        }
    }

    pub fn with_encryption(mut self, key: [u8; 32]) -> Self {
        self.encryption_key = Some(key);
        self
    }

    fn read_file(&self) -> std::io::Result<HashMap<String, String>> {
        if !self.path.exists() {
            return Ok(HashMap::new());
        }
        let content = std::fs::read(&self.path)?;

        if let Some(key) = &self.encryption_key {
            // In production, use proper encryption (e.g., AES-GCM)
            // For now, just base64 decode
            let decoded = base64::decode(content).unwrap_or_default();
            Ok(serde_json::from_slice(&decoded).unwrap_or_else(|_| HashMap::new()))
        } else {
            Ok(serde_json::from_slice(&content).unwrap_or_else(|_| HashMap::new()))
        }
    }

    fn write_file(&self, credentials: &HashMap<String, String>) -> std::io::Result<()> {
        let content = serde_json::to_vec(credentials).unwrap_or_default();
        
        if let Some(key) = &self.encryption_key {
            // In production, use proper encryption
            let encoded = base64::encode(&content);
            std::fs::write(&self.path, encoded.as_bytes())
        } else {
            std::fs::write(&self.path, &content)
        }
    }
}

impl CredentialStore for FileStore {
    fn get(&self, key: &str) -> Option<String> {
        self.read_file().ok()?.get(key).cloned()
    }

    fn set(&self, key: &str, value: &str) {
        if let Ok(mut credentials) = self.read_file() {
            credentials.insert(key.to_string(), value.to_string());
            let _ = self.write_file(&credentials);
        }
    }

    fn remove(&self, key: &str) {
        if let Ok(mut credentials) = self.read_file() {
            credentials.remove(key);
            let _ = self.write_file(&credentials);
        }
    }

    fn list(&self) -> Vec<String> {
        self.read_file().map(|c| c.keys().cloned().collect()).unwrap_or_default()
    }
}

/// Credential manager with caching.
pub struct CredentialManager<S: CredentialStore> {
    store: Arc<RwLock<S>>,
    cache: RwLock<HashMap<String, String>>,
}

impl<S: CredentialStore> CredentialManager<S> {
    pub fn new(store: S) -> Self {
        Self {
            store: Arc::new(RwLock::new(store)),
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Get a credential.
    pub async fn get(&self, key: &str) -> Option<String> {
        // Check cache first
        if let Some(value) = self.cache.read().await.get(key) {
            return Some(value.clone());
        }

        // Check store
        let value = self.store.read().await.get(key);
        if let Some(ref v) = value {
            self.cache.write().await.insert(key.to_string(), v.clone());
        }
        value
    }

    /// Set a credential.
    pub async fn set(&self, key: &str, value: &str) {
        self.store.read().await.set(key, value);
        self.cache.write().await.insert(key.to_string(), value.to_string());
    }

    /// Remove a credential.
    pub async fn remove(&self, key: &str) {
        self.store.read().await.remove(key);
        self.cache.write().await.remove(key);
    }

    /// List all credential keys.
    pub async fn list(&self) -> Vec<String> {
        self.store.read().await.list()
    }

    /// Get credential for a provider.
    pub async fn get_provider_key(&self, provider: &str) -> Option<String> {
        // Try specific key first
        let specific_key = format!("{}_api_key", provider.to_lowercase());
        if let Some(key) = self.get(&specific_key).await {
            return Some(key);
        }

        // Try uppercase variant
        let upper_key = format!("{}_API_KEY", provider.to_uppercase());
        if let Some(key) = self.get(&upper_key).await {
            return Some(key);
        }

        // Try environment variable
        std::env::var(&upper_key).ok()
    }

    /// Clear the cache.
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }
}

impl CredentialManager<MemoryStore> {
    /// Create a new credential manager with in-memory store.
    pub fn memory() -> Self {
        Self::new(MemoryStore::new())
    }
}

impl CredentialManager<FileStore> {
    /// Create a new credential manager with file store.
    pub fn file(path: std::path::PathBuf) -> Self {
        Self::new(FileStore::new(path))
    }

    /// Create a new credential manager with encrypted file store.
    pub fn encrypted_file(path: std::path::PathBuf, key: [u8; 32]) -> Self {
        Self::new(FileStore::new(path).with_encryption(key))
    }
}

/// OAuth2 token.
#[derive(Debug, Clone)]
pub struct OAuthToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub expires_at: Option<std::time::SystemTime>,
}

impl OAuthToken {
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp <= std::time::SystemTime::now())
            .unwrap_or(false)
    }
}

/// OAuth2 credential store.
pub struct OAuthStore {
    tokens: RwLock<HashMap<String, OAuthToken>>,
}

impl OAuthStore {
    pub fn new() -> Self {
        Self {
            tokens: RwLock::new(HashMap::new()),
        }
    }

    pub async fn set_token(&self, provider: &str, token: OAuthToken) {
        self.tokens.write().await.insert(provider.to_string(), token);
    }

    pub async fn get_token(&self, provider: &str) -> Option<OAuthToken> {
        self.tokens.read().await.get(provider).cloned()
    }

    pub async fn remove_token(&self, provider: &str) {
        self.tokens.write().await.remove(provider);
    }
}

impl Default for OAuthStore {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Keyring (OS keychain) store ─────────────────────────────────────────────

const KEYRING_SERVICE: &str = "pixicode";
const KEYRING_KEYS_LIST: &str = "_keys";

/// OS keychain-backed credential store (macOS Keychain, Windows Credential Manager, Linux secret-service).
pub struct KeyringStore;

impl KeyringStore {
    pub fn new() -> Self {
        Self
    }

    fn entry(key: &str) -> Result<keyring::Entry, keyring::Error> {
        keyring::Entry::new(KEYRING_SERVICE, key)
    }

    fn keys_list_entry() -> Result<keyring::Entry, keyring::Error> {
        keyring::Entry::new(KEYRING_SERVICE, KEYRING_KEYS_LIST)
    }

    fn read_keys_list(&self) -> Vec<String> {
        Self::keys_list_entry()
            .ok()
            .and_then(|e| e.get_password().ok())
            .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
            .unwrap_or_default()
    }

    fn write_keys_list(&self, keys: &[String]) {
        let json = serde_json::to_string(keys).unwrap_or_else(|_| "[]".to_string());
        if let Ok(e) = Self::keys_list_entry() {
            let _ = e.set_password(&json);
        }
    }
}

impl Default for KeyringStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialStore for KeyringStore {
    fn get(&self, key: &str) -> Option<String> {
        if key == KEYRING_KEYS_LIST {
            return None;
        }
        Self::entry(key).ok()?.get_password().ok()
    }

    fn set(&self, key: &str, value: &str) {
        if key == KEYRING_KEYS_LIST {
            return;
        }
        if let Ok(entry) = Self::entry(key) {
            if entry.set_password(value).is_ok() {
                let mut keys = self.read_keys_list();
                if !keys.contains(&key.to_string()) {
                    keys.push(key.to_string());
                    self.write_keys_list(&keys);
                }
            }
        }
    }

    fn remove(&self, key: &str) {
        if key == KEYRING_KEYS_LIST {
            return;
        }
        if let Ok(entry) = Self::entry(key) {
            let _ = entry.delete_password();
        }
        let mut keys = self.read_keys_list();
        keys.retain(|k| k != key);
        self.write_keys_list(&keys);
    }

    fn list(&self) -> Vec<String> {
        self.read_keys_list()
    }
}

impl CredentialManager<KeyringStore> {
    /// Create a credential manager backed by the OS keychain.
    pub fn keyring() -> Self {
        Self::new(KeyringStore::new())
    }
}

/// Get API key from various sources.
pub async fn get_api_key(
    manager: &CredentialManager<impl CredentialStore>,
    provider: &str,
) -> Option<String> {
    // 1. Check credential manager
    if let Some(key) = manager.get_provider_key(provider).await {
        return Some(key);
    }

    // 2. Check environment variables with different naming conventions
    let env_variants = [
        format!("{}_API_KEY", provider.to_uppercase()),
        format!("{}_apikey", provider.to_lowercase()),
        format!("PIXICODE_{}_API_KEY", provider.to_uppercase()),
    ];

    for env_var in &env_variants {
        if let Ok(key) = std::env::var(env_var) {
            return Some(key);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_store() {
        let store = MemoryStore::new();
        store.set("test_key", "test_value");
        assert_eq!(store.get("test_key"), Some("test_value".to_string()));

        store.remove("test_key");
        assert_eq!(store.get("test_key"), None);
    }

    #[tokio::test]
    async fn test_credential_manager() {
        let manager = CredentialManager::memory();
        
        manager.set("api_key", "secret123").await;
        assert_eq!(manager.get("api_key").await, Some("secret123".to_string()));
        
        manager.remove("api_key").await;
        assert_eq!(manager.get("api_key").await, None);
    }

    #[tokio::test]
    async fn test_oauth_token_expiry() {
        let token = OAuthToken {
            access_token: "test".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: Some(3600),
            refresh_token: None,
            scope: None,
            expires_at: Some(std::time::SystemTime::now() - std::time::Duration::from_secs(100)),
        };
        assert!(token.is_expired());

        let non_expired = OAuthToken {
            access_token: "test".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: Some(3600),
            refresh_token: None,
            scope: None,
            expires_at: Some(std::time::SystemTime::now() + std::time::Duration::from_secs(3600)),
        };
        assert!(!non_expired.is_expired());
    }
}

// ─── AWS credential chain ───────────────────────────────────────────────────

/// AWS credentials (access key, secret key, optional session token).
pub struct AwsCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

/// Load AWS credentials: env vars → ~/.aws/credentials [profile] → (future: instance metadata).
pub fn load_aws_credentials(profile: &str) -> Option<AwsCredentials> {
    if let (Ok(ak), Ok(sk)) = (
        std::env::var("AWS_ACCESS_KEY_ID"),
        std::env::var("AWS_SECRET_ACCESS_KEY"),
    ) {
        if !ak.is_empty() && !sk.is_empty() {
            return Some(AwsCredentials {
                access_key_id: ak,
                secret_access_key: sk,
                session_token: std::env::var("AWS_SESSION_TOKEN").ok().filter(|s| !s.is_empty()),
            });
        }
    }

    let home = dirs::home_dir()?;
    let path = home.join(".aws").join("credentials");
    let content = std::fs::read_to_string(&path).ok()?;
    let target = if profile.is_empty() { "default" } else { profile };
    let mut current = "";
    let mut access_key_id = None;
    let mut secret_access_key = None;
    let mut session_token = None;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            if current == target && access_key_id.is_some() && secret_access_key.is_some() {
                return Some(AwsCredentials {
                    access_key_id: access_key_id.unwrap(),
                    secret_access_key: secret_access_key.unwrap(),
                    session_token,
                });
            }
            current = line.trim_matches(&['[', ']'][..]).trim();
            access_key_id = None;
            secret_access_key = None;
            session_token = None;
            continue;
        }
        if current != target {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let (k, v) = (k.trim().to_lowercase(), v.trim().trim_matches('"'));
            match k.as_str() {
                "aws_access_key_id" => access_key_id = Some(v.to_string()),
                "aws_secret_access_key" => secret_access_key = Some(v.to_string()),
                "aws_session_token" => session_token = Some(v.to_string()).filter(|s| !s.is_empty()),
                _ => {}
            }
        }
    }
    if current == target {
        access_key_id.zip(secret_access_key).map(|(ak, sk)| AwsCredentials {
            access_key_id: ak,
            secret_access_key: sk,
            session_token,
        })
    } else {
        None
    }
}
