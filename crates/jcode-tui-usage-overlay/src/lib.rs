use ftui_style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UsageOverlayStatus {
    Loading,
    Good,
    Warning,
    Critical,
    Error,
    Info,
}

impl UsageOverlayStatus {
    pub fn label_for_display(self) -> &'static str {
        self.label()
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Loading => "loading",
            Self::Good => "healthy",
            Self::Warning => "watch",
            Self::Critical => "high",
            Self::Error => "error",
            Self::Info => "info",
        }
    }

    /// Source-of-truth RGB triplet for each status. [`Self::color`] delegates
    /// here so the constants stay in one place.
    pub const fn rgb(self) -> (u8, u8, u8) {
        match self {
            Self::Loading => (129, 184, 255),
            Self::Good => (111, 214, 181),
            Self::Warning => (255, 196, 112),
            Self::Critical => (255, 146, 110),
            Self::Error => (232, 134, 134),
            Self::Info => (196, 170, 255),
        }
    }

    /// Status color as a frankentui `ftui_style::Color`. Consumers that still
    /// drive ratatui rendering should convert at the call site via
    /// `jcode::tui::ftui_compat::ftui_color_to_rata` (or any other one-line
    /// shim) — see branch experimental/ratatui-to-frankentui.
    pub fn color(self) -> Color {
        let (r, g, b) = self.rgb();
        Color::rgb(r, g, b)
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Loading => "◌",
            Self::Good => "●",
            Self::Warning => "▲",
            Self::Critical => "◆",
            Self::Error => "✕",
            Self::Info => "○",
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UsageOverlayItem {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub status: UsageOverlayStatus,
    pub detail_lines: Vec<String>,
}

impl UsageOverlayItem {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        subtitle: impl Into<String>,
        status: UsageOverlayStatus,
        detail_lines: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            subtitle: subtitle.into(),
            status,
            detail_lines,
        }
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UsageOverlaySummary {
    pub provider_count: usize,
    pub warning_count: usize,
    pub critical_count: usize,
    pub error_count: usize,
    pub session_visible: bool,
}

pub fn item_matches_filter(item: &UsageOverlayItem, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }

    let haystack = format!(
        "{} {} {} {} {}",
        item.id,
        item.title,
        item.subtitle,
        item.status.label(),
        item.detail_lines.join(" ")
    )
    .to_lowercase();

    filter
        .split_whitespace()
        .all(|needle| haystack.contains(&needle.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_labels_match_display_copy() {
        assert_eq!(UsageOverlayStatus::Good.label_for_display(), "healthy");
        assert_eq!(UsageOverlayStatus::Critical.icon(), "◆");
    }

    #[test]
    fn item_filter_searches_details_and_status() {
        let item = UsageOverlayItem::new(
            "claude",
            "Claude usage",
            "85% used",
            UsageOverlayStatus::Warning,
            vec!["resets tomorrow".to_string()],
        );
        assert!(item_matches_filter(&item, "watch tomorrow"));
        assert!(item_matches_filter(&item, "claude 85"));
        assert!(!item_matches_filter(&item, "openai"));
    }

    #[test]
    fn rgb_values_are_stable() {
        assert_eq!(UsageOverlayStatus::Loading.rgb(), (129, 184, 255));
        assert_eq!(UsageOverlayStatus::Good.rgb(), (111, 214, 181));
        assert_eq!(UsageOverlayStatus::Warning.rgb(), (255, 196, 112));
        assert_eq!(UsageOverlayStatus::Critical.rgb(), (255, 146, 110));
        assert_eq!(UsageOverlayStatus::Error.rgb(), (232, 134, 134));
        assert_eq!(UsageOverlayStatus::Info.rgb(), (196, 170, 255));
    }

    #[test]
    fn color_round_trips_through_rgb_via_ftui_to_rgb() {
        for status in [
            UsageOverlayStatus::Loading,
            UsageOverlayStatus::Good,
            UsageOverlayStatus::Warning,
            UsageOverlayStatus::Critical,
            UsageOverlayStatus::Error,
            UsageOverlayStatus::Info,
        ] {
            let (r, g, b) = status.rgb();
            let c = status.color();
            // ftui_style::Color::to_rgb() always normalizes to its Rgb wrapper.
            let rgb = c.to_rgb();
            assert_eq!((rgb.r, rgb.g, rgb.b), (r, g, b));
        }
    }
}
