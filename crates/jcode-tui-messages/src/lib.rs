// Phase 5 widget work - stubbed for Phase 1.3 compilation
use ftui_text::text::Line;

#[derive(Debug, Clone)]
pub struct MessageCacheContext;

pub fn centered_wrap_width(_area_width: u16) -> usize { 80 }
pub fn get_cached_message_lines(_msg_id: u64) -> Vec<Line<'static>> { Vec::new() }
pub fn left_pad_lines_for_centered_mode(_lines: &mut [Line<'static>], _area_width: u16) {}

#[derive(Debug, Clone)]
pub struct DisplayMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Vec<String>,
    pub duration_secs: Option<f32>,
    pub title: Option<String>,
    pub tool_data: Option<jcode_message_types::ToolCall>,
}
impl DisplayMessage {
    pub fn error(_msg: impl Into<String>) -> Self {
        Self {
            role: "error".to_string(),
            content: _msg.into(),
            tool_calls: Vec::new(),
            duration_secs: None,
            title: None,
            tool_data: None,
        }
    }
    pub fn system(_msg: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: _msg.into(),
            tool_calls: Vec::new(),
            duration_secs: None,
            title: None,
            tool_data: None,
        }
    }
    pub fn user(_msg: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: _msg.into(),
            tool_calls: Vec::new(),
            duration_secs: None,
            title: None,
            tool_data: None,
        }
    }
    pub fn assistant(_msg: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: _msg.into(),
            tool_calls: Vec::new(),
            duration_secs: None,
            title: None,
            tool_data: None,
        }
    }
    pub fn tool_text(_msg: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: _msg.into(),
            tool_calls: Vec::new(),
            duration_secs: None,
            title: None,
            tool_data: None,
        }
    }
    pub fn meta(_msg: impl Into<String>) -> Self {
        Self {
            role: "meta".to_string(),
            content: _msg.into(),
            tool_calls: Vec::new(),
            duration_secs: None,
            title: None,
            tool_data: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranscriptPreviewLabels;
impl TranscriptPreviewLabels {
    pub const DESKTOP: Self = Self;
}

pub fn display_messages_from_rendered_messages(_messages: &[DisplayMessage]) -> Vec<Line<'static>> { Vec::new() }
pub fn latest_user_transcript_preview<'a, I>(_messages: I, _char_limit: usize) -> Option<String>
where
    I: DoubleEndedIterator<Item = (&'a str, &'a str)>,
{ None }
pub fn normalize_transcript_preview_text(_text: &str) -> String { String::new() }
pub fn transcript_preview_line(
    _role: &str,
    _content: &str,
    _char_limit: usize,
    _labels: TranscriptPreviewLabels,
) -> Option<String> { None }
pub fn transcript_preview_lines<'a, I>(_messages: I, _limit: usize, _char_limit: usize, _labels: TranscriptPreviewLabels) -> Vec<String>
where
    I: DoubleEndedIterator<Item = (&'a str, &'a str)>,
{ Vec::new() }
pub fn truncate_transcript_preview(_preview: &str, _max_lines: usize) -> String { String::new() }

#[derive(Debug, Clone)]
pub struct CopyTarget;
#[derive(Debug, Clone)]
pub struct EditToolRange;
#[derive(Debug, Clone)]
pub struct ImageRegion;
#[derive(Debug, Clone)]
pub struct PreparedChatFrame;
#[derive(Debug, Clone)]
pub struct PreparedMessages;
#[derive(Debug, Clone)]
pub struct PreparedSection;
#[derive(Debug, Clone)]
pub enum PreparedSectionKind { Unknown }

#[derive(Debug, Clone)]
pub struct WrappedLineMap;
