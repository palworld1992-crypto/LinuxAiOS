use crate::errors::LnnPredictorError;
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct SihLnnPredictor {
    query_buffer: Arc<DashMap<String, QueryLog>>,
    _prediction_horizon: usize,
    model_loaded: AtomicBool,
}

#[derive(Clone, Debug)]
pub struct QueryLog {
    pub query: String,
    pub timestamp: i64,
    pub frequency: u32,
}

impl SihLnnPredictor {
    pub fn new(buffer_size: usize, prediction_horizon: usize) -> Self {
        Self {
            query_buffer: Arc::new(DashMap::new()),
            _prediction_horizon: prediction_horizon,
            model_loaded: AtomicBool::new(false),
        }
    }

    pub fn load_model(&mut self, _path: &str) -> Result<(), LnnPredictorError> {
        self.model_loaded.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        self.model_loaded.load(Ordering::Relaxed)
    }

    pub fn record_query(&self, query: String) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64);

        if let Some(mut entry) = self.query_buffer.get_mut(&query) {
            entry.frequency += 1;
            entry.timestamp = timestamp;
        } else {
            self.query_buffer.insert(
                query.clone(),
                QueryLog {
                    query,
                    timestamp,
                    frequency: 1,
                },
            );
        }
    }

    pub fn predict_next(&self) -> Vec<QueryLog> {
        let mut predictions: Vec<QueryLog> = self.query_buffer.iter().map(|r| r.clone()).collect();

        predictions.sort_by(|a, b| b.frequency.cmp(&a.frequency));
        predictions.truncate(5);

        predictions
    }

    pub fn get_top_queries(&self, limit: usize) -> Vec<QueryLog> {
        let mut queries: Vec<QueryLog> = self.query_buffer.iter().map(|r| r.clone()).collect();

        queries.sort_by(|a, b| b.frequency.cmp(&a.frequency));
        queries.truncate(limit);

        queries
    }
}
