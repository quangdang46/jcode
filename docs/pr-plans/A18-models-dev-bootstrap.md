# A18 — Models.dev Auto-Bootstrap with Cache + Fingerprint

**Priority:** P1 | **Effort:** S | **Status:** ✅ Done

## Objective

Auto-bootstrap a models.dev YAML/JSON catalog on first run, cache it by
fingerprint of available provider IDs, and re-fetch when the fingerprint
changes. This gives jcode access to the complete [models.dev](https://models.dev)
provider/model catalog without requiring a fetch on every startup.

## Background

[models.dev](https://models.dev) publishes a `/api.json` endpoint that lists
providers and their available models with metadata (context windows, capabilities,
pricing tiers). opencode uses this as its authoritative model catalog source,
fetching it once at startup and caching it with a 5-minute TTL.

jcode currently relies on static model lists (`ALL_CLAUDE_MODELS`,
`ALL_OPENAI_MODELS`) plus per-provider API catalog fetches (Anthropic, OpenAI).
Adding models.dev support gives jcode a unified view of all known providers
and their models, especially useful for OpenAI-compatible providers that lack
a `/v1/models` endpoint.

## Design

### Core module: `jcode-provider-core/src/models_dev.rs`

A standalone `ModelsDevClient` that:

1. **Fetches** `https://models.dev/api.json` via reqwest HTTP client
2. **Caches** the raw JSON to disk (`~/.config/jcode/models_dev_cache.json`)
3. **Fingerprints** the local provider set — computed from sorted, concatenated
   provider IDs (`login_providers()` + `openai_compatible_profiles()`), hashed
   via SHA-256
4. **Re-fetches** only when the fingerprint changes (a provider was added or
   removed from jcode's static catalog)

### Cache invalidation strategy

| Condition | Action |
|-----------|--------|
| No cache file | Fetch, write cache |
| Fingerprint mismatch | Fetch, write cache (provider set changed) |
| Fingerprint matches, TTL expired | Return cached, background re-fetch |
| Fingerprint matches, TTL fresh | Return cached |

### Key data structures

```rust
pub type ModelsDevCatalog = HashMap<String, ModelsDevProvider>;

pub struct ModelsDevProvider {
    pub id: String,
    pub name: String,
    pub env: Vec<String>,
    pub npm: Option<String>,
    pub api: Option<String>,
    pub models: HashMap<String, ModelsDevModel>,
}

pub struct ModelsDevModel {
    pub id: String,
    pub name: String,
    pub release_date: String,
    pub reasoning: bool,
    pub tool_call: bool,
    pub limit: ModelsDevModelLimit,
    // …
}

pub struct ModelsDevCacheMeta {
    pub fingerprint: String,
    pub fetched_at_unix_secs: u64,
}
```

### Fingerprint computation

```rust
pub fn compute_fingerprint(provider_ids: &[impl AsRef<str>]) -> String {
    let mut sorted: Vec<&str> = ...;
    sorted.sort();
    let concatenated = sorted.join(",");
    let hash = stable_hash_str(&concatenated);
    format!("{:016x}", hash)
}
```

### Usage

```rust
let cache_dir = jcode_storage::app_config_dir()?;
let client = ModelsDevClient::new(cache_dir);

// Get all known provider IDs from the metadata crate
let provider_ids = jcode_provider_metadata::login_providers()
    .iter().map(|p| p.id).collect::<Vec<_>>();
let fingerprint = compute_fingerprint(&provider_ids);

match client.load_or_fetch(&fingerprint).await {
    Ok(catalog) => { /* populate models from catalog */ }
    Err(e) => { /* log warning, fall back to static lists */ }
}
```

## Files Changed

### Added
- `crates/jcode-provider-core/src/models_dev.rs` — new module with:
  - `ModelsDevProvider` / `ModelsDevModel` / `ModelsDevModelLimit` data types
  - `ModelsDevCatalog` type alias
  - `ModelsDevCacheMeta` for cache metadata
  - `ModelsDevClient` with `load_or_fetch()`, `force_fetch()`, `read_meta()`
  - `compute_fingerprint()` helper
  - `ModelsDevError` error type
  - Atomic file write via `atomic_write_json()`
  - Unit tests for fingerprinting, serialization, and cache meta

### Modified
- `crates/jcode-provider-core/Cargo.toml` — added `rand` dependency
- `crates/jcode-provider-core/src/lib.rs` — registered `models_dev` module

## Testing

- `compute_fingerprint_is_deterministic` — same IDs → same fingerprint
- `compute_fingerprint_changes_on_different_ids` — different IDs → different fingerprint
- `compute_fingerprint_stable_for_same_ids` — stable across calls
- `compute_fingerprint_empty_set` — empty set produces valid hash
- `models_dev_cache_meta_is_fresh_respects_ttl` — TTL check works
- `serde_roundtrip_catalog` — serialization roundtrip preserves data

## Integration Notes

The `ModelsDevClient` is designed to be wired into jcode's provider startup
(`src/cli/startup.rs` or `crates/jcode-base/src/provider_catalog.rs`) in a
follow-up PR by:

1. Computing the fingerprint from `jcode_provider_metadata::login_providers()`
2. Calling `ModelsDevClient::load_or_fetch()` early in startup
3. Passing the catalog to a `populate_models_from_models_dev()` function that
   seeds the context limit cache and provider model lists

## Reference

- opencode implementation: `packages/core/src/models-dev.ts`
- opencode schema: `packages/schema/src/models-dev.ts`
- opencode plugin: `packages/core/src/plugin/models-dev.ts`
