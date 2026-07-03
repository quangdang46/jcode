use super::*;
use crate::tui::{TurnSummary, TuiState, detect_kv_cache_problem, ui};
use std::collections::{HashMap, HashSet};

impl App {
    pub(super) fn current_skills_snapshot(&self) -> std::sync::Arc<crate::skill::SkillRegistry> {
        self.registry
            .skills()
            .try_read()
            .map(|skills| std::sync::Arc::new(skills.clone()))
            .unwrap_or_else(|_| self.skills.clone())
    }

    pub fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn is_processing(&self) -> bool {
        self.is_processing || self.pending_queued_dispatch || self.split_launch_in_flight()
    }

    /// Keep a power inhibitor held while a turn is processing/streaming so the
    /// machine does not idle-sleep mid-stream. No-op on unsupported platforms.
    pub(super) fn sync_sleep_guard(&mut self) {
        self.power_inhibitor.set_active(self.is_processing());
    }

    pub fn streaming_text(&self) -> &str {
        &self.streaming.streaming_text
    }

    pub fn active_skill(&self) -> Option<&str> {
        self.active_skill.as_deref()
    }

    pub fn available_skills(&self) -> Vec<String> {
        let skills = self.current_skills_snapshot();
        skills.list().iter().map(|s| s.name.clone()).collect()
    }

    pub fn queued_count(&self) -> usize {
        self.queued_messages.len() + self.hidden_queued_system_messages.len()
    }

    pub fn queued_messages(&self) -> &[String] {
        &self.queued_messages
    }

    pub fn streaming_tokens(&self) -> (u64, u64) {
        (
            self.streaming.streaming_input_tokens,
            self.streaming.streaming_output_tokens,
        )
    }

    pub(super) fn build_turn_footer(&self, duration: Option<f32>) -> Option<String> {
        let mut parts = Vec::new();

        // Model and provider prefix (always present).
        let model = <Self as TuiState>::provider_model(self);
        let provider = <Self as TuiState>::provider_name(self);
        parts.push(model);
        parts.push(provider);

        if let Some(secs) = duration {
            let duration_ms = (secs.max(0.0) * 1000.0).round() as u64;
            parts.push(Message::format_duration(duration_ms));
        }
        if let Some(tps) = self.compute_streaming_tps() {
            parts.push(format!("{:.1} tps", tps));
        }
        if self.streaming.streaming_input_tokens > 0 || self.streaming.streaming_output_tokens > 0 {
            parts.push(format!(
                "↑{} ↓{}",
                format_tokens(self.streaming.streaming_input_tokens),
                format_tokens(self.streaming.streaming_output_tokens)
            ));
        }
        if let Some(cache) = format_cache_footer(
            self.streaming.streaming_cache_read_tokens,
            self.streaming.streaming_cache_creation_tokens,
        ) {
            parts.push(cache);
        }

        Some(parts.join(" · "))
    }

    pub(super) fn has_streaming_footer_stats(&self) -> bool {
        self.streaming.streaming_input_tokens > 0
            || self.streaming.streaming_output_tokens > 0
            || self.streaming.streaming_cache_read_tokens.is_some()
            || self.streaming.streaming_cache_creation_tokens.is_some()
            || self.compute_streaming_tps().is_some()
    }

    pub(super) fn push_turn_footer(&mut self, duration: Option<f32>) {
        self.log_cache_miss_if_unexpected();
        self.record_completed_stream_cache_usage();

        self.last_api_completed = Some(Instant::now());
        self.last_api_completed_provider = Some(<Self as TuiState>::provider_name(self));
        self.last_api_completed_model = Some(<Self as TuiState>::provider_model(self));
        self.last_turn_input_tokens = {
            let input = self.streaming.streaming_input_tokens;
            if input > 0 { Some(input) } else { None }
        };

        if let Some(footer) = self.build_turn_footer(duration) {
            self.push_display_message(DisplayMessage {
                role: "meta".to_string(),
                content: footer,
                tool_calls: vec![],
                duration_secs: None,
                title: None,
                tool_data: None,
            });
        }
    }

