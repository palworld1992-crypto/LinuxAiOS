//! Query Generator - Sinh câu hỏi thông minh dựa trên knowledge gaps

use crate::knowledge::KnowledgeBase;
use crate::web_scraper::Platform;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::debug;

pub struct QueryGenerator {
    knowledge_base: Arc<KnowledgeBase>,
    known_topics: DashMap<String, bool>,
}

impl QueryGenerator {
    pub fn new(knowledge_base: Arc<KnowledgeBase>) -> Self {
        Self {
            knowledge_base,
            known_topics: DashMap::new(),
        }
    }

    pub fn generate_queries(
        &self,
        platform: Platform,
        topics: &[String],
        count: usize,
    ) -> Vec<String> {
        let mut queries = Vec::with_capacity(count);

        for topic in topics {
            if !self.is_known_topic(topic) {
                queries.push(format!("Explain {} in detail", topic));
                queries.push(format!("What are the best practices for {}?", topic));

                if queries.len() >= count {
                    break;
                }
            }
        }

        debug!("Generated {} queries for {:?}", queries.len(), platform);
        queries
    }

    fn is_known_topic(&self, topic: &str) -> bool {
        self.known_topics.contains_key(topic)
    }

    pub fn mark_topic_known(&self, topic: String) {
        self.known_topics.insert(topic, true);
    }
}
