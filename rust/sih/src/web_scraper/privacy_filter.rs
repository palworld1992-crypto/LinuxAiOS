//! Privacy Filter - Xóa thông tin nhạy cảm khỏi log và bộ nhớ

use regex::Regex;
use tracing::debug;

pub struct ScraperPrivacyFilter {
    patterns: Vec<(Regex, &'static str)>,
}

impl ScraperPrivacyFilter {
    pub fn new() -> Self {
        let patterns = vec![
            (
                Regex::new(r"\b[\w.-]+@[\w.-]+\.\w+\b")
                    .unwrap_or_else(|_| panic!("Invalid regex: email pattern")),
                "[EMAIL]",
            ),
            (
                Regex::new(r"\b\d{10,}\b")
                    .unwrap_or_else(|_| panic!("Invalid regex: phone pattern")),
                "[PHONE]",
            ),
            (
                Regex::new(r"(?i)(api[_-]?key|secret|token|password)[=:]\s*\S+")
                    .unwrap_or_else(|_| panic!("Invalid regex: API key pattern")),
                "[API_KEY]",
            ),
            (
                Regex::new(r"\b\d{3}-\d{2}-\d{4}\b")
                    .unwrap_or_else(|_| panic!("Invalid regex: SSN pattern")),
                "[SSN]",
            ),
            (
                Regex::new(r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b")
                    .unwrap_or_else(|_| panic!("Invalid regex: card pattern")),
                "[CARD]",
            ),
        ];

        Self { patterns }
    }

    pub fn filter(&self, text: &str) -> String {
        let mut filtered = text.to_string();

        for (pattern, replacement) in &self.patterns {
            filtered = pattern.replace_all(&filtered, *replacement).to_string();
        }

        debug!("Filtered {} sensitive patterns", self.patterns.len());
        filtered
    }

    pub fn filter_logs(&self, text: &str) -> String {
        self.filter(text)
    }
}

impl Default for ScraperPrivacyFilter {
    fn default() -> Self {
        Self::new()
    }
}
