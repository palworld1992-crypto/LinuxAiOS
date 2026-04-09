use crate::PrivacyFilterError;
use dashmap::DashMap;
use regex::Regex;
use std::sync::Arc;

pub struct PrivacyFilter {
    patterns: Arc<DashMap<String, PrivacyPattern>>,
    enabled: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Clone, Debug)]
struct PrivacyPattern {
    name: String,
    regex: Regex,
    replacement: String,
}

impl PrivacyFilter {
    pub fn new() -> Result<Self, PrivacyFilterError> {
        let patterns = vec![
            PrivacyPattern {
                name: "email".to_string(),
                regex: Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")
                    .map_err(|e| PrivacyFilterError::InvalidPattern(e.to_string()))?,
                replacement: "[EMAIL]".to_string(),
            },
            PrivacyPattern {
                name: "phone".to_string(),
                regex: Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b")
                    .map_err(|e| PrivacyFilterError::InvalidPattern(e.to_string()))?,
                replacement: "[PHONE]".to_string(),
            },
            PrivacyPattern {
                name: "ssn".to_string(),
                regex: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b")
                    .map_err(|e| PrivacyFilterError::InvalidPattern(e.to_string()))?,
                replacement: "[SSN]".to_string(),
            },
            PrivacyPattern {
                name: "api_key".to_string(),
                regex: Regex::new(r"(?i)(api[_-]?key|secret[_-]?key|access[_-]?token)")
                    .map_err(|e| PrivacyFilterError::InvalidPattern(e.to_string()))?,
                replacement: "$1=[REDACTED]".to_string(),
            },
            PrivacyPattern {
                name: "password".to_string(),
                regex: Regex::new(r"(?i)(password|passwd|pwd)")
                    .map_err(|e| PrivacyFilterError::InvalidPattern(e.to_string()))?,
                replacement: "$1=[REDACTED]".to_string(),
            },
        ];

        Ok(Self {
            patterns: Arc::new(DashMap::new()),
            enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        })
    }

    pub fn filter(&self, content: &str) -> Result<String, PrivacyFilterError> {
        if !self.enabled.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(content.to_string());
        }

        let mut result = content.to_string();

        for pattern in self.patterns.iter() {
            result = pattern
                .regex
                .replace_all(&result, pattern.replacement.as_str())
                .to_string();
        }

        Ok(result)
    }

    pub fn add_pattern(
        &self,
        name: &str,
        pattern: &str,
        replacement: &str,
    ) -> Result<(), PrivacyFilterError> {
        let regex =
            Regex::new(pattern).map_err(|e| PrivacyFilterError::InvalidPattern(e.to_string()))?;

        self.patterns.insert(
            name.to_string(),
            PrivacyPattern {
                name: name.to_string(),
                regex,
                replacement: replacement.to_string(),
            },
        );

        Ok(())
    }

    pub fn remove_pattern(&self, name: &str) -> Result<(), PrivacyFilterError> {
        self.patterns.remove(name);
        Ok(())
    }

    pub fn enable(&self) {
        self.enabled
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn disable(&self) {
        self.enabled
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(std::sync::atomic::Ordering::SeqCst)
    }
}
