//! XML delegator scanner — wraps Anthropic or DeepSeek scanner based on xml_tagset.
//!
//! The `XmlInbandScanner` delegates to:
//! - `AnthropicInbandScanner` when `xml_tagset` is `None` or `"anthropic"`
//! - `DeepSeekInbandScanner` when `xml_tagset` is `"dsml"`

use crate::anthropic::AnthropicInbandScanner;
use crate::deepseek::DeepSeekInbandScanner;
use crate::types::{InbandScanEvent, InbandScanner, InbandScannerOptions};

/// A delegating scanner that routes to the appropriate XML-based dialect scanner.
///
/// The tagset is determined by `InbandScannerOptions::xml_tagset`:
/// - `None` or `"anthropic"` → `AnthropicInbandScanner`
/// - `"dsml"` → `DeepSeekInbandScanner`
pub struct XmlInbandScanner {
    inner: Box<dyn InbandScanner>,
}

impl XmlInbandScanner {
    pub fn new(options: &InbandScannerOptions) -> Self {
        let inner: Box<dyn InbandScanner> = match options.xml_tagset.as_deref() {
            Some("dsml") => Box::new(DeepSeekInbandScanner::new(options)),
            _ => Box::new(AnthropicInbandScanner::new(options)),
        };
        Self { inner }
    }
}

impl InbandScanner for XmlInbandScanner {
    fn feed(&mut self, text: &str) -> Vec<InbandScanEvent> {
        self.inner.feed(text)
    }

    fn flush(&mut self) -> Vec<InbandScanEvent> {
        self.inner.flush()
    }
}
