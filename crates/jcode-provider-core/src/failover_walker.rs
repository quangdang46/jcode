//! Reactive failover walker — detects provider errors in-flight and
//! orchestrates automatic fallback to the next best model/route.
//!
//! ## Architecture
//!
//! This module composes three existing pieces:
//! 1. [`classify_failover_error_message_structured`] — error classification
//! 2. [`pick_next_fallback_route`] — next best route selection
//! 3. [`FailoverDecision`] — what action to take for an error
//!
//! It adds the **reactive walker** state machine that:
//! • Tracks per-session fallback state (current model, fallback index, cooldowns)
//! • Detects provider failures in-flight and aborts the current request
//! • Picks the next available model from the fallback chain
//! • Respects cooldowns to avoid burning retries on broken providers
//! • Equivalence-detection to avoid pointless same-model switches
//!
//! Reference: oh-my-openagent `runtime-fallback/` (~55 files)

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::failover::{classify_failover_error_message_structured, ErrorCode, FailoverDecision};
use crate::fallback_pick::pick_next_fallback_route;
use crate::ModelRoute;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Per-session walker state.
#[derive(Debug, Clone)]
pub struct WalkState {
    /// The original model the session started with.
    pub original_model: String,
    /// The currently-active model after any fallbacks.
    pub current_model: String,
    /// Index in the fallback chain we've walked to (0 = none yet).
    pub fallback_index: usize,
    /// Models that failed recently, mapped to the Instant they failed (for cooldown).
    pub failed_models: HashMap<String, Instant>,
    /// Total attempt count for this session.
    pub attempt_count: u32,
}

/// Result of preparing a fallback.
#[derive(Debug)]
pub struct PreparedFallback {
    pub success: bool,
    pub new_model: Option<String>,
    pub error: Option<String>,
    pub max_attempts_reached: bool,
}

/// Result of walking a failover.
#[derive(Debug)]
pub struct WalkResult {
    pub should_failover: bool,
    pub new_model: Option<String>,
    pub decision: FailoverDecision,
    pub error_code: Option<ErrorCode>,
    pub message: String,
}

// ---------------------------------------------------------------------------
// ReactiveFailoverWalker
// ---------------------------------------------------------------------------

/// Reactive failover walker that orchestrates provider fallback.
pub struct ReactiveFailoverWalker {
    /// Per-session walk states.
    sessions: HashMap<String, WalkState>,
    /// Sessions whose current request was internally aborted for fallback.
    internally_aborted: HashSet<String>,
    /// Default cooldown duration for failed models.
    cooldown: Duration,
    /// Maximum attempts per session before giving up.
    max_attempts: u32,
}

impl Default for ReactiveFailoverWalker {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            internally_aborted: HashSet::new(),
            cooldown: Duration::from_secs(120),
            max_attempts: 5,
        }
    }
}

impl ReactiveFailoverWalker {
    pub fn new(cooldown_secs: u64, max_attempts: u32) -> Self {
        Self {
            sessions: HashMap::new(),
            internally_aborted: HashSet::new(),
            cooldown: Duration::from_secs(cooldown_secs),
            max_attempts,
        }
    }

    /// Register a new session with its initial model.
    pub fn register_session(&mut self, session_id: &str, model: &str) {
        self.sessions.insert(
            session_id.to_string(),
            WalkState {
                original_model: model.to_string(),
                current_model: model.to_string(),
                fallback_index: 0,
                failed_models: HashMap::new(),
                attempt_count: 0,
            },
        );
    }

    /// Remove a session's state (session ended).
    pub fn unregister_session(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
        self.internally_aborted.remove(session_id);
    }

    /// Get the current walk state for a session.
    pub fn get_state(&self, session_id: &str) -> Option<&WalkState> {
        self.sessions.get(session_id)
    }

    /// Mark a session as internally aborted (so its error handler doesn't
    /// reset the attempt count).
    pub fn mark_internally_aborted(&mut self, session_id: &str) {
        self.internally_aborted.insert(session_id.to_string());
    }

