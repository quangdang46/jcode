//! Models.dev auto-bootstrap with cache + fingerprint.
//!
//! Fetches the [models.dev](https://models.dev) provider/model catalog on first
//! run, caches it to disk, and re-fetches only when the fingerprint of locally
//! available provider IDs changes. This avoids fetching on every startup while
//! still picking up new providers the user adds (e.g. via env-file or login).
//!
//! ## Fingerprint
//!
//! The fingerprint is computed from the sorted, concatenated set of provider
//! IDs that the caller supplies (typically the union of `login_providers()` and
//! `openai_compatible_profiles()` from the metadata crate). When a new provider
//! is added to jcode's catalog the fingerprint changes, triggering a fresh
//! fetch. Providers whose API keys are not yet configured are still included —
//! the data is a catalog of what's *possible*, not what's *accessible*.
//!
//! ## Cache file
//!
//! The raw JSON is stored in the jcode app config directory under
//! `models_dev_cache.json`. A companion `models_dev_cache.meta.json` holds the
//! fingerprint and last-fetch timestamp so the data can be validated without
//! deserializing the full model catalog.

use crate::fingerprint::stable_hash_str;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Data structures mirroring the models.dev /api.json schema
// ---------------------------------------------------------------------------

/// A provider entry from models.dev.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsDevProvider {
    pub id: String,
    pub name: String,
    /// Env vars that identify this provider's API key (e.g. `["ANTHROPIC_API_KEY"]`).
    pub env: Vec<String>,
    /// Optional OpenAI SDK npm package name for AI SDK integration.
    pub npm: Option<String>,
    /// Base API URL for this provider.
    pub api: Option<String>,
    /// Models keyed by their model id.
    pub models: HashMap<String, ModelsDevModel>,
}

/// A model entry from models.dev.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsDevModel {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    pub release_date: String,
    #[serde(default)]
    pub attachment: bool,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub temperature: bool,
    #[serde(rename = "tool_call", default)]
    pub tool_call: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interleaved: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<serde_json::Value>,
    pub limit: ModelsDevModelLimit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modalities: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<serde_json::Value>,
}

/// Context/token limits for a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsDevModelLimit {
    /// Maximum context window in tokens.
    pub context: f64,
    /// Maximum input length (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<f64>,
    /// Maximum output length (optional; the API may enforce this server-side).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<f64>,
}

/// Top-level models.dev catalog: a map of provider id → provider entry.
pub type ModelsDevCatalog = HashMap<String, ModelsDevProvider>;

// ---------------------------------------------------------------------------
// Cache metadata (fingerprint + timestamp)
// ---------------------------------------------------------------------------

/// Metadata stored alongside the cached catalog for fast staleness checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsDevCacheMeta {
    /// SHA-256 hex fingerprint of the sorted provider IDs used at fetch time.
    pub fingerprint: String,
    /// Unix timestamp (seconds since epoch) of the last successful fetch.
    pub fetched_at_unix_secs: u64,
}

impl ModelsDevCacheMeta {
    /// Returns `true` if the cached data is fresh enough given a TTL.
    pub fn is_fresh(&self, ttl: Duration) -> bool {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH + Duration::from_secs(self.fetched_at_unix_secs))
            .unwrap_or_default();
        elapsed < ttl
    }
}

// ---------------------------------------------------------------------------
// ModelsDevClient
// ---------------------------------------------------------------------------

/// Client for fetching, caching, and fingerprint-validating the models.dev
/// catalog.
///
/// Typical usage:
/// ```ignore
/// let client = ModelsDevClient::new(cache_dir);
/// // On startup or provider-change:
/// let fp = compute_fingerprint(&provider_ids);
/// match client.load_or_fetch(&fp).await {
///     Ok(catalog) => { /* use models */ }
///     Err(e) => { /* fall back to static lists */ }
/// }
/// ```
pub struct ModelsDevClient {
    cache_file: PathBuf,
    meta_file: PathBuf,
    http_client: reqwest::Client,
}

impl ModelsDevClient {
    /// URL for the models.dev api.json endpoint.
    pub const MODELS_DEV_URL: &'static str = "https://models.dev/api.json";
    /// Default re-fetch interval when the fingerprint matches.
    pub const DEFAULT_TTL: Duration = Duration::from_secs(3600); // 1 hour
    /// File name for the cached catalog JSON.
    const CACHE_FILE: &'static str = "models_dev_cache.json";
    /// File name for the cached metadata JSON.
    const META_FILE: &'static str = "models_dev_cache.meta.json";