    /// Log detailed info when an unexpected cache miss occurs (cache write on turn 3+)
    pub(super) fn log_cache_miss_if_unexpected(&self) {
        let user_turn_count = self
            .display_messages
            .iter()
            .filter(|m| m.role == "user")
            .count();

        let provider = <Self as TuiState>::provider_name(self);
        let upstream_provider = self.upstream_provider();
        let cache_ttl = self.cache_ttl_status();
        let cache_problem = detect_kv_cache_problem(
            &provider,
            upstream_provider,
            user_turn_count,
            self.streaming.streaming_input_tokens,
            self.streaming.streaming_cache_read_tokens,
            self.streaming.streaming_cache_creation_tokens,
            cache_ttl.as_ref(),
        );

        if let Some(problem) = cache_problem {
            // Collect context for debugging
            let session_id = self.session_id().to_string();
            let model = <Self as TuiState>::provider_model(self);
            let input_tokens = self.streaming.streaming_input_tokens;
            let output_tokens = self.streaming.streaming_output_tokens;

            // Format as Option to distinguish None vs Some(0)
            let cache_creation_dbg =
                format!("{:?}", self.streaming.streaming_cache_creation_tokens);
            let cache_read_dbg = format!("{:?}", self.streaming.streaming_cache_read_tokens);

            // Count message types in conversation
            let mut user_msgs = 0;
            let mut assistant_msgs = 0;
            let mut tool_msgs = 0;
            let mut other_msgs = 0;
            for msg in &self.display_messages {
                match msg.role.as_str() {
                    "user" => user_msgs += 1,
                    "assistant" => assistant_msgs += 1,
                    "tool_result" | "tool_use" => tool_msgs += 1,
                    _ => other_msgs += 1,
                }
            }

            crate::logging::warn(&format!(
                "CACHE_MISS: {} on turn {} | \
                 cache_creation={} cache_read={} | \
                 input={} output={} affected={:?} | \
                 session={} provider={} upstream={:?} model={} | \
                 msgs: user={} assistant={} tool={} other={}",
                problem.log_reason(),
                user_turn_count,
                cache_creation_dbg,
                cache_read_dbg,
                input_tokens,
                output_tokens,
                problem.affected_tokens,
                session_id,
                provider,
                upstream_provider,
                model,
                user_msgs,
                assistant_msgs,
                tool_msgs,
                other_msgs
            ));
        }
    }

    /// Check if approaching context limit and show warning
    pub(super) fn check_context_warning(&mut self, input_tokens: u64) {
        let usage_percent = (input_tokens as f64 / self.context_limit as f64) * 100.0;

        // Warn at 70%, 80%, 90%
        if !self.context_warning_shown && usage_percent >= 70.0 {
            let warning = format!(
                "\n⚠️  Context usage: {:.0}% ({}/{}k tokens) - compaction approaching\n\n",
                usage_percent,
                input_tokens / 1000,
                self.context_limit / 1000
            );
            self.append_streaming_text(&warning);
            self.context_warning_shown = true;
        } else if self.context_warning_shown && usage_percent >= 80.0 {
            // Reset to show 80% warning
            if usage_percent < 85.0 {
                let warning = format!(
                    "\n⚠️  Context usage: {:.0}% - compaction imminent\n\n",
                    usage_percent
                );
                self.append_streaming_text(&warning);
            }
        }
    }

    /// Get context usage as percentage
    pub fn context_usage_percent(&self) -> f64 {
        self.current_stream_context_tokens()
            .map(|tokens| (tokens as f64 / self.context_limit as f64) * 100.0)
            .unwrap_or(0.0)
    }

    /// Time since last streaming event (for detecting stale connections)
    pub fn time_since_activity(&self) -> Option<Duration> {
        if let Some(last_activity) = self.last_stream_activity {
            return Some(last_activity.elapsed());
        }
        if !self.display_messages.is_empty() && !self.is_processing {
            return Some(crate::tui::REDRAW_DEEP_IDLE_AFTER + Duration::from_secs(1));
        }
        Some(self.app_started.elapsed())
    }

    pub(super) fn split_launch_in_flight(&self) -> bool {
        self.is_remote
            && !self.is_processing
            && self
                .pending_split_started_at
                .is_some_and(|started_at| started_at.elapsed() < Duration::from_millis(350))
    }

    pub fn streaming_tool_calls(&self) -> &[ToolCall] {
        &self.streaming_tool_calls
    }

    pub fn status(&self) -> &ProcessingStatus {
        &self.status
    }

    pub fn subagent_status(&self) -> Option<&str> {
        self.subagent_status.as_deref()
    }

    pub fn elapsed(&self) -> Option<Duration> {
        if let Some(d) = self.replay_elapsed_override {
            return Some(d);
        }
        if self.is_processing() {
            return self
                .visible_turn_started
                .or(self.processing_started)
                .map(|t| t.elapsed());
        }
        self.split_launch_in_flight()
            .then(|| self.pending_split_started_at.map(|t| t.elapsed()))
            .flatten()
    }

    pub(super) fn display_turn_duration_secs(&self) -> Option<f32> {
        self.visible_turn_started
            .or(self.processing_started)
            .map(|started| started.elapsed().as_secs_f32())
    }