    /// Check if a session was internally aborted.
    pub fn is_internally_aborted(&self, session_id: &str) -> bool {
        self.internally_aborted.contains(session_id)
    }

    /// Record a successful completion for a session (clear cooldowns
    /// for the current model).
    pub fn record_success(&mut self, session_id: &str) {
        if let Some(state) = self.sessions.get_mut(session_id) {
            state.failed_models.remove(&state.current_model);
        }
    }

    /// Record a failure for a model (add to cooldown map).
    pub fn record_failure(&mut self, session_id: &str, model: &str) {
        if let Some(state) = self.sessions.get_mut(session_id) {
            state
                .failed_models
                .insert(model.to_string(), Instant::now());
        }
    }

    /// Check if a model is in cooldown for the given session.
    pub fn is_model_in_cooldown(&self, session_id: &str, model: &str) -> bool {
        self.sessions
            .get(session_id)
            .and_then(|s| s.failed_models.get(model))
            .map(|failed_at| failed_at.elapsed() < self.cooldown)
            .unwrap_or(false)
    }

    /// Find the next available fallback model from the chain, respecting
    /// cooldowns and equivalence.
    ///
    /// Returns `None` when every candidate is in cooldown or equivalent.
    pub fn find_next_available_fallback<'a>(
        &self,
        session_id: &str,
        fallback_models: &'a [String],
    ) -> Option<&'a str> {
        let state = match self.sessions.get(session_id) {
            Some(s) => s,
            None => return None,
        };

        for model in fallback_models.iter().skip(state.fallback_index) {
            // Skip if equivalent to current model
            if models_equivalent(model, &state.current_model) {
                continue;
            }
            // Skip if in cooldown
            if self.is_model_in_cooldown(session_id, model) {
                continue;
            }
            return Some(model.as_str());
        }
        None
    }

    /// Prepare a fallback: given the current session state and the fallback
    /// model chain, pick the next candidate.
    ///
    /// This is the pure-logic equivalent of oh-my-openagent's
    /// `prepareFallback()` from `fallback-state.ts`.
    pub fn prepare_fallback(
        &self,
        session_id: &str,
        fallback_models: &[String],
    ) -> PreparedFallback {
        let state = match self.sessions.get(session_id) {
            Some(s) => s,
            None => {
                return PreparedFallback {
                    success: false,
                    new_model: None,
                    error: Some("session not registered".to_string()),
                    max_attempts_reached: false,
                }
            }
        };

        if state.attempt_count >= self.max_attempts {
            return PreparedFallback {
                success: false,
                new_model: None,
                error: Some(format!("max attempts ({}) reached", self.max_attempts)),
                max_attempts_reached: true,
            };
        }

        match self.find_next_available_fallback(session_id, fallback_models) {
            Some(model) => PreparedFallback {
                success: true,
                new_model: Some(model.to_string()),
                error: None,
                max_attempts_reached: false,
            },
            None => PreparedFallback {
                success: false,
                new_model: None,
                error: Some("no available fallback models".to_string()),
                max_attempts_reached: false,
            },
        }
    }

    /// Walk the failover: classify an error → decide what to do →
    /// if retryable, prepare fallback.
    ///
    /// This is the main orchestration entry point, called when a provider
    /// error occurs mid-stream.
    pub fn walk_failover(
        &mut self,
        session_id: &str,
        error_message: &str,
        routes: &[ModelRoute],
        current_model: &str,
        current_provider: &str,
        current_api_method: &str,
    ) -> WalkResult {
        // 1. Classify the error
        let (decision, error_code) =
            classify_failover_error_message_structured(error_message, None, None, None, None);

        // Ensure session is registered
        if !self.sessions.contains_key(session_id) {
            self.register_session(session_id, current_model);
        }

        // 2. Handle non-failover decisions
        if !decision.should_failover() {
            let code_str = error_code
                .map(|c| c.as_str().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            return WalkResult {
                should_failover: false,
                new_model: None,
                decision,
                error_code,
                message: format!("{} (error: {})", decision.as_str(), code_str),
            };
        }

        // 3. Mark provider unavailable if decision says so
        if decision.should_mark_provider_unavailable() {
            self.record_failure(session_id, current_model);
        }

        // 4. Pick the next available route
        let pick =
            pick_next_fallback_route(routes, current_model, current_provider, current_api_method);

        match pick {
            Some(index) => {
                let new_model = routes[index].model.clone();
                // Update walk state
                if let Some(state) = self.sessions.get_mut(session_id) {
                    state.current_model = new_model.clone();
                    state.fallback_index += 1;
                    state.attempt_count += 1;
                }
                // Mark as internally aborted so error handlers know
                self.mark_internally_aborted(session_id);

                let code_str = error_code
                    .map(|c| c.as_str().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                WalkResult {
                    should_failover: true,
                    new_model: Some(new_model.clone()),
                    decision,
                    error_code,
                    message: format!(
                        "Failing over from {} to {} ({}): {}",
                        current_model,
                        new_model,
                        code_str,
                        error_message.lines().next().unwrap_or(error_message)
                    ),
                }
            }
            None => WalkResult {
                should_failover: false,
                new_model: None,
                decision,
                error_code,
                message: "No fallback route available".to_string(),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Check if two model identifiers are equivalent (same model, diff endpoint).
fn models_equivalent(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn route(model: &str, provider: &str, api_method: &str) -> ModelRoute {
        ModelRoute {
            model: model.to_string(),
            provider: provider.to_string(),
            api_method: api_method.to_string(),
            available: true,
            detail: String::new(),
            cheapness: None,
        }
    }

    #[test]
    fn test_session_lifecycle() {
        let mut walker = ReactiveFailoverWalker::default();
        walker.register_session("sess-1", "claude-sonnet-4");
        let state = walker.get_state("sess-1").unwrap();
        assert_eq!(state.original_model, "claude-sonnet-4");
        assert_eq!(state.current_model, "claude-sonnet-4");
        assert_eq!(state.fallback_index, 0);
        assert_eq!(state.attempt_count, 0);

        walker.unregister_session("sess-1");
        assert!(walker.get_state("sess-1").is_none());
    }

    #[test]
    fn test_cooldown_tracking() {
        let mut walker = ReactiveFailoverWalker::new(60, 5);
        walker.register_session("sess-1", "gpt-5");
        walker.record_failure("sess-1", "gpt-5");

        // Immediately after failure, model should be in cooldown
        assert!(walker.is_model_in_cooldown("sess-1", "gpt-5"));

        // Record success should clear cooldown
        walker.record_success("sess-1");
        assert!(!walker.is_model_in_cooldown("sess-1", "gpt-5"));
    }

    #[test]
    fn test_find_next_available_fallback_skips_equivalent() {
        let mut walker = ReactiveFailoverWalker::default();
        walker.register_session("sess-1", "claude-sonnet-4");

        let fallbacks = vec![
            "claude-sonnet-4".to_string(), // equivalent → skip
            "claude-haiku-4".to_string(),  // different → pick
        ];

        let next = walker.find_next_available_fallback("sess-1", &fallbacks);
        assert_eq!(next, Some("claude-haiku-4"));
    }

    #[test]
    fn test_find_next_available_fallback_skips_cooldown() {
        let mut walker = ReactiveFailoverWalker::new(9999, 5);
        walker.register_session("sess-1", "claude-sonnet-4");
        walker.record_failure("sess-1", "claude-haiku-4");

        let fallbacks = vec![
            "claude-haiku-4".to_string(), // in cooldown → skip
            "gpt-5".to_string(),          // fresh → pick
        ];

        let next = walker.find_next_available_fallback("sess-1", &fallbacks);
        assert_eq!(next, Some("gpt-5"));
    }

    #[test]
    fn test_prepare_fallback_max_attempts() {
        let mut walker = ReactiveFailoverWalker::new(60, 2);
        walker.register_session("sess-1", "claude-sonnet-4");

        let fallbacks = vec!["gpt-5".to_string()];

        // First attempt
        let r1 = walker.prepare_fallback("sess-1", &fallbacks);
        assert!(r1.success);
        // Manually simulate hitting max
        if let Some(state) = walker.sessions.get_mut("sess-1") {
            state.attempt_count = 2;
        }

        let r2 = walker.prepare_fallback("sess-1", &fallbacks);
        assert!(!r2.success);
        assert!(r2.max_attempts_reached);
    }

    #[test]
    fn test_walk_failover_rate_limited_finds_fallback() {
        let mut walker = ReactiveFailoverWalker::default();
        walker.register_session("sess-1", "claude-sonnet-4");

        let routes = vec![
            route("claude-sonnet-4", "Anthropic", "claude-api"),
            route("claude-sonnet-4", "Anthropic", "claude-oauth"),
        ];

        let result = walker.walk_failover(
            "sess-1",
            "429 Too Many Requests",
            &routes,
            "claude-sonnet-4",
            "Anthropic",
            "claude-api",
        );

        assert!(result.should_failover);
        assert!(result.new_model.is_some());
        assert_eq!(result.error_code, Some(ErrorCode::RateLimited));
        // Should prefer same model with different auth method
        assert_eq!(result.new_model.as_deref(), Some("claude-sonnet-4"));
    }

    #[test]
    fn test_walk_failover_context_length_retries() {
        let mut walker = ReactiveFailoverWalker::default();
        walker.register_session("sess-1", "claude-sonnet-4");

        let routes = vec![route("gpt-5", "OpenAI", "openai-oauth")];

        let result = walker.walk_failover(
            "sess-1",
            "maximum context length is 200000 tokens",
            &routes,
            "claude-sonnet-4",
            "Anthropic",
            "claude-api",
        );

        // Context length errors use RetryNextProvider — they DO failover
        // to a different model that may have a larger context window.
        assert!(result.should_failover);
        assert_eq!(result.error_code, Some(ErrorCode::ContextLengthExceeded));
    }

    #[test]
    fn test_walk_failover_internally_aborted_tracking() {
        let mut walker = ReactiveFailoverWalker::default();
        walker.register_session("sess-1", "gpt-5");

        let routes = vec![route("claude-sonnet-4", "Anthropic", "claude-oauth")];

        walker.walk_failover(
            "sess-1",
            "502 Bad Gateway",
            &routes,
            "gpt-5",
            "OpenAI",
            "openai-key",
        );

        assert!(walker.is_internally_aborted("sess-1"));
    }

    #[test]
    fn test_walk_failover_no_routes_available() {
        let mut walker = ReactiveFailoverWalker::default();
        walker.register_session("sess-1", "claude-sonnet-4");

        // Only the current route (no alternatives)
        let routes = vec![route("claude-sonnet-4", "Anthropic", "claude-api")];

        let result = walker.walk_failover(
            "sess-1",
            "503 Service Unavailable",
            &routes,
            "claude-sonnet-4",
            "Anthropic",
            "claude-api",
        );

        assert!(!result.should_failover);
        assert!(result.new_model.is_none());
    }

    #[test]
    fn test_models_equivalent() {
        assert!(models_equivalent("claude-sonnet-4", "  claude-sonnet-4  "));
        assert!(models_equivalent("GPT-5", "gpt-5"));
        assert!(!models_equivalent("claude-sonnet-4", "claude-haiku-4"));
    }

    #[test]
    fn test_unregister_cleans_internally_aborted() {
        let mut walker = ReactiveFailoverWalker::default();
        walker.register_session("sess-1", "gpt-5");
        walker.mark_internally_aborted("sess-1");
        assert!(walker.is_internally_aborted("sess-1"));

        walker.unregister_session("sess-1");
        assert!(!walker.is_internally_aborted("sess-1"));
    }
}
