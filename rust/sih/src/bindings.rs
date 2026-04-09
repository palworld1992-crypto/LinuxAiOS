use dashmap::DashMap;
use pyo3::prelude::*;
use std::sync::Arc;

use crate::dkss::DKSSCoordinator;
use crate::hardware::SihHardwareCollector;
use crate::knowledge::KnowledgeBase;

static HARDWARE_COLLECTOR: DashMap<u64, SihHardwareCollector> = DashMap::new();
static KNOWLEDGE_BASE: DashMap<u64, KnowledgeBase> = DashMap::new();
static DKSS_COORDINATOR: DashMap<u64, DKSSCoordinator> = DashMap::new();

pub fn init_hardware_collector(collector: Arc<SihHardwareCollector>) {
    HARDWARE_COLLECTOR.insert(0, (*collector).clone());
}

pub fn init_knowledge_base(kb: Arc<KnowledgeBase>) {
    KNOWLEDGE_BASE.insert(0, (*kb).clone());
}

pub fn init_dkss_coordinator(coordinator: Arc<DKSSCoordinator>) {
    DKSS_COORDINATOR.insert(0, (*coordinator).clone());
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyHardwareMetrics {
    #[pyo3(get)]
    pub timestamp: u64,
    #[pyo3(get)]
    pub cpu_usage: f32,
    #[pyo3(get)]
    pub memory_usage: f32,
    #[pyo3(get)]
    pub disk_usage: f32,
    #[pyo3(get)]
    pub network_rx: u64,
    #[pyo3(get)]
    pub network_tx: u64,
    #[pyo3(get)]
    pub temperature: f32,
}

#[pymethods]
impl PyHardwareMetrics {
    #[new]
    pub fn new(
        timestamp: u64,
        cpu_usage: f32,
        memory_usage: f32,
        disk_usage: f32,
        network_rx: u64,
        network_tx: u64,
        temperature: f32,
    ) -> Self {
        Self {
            timestamp,
            cpu_usage,
            memory_usage,
            disk_usage,
            network_rx,
            network_tx,
            temperature,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyKnowledgeEntry {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub content: String,
    #[pyo3(get)]
    pub embedding: Option<Vec<f32>>,
    #[pyo3(get)]
    pub source: String,
    #[pyo3(get)]
    pub trust_score: f32,
    #[pyo3(get)]
    pub created_at: i64,
    #[pyo3(get)]
    pub updated_at: i64,
    #[pyo3(get)]
    pub tags: Vec<String>,
}

#[pymethods]
impl PyKnowledgeEntry {
    #[new]
    pub fn new(
        id: String,
        content: String,
        source: String,
        trust_score: f32,
        created_at: i64,
        updated_at: i64,
        tags: Vec<String>,
        embedding: Option<Vec<f32>>,
    ) -> Self {
        Self {
            id,
            content,
            embedding,
            source,
            trust_score,
            created_at,
            updated_at,
            tags,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyProposalMetadata {
    #[pyo3(get)]
    pub proposal_id: String,
    #[pyo3(get)]
    pub proposal_type: String,
    #[pyo3(get)]
    pub timestamp: u64,
    #[pyo3(get)]
    pub status: String,
    #[pyo3(get)]
    pub trust_score: f32,
}

#[pymethods]
impl PyProposalMetadata {
    #[new]
    pub fn new(
        proposal_id: String,
        proposal_type: String,
        timestamp: u64,
        status: String,
        trust_score: f32,
    ) -> Self {
        Self {
            proposal_id,
            proposal_type,
            timestamp,
            status,
            trust_score,
        }
    }
}

#[pyfunction]
pub fn get_metrics_batch(
    py: Python,
    start_timestamp: u64,
    end_timestamp: u64,
) -> PyResult<Vec<PyHardwareMetrics>> {
    // TODO(Phase 7): Integrate with HardwareCollector via global state
    // Phase 7 will implement actual metric retrieval
    match HARDWARE_COLLECTOR.get(&0) {
        Some(collector) => {
            let metrics = collector
                .get_metrics_in_range(start_timestamp, end_timestamp)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(metrics
                .into_iter()
                .map(|m| PyHardwareMetrics {
                    timestamp: m.timestamp,
                    cpu_usage: m.cpu_usage,
                    memory_usage: m.memory_usage,
                    disk_usage: m.disk_usage,
                    network_rx: m.network_rx,
                    network_tx: m.network_tx,
                    temperature: m.temperature,
                })
                .collect())
        }
        None => {
            // Return empty if not initialized yet
            Ok(vec![])
        }
    }
}

#[pyfunction]
pub fn query_knowledge(py: Python, query: &str, top_k: usize) -> PyResult<Vec<PyKnowledgeEntry>> {
    // TODO(Phase 7): Integrate with KnowledgeBase via global state
    match KNOWLEDGE_BASE.get(&0) {
        Some(kb) => {
            let entries = kb
                .query(query, top_k)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(entries
                .into_iter()
                .map(|e| PyKnowledgeEntry {
                    id: e.id,
                    content: e.content,
                    embedding: e.embedding,
                    source: e.source,
                    trust_score: e.trust_score,
                    created_at: e.created_at,
                    updated_at: e.updated_at,
                    tags: e.tags,
                })
                .collect())
        }
        None => Ok(vec![]),
    }
}

#[pyfunction]
pub fn evaluate_proposal(py: Python, proposal_json: &str) -> PyResult<f32> {
    // TODO(Phase 7): Integrate with DKSSCoordinator for proposal evaluation
    match DKSS_COORDINATOR.get(&0) {
        Some(coordinator) => coordinator
            .evaluate_proposal_json(proposal_json)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string())),
        None => Ok(0.5),
    }
}

#[pymodule]
fn sih_bindings(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyHardwareMetrics>()?;
    m.add_class::<PyKnowledgeEntry>()?;
    m.add_class::<PyProposalMetadata>()?;
    m.add_function(wrap_pyfunction!(get_metrics_batch, m)?)?;
    m.add_function(wrap_pyfunction!(query_knowledge, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_proposal, m)?)?;
    Ok(())
}
