use std::sync::Arc;

use mempalace_core::{
    EmbedderEvent, EventCapture, MemoryWriteEvent, PostToolEvent,
    PreToolEvent, SessionStartEvent, StopEvent, UserPromptEvent,
};

pub struct EventCaptureBridge {
    palace: Arc<dyn mempalace_core::MemoryProvider>,
}

impl EventCaptureBridge {
    pub fn new(palace: Arc<dyn mempalace_core::MemoryProvider>) -> Self {
        Self { palace }
    }

    pub fn into_box(self) -> Box<dyn EventCapture + Send + Sync> {
        Box::new(self) as Box<dyn EventCapture + Send + Sync>
    }
}

impl EventCapture for EventCaptureBridge {
    fn on_session_start(&self, event: SessionStartEvent) {
        tracing::debug!(
            session_id = %event.session_id,
            project_dir = %event.project_dir,
            "session_start"
        );
    }

    fn on_user_prompt_submit(&self, event: UserPromptEvent) {
        tracing::debug!(
            session_id = %event.session_id,
            preview = %event.preview,
            "user_prompt"
        );
    }

    fn on_pre_tool_use(&self, event: PreToolEvent) {
        tracing::trace!(
            tool_name = %event.tool_name,
            params_preview = %event.params_preview,
            "pre_tool"
        );
    }

    fn on_post_tool_use(&self, event: PostToolEvent) {
        tracing::trace!(
            tool_name = %event.tool_name,
            result_summary = %event.result_summary,
            success = event.success,
            "post_tool"
        );
    }

    fn on_memory_write(&self, event: MemoryWriteEvent) {
        tracing::debug!(
            operation = %event.operation,
            memory_id = %event.memory_id,
            success = event.success,
            "memory_write"
        );
    }

    fn on_stop(&self, event: StopEvent) {
        tracing::debug!(
            session_id = ?event.session_id,
            "stop"
        );
    }

    fn on_embedder_ready(&self, event: EmbedderEvent) {
        tracing::info!(
            model_name = %event.model_name,
            success = event.success,
            error = ?event.error,
            "embedder_ready"
        );
    }
}