    pub(super) fn clear_visible_turn_started(&mut self) {
        self.visible_turn_started = None;
    }

    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }

    pub fn provider_model(&self) -> String {
        self.provider.model()
    }

    /// Get the upstream provider (e.g., which provider OpenRouter routed to)
    pub fn upstream_provider(&self) -> Option<&str> {
        self.upstream_provider.as_deref()
    }

    pub fn mcp_servers(&self) -> Vec<(String, usize)> {
        self.mcp_server_names.clone()
    }

    /// Scroll to the previous user prompt (scroll up - earlier in conversation)
    pub fn scroll_to_prev_prompt(&mut self) {
        let positions = ui::last_user_prompt_positions();
        if positions.is_empty() {
            return;
        }
        // An explicit jump should win over a still-settling history prepend.
        self.pending_history_anchor = None;

        let current = self.scroll_offset;

        // positions are in document order (top to bottom).
        // Find the last position that is strictly less than current (i.e. earlier/above).
        // If we're at the bottom (!auto_scroll_paused), treat current as past-the-end.
        if !self.auto_scroll_paused {
            // Jump to the most recent (last) prompt
            if let Some(&pos) = positions.last() {
                self.scroll_offset = pos;
                self.auto_scroll_paused = true;
            }
            return;
        }

        let mut target = None;
        for &pos in positions.iter().rev() {
            if pos < current {
                target = Some(pos);
                break;
            }
        }

        if let Some(pos) = target {
            self.scroll_offset = pos;
        } else {
            // No earlier prompt is loaded. If older compacted history exists,
            // pull it in (anchored) and jump to the very top so the next press
            // continues into the freshly loaded prompts instead of stalling.
            if self.compacted_history_has_remaining() {
                self.scroll_offset = 0;
                self.auto_scroll_paused = true;
                self.maybe_queue_compacted_history_load();
            }
        }
    }

    /// Scroll to the next user prompt (scroll down - later in conversation)
    pub fn scroll_to_next_prompt(&mut self) {
        let positions = ui::last_user_prompt_positions();
        if positions.is_empty() || !self.auto_scroll_paused {
            return;
        }
        self.pending_history_anchor = None;

        let current = self.scroll_offset;

        // Find the first position strictly greater than current (i.e. later/below).
        for &pos in &positions {
            if pos > current {
                self.scroll_offset = pos;
                return;
            }
        }

        // No more prompts below - go to bottom
        self.follow_chat_bottom();
    }

    /// Scroll to Nth most-recent user prompt (1 = most recent, 2 = second most recent, etc.).
    /// Uses actual wrapped line positions from the last render frame for accurate placement,
    /// positioning the prompt at the top of the viewport.
    pub(super) fn scroll_to_recent_prompt_rank(&mut self, rank: usize) {
        let rank = rank.max(1);
        let positions = ui::last_user_prompt_positions();
        let max_scroll = ui::last_max_scroll();

        if positions.is_empty() {
            return;
        }
        self.pending_history_anchor = None;

        // positions are in document order (top to bottom), we want most-recent first
        let target_idx = positions.len().saturating_sub(rank);
        let target_line = positions[target_idx];
        self.set_status_notice(format!(
            "Ctrl+{}: idx={}/{} line={} max={}",
            rank,
            target_idx,
            positions.len(),
            target_line,
            max_scroll
        ));
        self.scroll_offset = target_line;
        self.auto_scroll_paused = true;
    }

    /// Scan `display_messages` and build per-turn summaries.
    /// For each assistant message with `duration_secs > 0`, count the tool types
    /// in the following tool messages that belong to that turn.
    /// After building summaries, collapse all completed turns except the newest
    /// one (the latest completed turn stays expanded). Bumps
    /// `display_messages_version` so the body cache rebuilds.
    pub(super) fn compute_turn_summaries_and_collapse(&mut self) {
        let messages = self.display_messages.as_slice();
        let mut summaries: Vec<Option<TurnSummary>> = Vec::new();
        let mut collapsed: HashSet<usize> = HashSet::new();

        // Track which tool messages belong to which turn.
        // We walk in display order:
        //   "user"     → starts a new turn
        //   "meta"     → turn footer with duration (marks turn end)
        //   "assistant"→ part of the current turn
        //   "tool"     → tool result, part of the current turn
        let mut turn_start: Option<usize> = None;       // msg index where current turn began
        let mut thinking_secs: u64 = 0;
        let mut tool_counts: HashMap<String, u32> = HashMap::new();
        let mut turn_label_line: Option<usize> = None;   // msg index of the meta/footer line
        let mut has_assistant: bool = false;

        // Helper: finalize the current turn into the summaries vec.
        // `end_pos` is the message position following the footer (past the turn).
        let finalize_turn = |summaries: &mut Vec<Option<TurnSummary>>,
                             collapsed: &mut HashSet<usize>,
                             turn_start: Option<usize>,
                             turn_label_line: &mut Option<usize>,
                             thinking_secs: &mut u64,
                             tool_counts: &mut HashMap<String, u32>,
                             has_assistant: &mut bool| {
            let Some(turn_line) = turn_label_line.take() else {
                return;
            };
            // Only create summaries for turns that had assistant output.
            if *has_assistant || !tool_counts.is_empty() {
                let summary = TurnSummary {
                    thinking_secs: *thinking_secs,
                    tool_counts: std::mem::take(tool_counts),
                };
                // Pad summaries vec to cover the footer line's position.
                while summaries.len() <= turn_line {
                    summaries.push(None);
                }
                summaries[turn_line] = Some(summary);
            }
            *thinking_secs = 0;
            *has_assistant = false;
            let _ = turn_start; // keep
        };

        for (msg_idx, msg) in messages.iter().enumerate() {
            match msg.effective_role() {
                "user" => {
                    // A new user turn starts. Finalize any prior in-progress turn
                    // that lacked a meta footer (should not normally happen, but
                    // defensively close it).
                    if turn_label_line.is_some() {
                        finalize_turn(
                            &mut summaries,
                            &mut collapsed,
                            turn_start,
                            &mut turn_label_line,
                            &mut thinking_secs,
                            &mut tool_counts,
                            &mut has_assistant,
                        );
                    }
                    turn_start = Some(msg_idx);
                }
                "assistant" => {
                    has_assistant = true;
                    if let Some(secs) = msg.duration_secs
                        && secs > 0.0
                    {
                        thinking_secs = secs as u64;
                    } else {
                        // Count elapsed from the active turn start if no
                        // explicit duration was recorded on this assistant
                        // message but we have the turn timer.
                    }
                }
                "tool" => {
                    if let Some(ref tc) = msg.tool_data {
                        let name = tc.name.clone();
                        *tool_counts.entry(name).or_insert(0) += 1;
                    }
                }
                "meta" => {
                    // A "meta" role marks a turn footer / summary line.
                    // Finalize the current turn at this index.
                    if let Some(secs) = msg.duration_secs
                        && secs > 0.0
                    {
                        thinking_secs = secs as u64;
                    }
                    turn_label_line = Some(msg_idx);
                    finalize_turn(
                        &mut summaries,
                        &mut collapsed,
                        turn_start,
                        &mut turn_label_line,
                        &mut thinking_secs,
                        &mut tool_counts,
                        &mut has_assistant,
                    );
                }
                _ => {}
            }
        }

        // Flush any trailing turn that ended without a meta footer.
        if turn_label_line.is_some() {
            finalize_turn(
                &mut summaries,
                &mut collapsed,
                turn_start,
                &mut turn_label_line,
                &mut thinking_secs,
                &mut tool_counts,
                &mut has_assistant,
            );
        }

        // Collapse all completed turns except the newest one.
        // A "completed turn" is one that has a summary.
        let completed_indices: Vec<usize> = summaries
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_some())
            .map(|(i, _)| i)
            .collect();

        // Keep the newest completed turn expanded, collapse the rest.
        if completed_indices.len() > 1 {
            for &idx in completed_indices.iter().rev().skip(1) {
                collapsed.insert(idx);
            }
        }

        let changed = summaries != self.turn_summaries || collapsed != self.collapsed_turns;
        if changed {
            self.turn_summaries = summaries;
            self.collapsed_turns = collapsed;
            self.display_messages_version = self.display_messages_version.wrapping_add(1);
        }
    }

    pub(super) fn toggle_input_stash(&mut self) {
        if let Some((stashed, stashed_cursor)) = self.stashed_input.take() {
            let current_input = std::mem::replace(&mut self.input, stashed);
            let current_cursor = std::mem::replace(&mut self.cursor_pos, stashed_cursor);
            if current_input.is_empty() {
                self.set_status_notice("📋 Input restored from stash");
            } else {
                self.stashed_input = Some((current_input, current_cursor));
                self.set_status_notice("📋 Swapped input with stash");
            }
        } else if !self.input.is_empty() {
            let input = std::mem::take(&mut self.input);
            let cursor = std::mem::replace(&mut self.cursor_pos, 0);
            self.stashed_input = Some((input, cursor));
            self.set_status_notice("📋 Input stashed");
        }
    }
}