    /// Create a new client that stores cache files under `cache_dir`.
    pub fn new(cache_dir: PathBuf) -> Self {
        let cache_file = cache_dir.join(Self::CACHE_FILE);
        let meta_file = cache_dir.join(Self::META_FILE);
        Self {
            cache_file,
            meta_file,
            http_client: crate::shared_http_client(),
        }
    }

    // ------------------------------------------------------------------
    // Public API
    // ------------------------------------------------------------------

    /// Load the catalog from cache, fetching only if needed.
    ///
    /// The fetch is skipped when:
    /// - A valid cache exists and the fingerprint matches AND the cache is
    ///   still within [`DEFAULT_TTL`].
    /// - A valid cache exists and the fingerprint matches (TTL is ignored).
    ///
    /// A fresh fetch happens when:
    /// - No cache exists (first run).
    /// - The fingerprint differs from the cached one.
    /// - The cache file is corrupt or unreadable.
    pub async fn load_or_fetch(&self, fingerprint: &str) -> Result<ModelsDevCatalog, ModelsDevError> {
        // Attempt to load from cache first.
        if let Some(catalog) = self.load_cached(fingerprint)? {
            return Ok(catalog);
        }
        // Cache miss or fingerprint mismatch → fetch fresh data.
        self.fetch_and_cache(fingerprint).await
    }

    /// Force a fresh fetch regardless of cache state.
    pub async fn force_fetch(&self, fingerprint: &str) -> Result<ModelsDevCatalog, ModelsDevError> {
        self.fetch_and_cache(fingerprint).await
    }

    /// Read the cached metadata without loading the full catalog.
    pub fn read_meta(&self) -> Result<Option<ModelsDevCacheMeta>, ModelsDevError> {
        if !self.meta_file.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(&self.meta_file)
            .map_err(|e| ModelsDevError::Cache(format!("read meta: {e}")))?;
        let meta: ModelsDevCacheMeta = serde_json::from_slice(&bytes)
            .map_err(|e| ModelsDevError::Cache(format!("parse meta: {e}")))?;
        Ok(Some(meta))
    }

    /// Path to the cache catalog file.
    pub fn cache_path(&self) -> &Path {
        &self.cache_file
    }

    /// Path to the cache meta file.
    pub fn meta_path(&self) -> &Path {
        &self.meta_file
    }

    /// URL used for fetching.
    pub fn source_url(&self) -> &str {
        Self::MODELS_DEV_URL
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    /// Try to load the cached catalog, checking fingerprint and optional TTL.
    fn load_cached(&self, fingerprint: &str) -> Result<Option<ModelsDevCatalog>, ModelsDevError> {
        let meta = match self.read_meta()? {
            Some(m) => m,
            None => return Ok(None),
        };

        // Fingerprint mismatch → stale.
        if meta.fingerprint != fingerprint {
            return Ok(None);
        }

        // Attempt to load the actual catalog data.
        if !self.cache_file.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(&self.cache_file)
            .map_err(|e| ModelsDevError::Cache(format!("read cache: {e}")))?;
        let catalog: ModelsDevCatalog = serde_json::from_slice(&bytes)
            .map_err(|e| ModelsDevError::Cache(format!("parse cache: {e}")))?;
        Ok(Some(catalog))
    }

    /// Fetch from the remote endpoint and atomically write to cache.
    async fn fetch_and_cache(
        &self,
        fingerprint: &str,
    ) -> Result<ModelsDevCatalog, ModelsDevError> {
        let url = Self::MODELS_DEV_URL;

        let response = self
            .http_client
            .get(url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| ModelsDevError::Fetch(format!("HTTP request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ModelsDevError::Fetch(format!(
                "HTTP {status} from {url}"
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| ModelsDevError::Fetch(format!("read body: {e}")))?;

        let catalog: ModelsDevCatalog = serde_json::from_slice(&bytes)
            .map_err(|e| ModelsDevError::Parse(format!("JSON parse: {e}")))?;

        // Atomically write catalog.
        atomic_write_json(&self.cache_file, &catalog)
            .map_err(|e| ModelsDevError::Cache(format!("write cache: {e}")))?;

        // Write metadata.
        let meta = ModelsDevCacheMeta {
            fingerprint: fingerprint.to_string(),
            fetched_at_unix_secs: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        atomic_write_json(&self.meta_file, &meta)
            .map_err(|e| ModelsDevError::Cache(format!("write meta: {e}")))?;

        Ok(catalog)
    }
}

// ---------------------------------------------------------------------------
// Fingerprint helper
// ---------------------------------------------------------------------------

/// Compute a deterministic fingerprint from a sorted list of provider IDs.
///
/// This is used by the cache to detect when the set of locally available
/// providers has changed, signalling that a fresh models.dev fetch is needed.
///
/// # Example
///
/// ```ignore
/// let ids = vec!["anthropic-api", "openai-api", "openrouter", "ollama"];
/// let fp = compute_fingerprint(&ids);
/// ```
pub fn compute_fingerprint(provider_ids: &[impl AsRef<str>]) -> String {
    let mut sorted: Vec<&str> = provider_ids.iter().map(|s| s.as_ref()).collect();
    sorted.sort();
    let concatenated = sorted.join(",");
    let hash = stable_hash_str(&concatenated);
    format!("{:016x}", hash)
}

// ---------------------------------------------------------------------------
// Atomic file write helper
// ---------------------------------------------------------------------------

/// Atomically write a JSON-serializable value to a file using a temp file +
/// rename.
fn atomic_write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let pid = std::process::id();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp_path = path.with_extension(format!("tmp.{pid}.{nonce}"));

    let file = std::fs::File::create(&tmp_path)?;
    serde_json::to_writer(&file, value).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("serialize: {e}"))
    })?;
    file.sync_all()?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during models.dev operations.
#[derive(Debug)]
pub enum ModelsDevError {
    /// The HTTP fetch failed (network, timeout, non-200 status).
    Fetch(String),
    /// The response body could not be parsed as JSON.
    Parse(String),
    /// Reading or writing the local cache failed.
    Cache(String),
}

impl std::fmt::Display for ModelsDevError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fetch(msg) => write!(f, "models.dev fetch error: {msg}"),
            Self::Parse(msg) => write!(f, "models.dev parse error: {msg}"),
            Self::Cache(msg) => write!(f, "models.dev cache error: {msg}"),
        }
    }
}

