use std::sync::Arc;

use mempalace_core::{
    EmbedderEvent, EventCapture, MemoryWriteEvent, PostToolEvent,
    PreToolEvent, SessionStartEvent, StopEvent, UserPromptEvent,
};

use jcode_memory_types::{MemoryCategory, MemoryEntry, MemoryProvider};

/// Bridge that forwards jcode runtime events into mempalace via the
/// jcode MemoryProvider interface (implemented by MempalaceProvider).
///
/// This makes mempalace aware of session activity so it can maintain
/// context across agent sessions.
pub struct EventCaptureBridge {
    memory: Arc<dyn MemoryProvider>,
}

impl EventCaptureBridge {
    /// Create a new bridge forwarding events to the given memory provider.
    pub fn new(memory: Arc<dyn MemoryProvider>) -> Self {
        Self { memory }
    }

    /// Convert to a boxed trait object for registration with jcode's
    /// event system.
    pub fn into_box(self) -> Box<dyn EventCapture + Send + Sync> {
        Box::new(self) as Box<dyn EventCapture + Send + Sync>
    }
}

impl EventCapture for EventCaptureBridge {
    fn on_session_start(&self, event: SessionStartEvent) {
        tracing::debug!(
            session_id = %event.session_id,
            project_dir = %event.project_dir,
            "forwarding session_start to mempalace"
        );

        let content = format!(
            "[session] Started session {} in {}",
            event.session_id, event.project_dir
        );
        let entry = MemoryEntry::new(MemoryCategory::Fact, content)
            .with_source("event_capture:session_start");

        if let Err(e) = self.memory.remember_project(entry) {
            tracing::warn!(error = %e, "failed to file session_start in mempalace");
        }
    }

    fn on_user_prompt_submit(&self, event: UserPromptEvent) {
        tracing::debug!(
            session_id = %event.session_id,
            preview = %event.preview,
            "forwarding user_prompt to mempalace"
        );

        // Use the full prompt if available, otherwise fall back to preview.
        let content = if event.prompt.len() > event.preview.len() && !event.preview.is_empty() {
            format!("[user] {}", event.prompt)
        } else {
            format!("[user] {}", event.preview)
        };

        let entry = MemoryEntry::new(MemoryCategory::Fact, content)
            .with_source("event_capture:user_prompt");

        if let Err(e) = self.memory.remember_project(entry) {
            tracing::warn!(error = %e, "failed to file user_prompt in mempalace");
        }
    }

    fn on_pre_tool_use(&self, event: PreToolEvent) {
        // Pre-tool events are high-frequency and don't add unique memory signal
        // beyond what the post-tool event already captures. Skip to avoid noise.
        tracing::trace!(
            tool_name = %event.tool_name,
            params_preview = %event.params_preview,
            "pre_tool (skipped)"
        );
    }

    fn on_post_tool_use(&self, event: PostToolEvent) {
        tracing::trace!(
            tool_name = %event.tool_name,
            result_summary = %event.result_summary,
            success = event.success,
            "forwarding post_tool to mempalace"
        );

        // Only file successful tool results — failures are logged but not filed
        // as they may contain sensitive error details.
        if !event.success {
            tracing::debug!(tool_name = %event.tool_name, "skipping failed tool result");
            return;
        }

        let content = format!(
            "[tool:{}] {}",
            event.tool_name, event.result_summary
        );
        let entry = MemoryEntry::new(MemoryCategory::Fact, content)
            .with_source("event_capture:tool_result");

        if let Err(e) = self.memory.remember_project(entry) {
            tracing::warn!(error = %e, "failed to file tool_result in mempalace");
        }
    }

    fn on_memory_write(&self, event: MemoryWriteEvent) {
        // Memory write events are jcode's own internal bookkeeping.
        // Forwarding them would create echo/feedback loops. Skip.
        tracing::debug!(
            operation = %event.operation,
            memory_id = %event.memory_id,
            success = event.success,
            "memory_write (skipped)"
        );
    }

    fn on_stop(&self, event: StopEvent) {
        tracing::debug!(
            session_id = ?event.session_id,
            "stop event received"
        );
        // Session stop is informational — mempalace tracks session lifetime
        // via the session_start event already filed above.
    }

    fn on_embedder_ready(&self, event: EmbedderEvent) {
        tracing::info!(
            model_name = %event.model_name,
            success = event.success,
            error = ?event.error,
            "embedder_ready (informational)"
        );
        // Embedder status is internal infrastructure telemetry.
        // Not useful for memory filing.
    }
}