//! Response Parser - Trích xuất nội dung từ HTML/text

use crate::web_scraper::Platform;
use regex::Regex;
use tracing::debug;

#[derive(Clone, Debug)]
pub struct ExtractedData {
    pub text: String,
    pub code_snippets: Vec<String>,
    pub config_blocks: Vec<String>,
    pub trust_hint: f32,
}

pub struct ResponseParser {
    code_pattern: Regex,
    config_pattern: Regex,
}

impl ResponseParser {
    pub fn new() -> Self {
        Self {
            code_pattern: Regex::new(r"(?s)```[\w]*\n(.+?)```")
                .unwrap_or_else(|_| panic!("Invalid regex: code pattern")),
            config_pattern: Regex::new(r"(?s)(\{[\s\S]*?\})|(\[[\s\S]*?\])")
                .unwrap_or_else(|_| panic!("Invalid regex: config pattern")),
        }
    }

    pub fn parse(&self, response: &str, platform: Platform) -> ExtractedData {
        let text = self.extract_text(response);
        let code_snippets = self.extract_code(response);
        let config_blocks = self.extract_config(response);

        let trust_hint = self.calculate_trust_hint(&text, &code_snippets, &config_blocks);

        debug!(
            "Parsed response from {:?}: {} chars, {} code blocks",
            platform,
            text.len(),
            code_snippets.len()
        );

        ExtractedData {
            text,
            code_snippets,
            config_blocks,
            trust_hint,
        }
    }

    fn extract_text(&self, response: &str) -> String {
        let mut text = response.to_string();

        if let Some(captures) = self.code_pattern.captures(response) {
            if let Some(m) = captures.get(1) {
                text = text.replace(m.as_str(), "[CODE]");
            }
        }

        text.lines()
            .filter(|l| !l.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn extract_code(&self, response: &str) -> Vec<String> {
        self.code_pattern
            .captures_iter(response)
            .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
            .collect()
    }

    fn extract_config(&self, response: &str) -> Vec<String> {
        self.config_pattern
            .captures_iter(response)
            .filter_map(|c| c.get(0).map(|m| m.as_str().to_string()))
            .collect()
    }

    fn calculate_trust_hint(&self, text: &str, code: &[String], config: &[String]) -> f32 {
        let mut score: f32 = 0.5;

        if text.len() > 100 {
            score += 0.1;
        }

        if !code.is_empty() {
            score += 0.2;
        }

        if !config.is_empty() {
            score += 0.1;
        }

        score.min(1.0)
    }
}

impl Default for ResponseParser {
    fn default() -> Self {
        Self::new()
    }
}