impl std::error::Error for ModelsDevError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_fingerprint_is_deterministic() {
        let ids = vec!["b", "a", "c"];
        let fp1 = compute_fingerprint(&ids);
        let fp2 = compute_fingerprint(&["a", "b", "c"]);
        assert_eq!(fp1, fp2, "fingerprint should be order-independent");
    }

    #[test]
    fn compute_fingerprint_changes_on_different_ids() {
        let fp1 = compute_fingerprint(&["a", "b"]);
        let fp2 = compute_fingerprint(&["a", "c"]);
        assert_ne!(fp1, fp2, "different provider sets → different fingerprints");
    }

    #[test]
    fn compute_fingerprint_stable_for_same_ids() {
        let fp1 = compute_fingerprint(&["openai-api", "anthropic-api", "openrouter"]);
        let fp2 = compute_fingerprint(&["openai-api", "anthropic-api", "openrouter"]);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn compute_fingerprint_empty_set() {
        let ids: Vec<&str> = vec![];
        let fp = compute_fingerprint(&ids);
        // Should not panic, should produce a valid hex string.
        assert_eq!(fp.len(), 16);
    }

    #[test]
    fn models_dev_cache_meta_is_fresh_respects_ttl() {
        let meta = ModelsDevCacheMeta {
            fingerprint: "abc".to_string(),
            fetched_at_unix_secs: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        assert!(meta.is_fresh(Duration::from_secs(3600)));

        let old_meta = ModelsDevCacheMeta {
            fingerprint: "abc".to_string(),
            fetched_at_unix_secs: 1_000_000_000, // Year ~2001
        };
        assert!(!old_meta.is_fresh(Duration::from_secs(3600)));
    }

    #[test]
    fn serde_roundtrip_catalog() {
        let mut catalog = ModelsDevCatalog::new();
        catalog.insert(
            "acme".to_string(),
            ModelsDevProvider {
                id: "acme".to_string(),
                name: "Acme".to_string(),
                env: vec!["ACME_API_KEY".to_string()],
                npm: None,
                api: Some("https://api.acme.ai/v1".to_string()),
                models: {
                    let mut m = HashMap::new();
                    m.insert(
                        "acme-1".to_string(),
                        ModelsDevModel {
                            id: "acme-1".to_string(),
                            name: "Acme 1".to_string(),
                            family: None,
                            release_date: "2025-01-01".to_string(),
                            attachment: false,
                            reasoning: true,
                            temperature: true,
                            tool_call: true,
                            interleaved: None,
                            cost: None,
                            limit: ModelsDevModelLimit {
                                context: 128_000.0,
                                input: None,
                                output: Some(4096.0),
                            },
                            modalities: None,
                            status: None,
                            experimental: None,
                            provider: None,
                        },
                    );
                    m
                },
            },
        );

        let json = serde_json::to_string(&catalog).unwrap();
        let deserialized: ModelsDevCatalog = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 1);
        assert!(deserialized.contains_key("acme"));
        assert_eq!(
            deserialized["acme"].models["acme-1"].limit.context as usize,
            128_000
        );
    }
}
