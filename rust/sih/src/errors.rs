#[derive(Debug, thiserror::Error)]
pub enum HardwareCollectorError {
    #[error("Collector is not running")]
    NotRunning,
    #[error("Collection failed: {0}")]
    CollectionFailed(String),
    #[error("Collector error: {0}")]
    Collector(#[from] crate::hardware::collector::CollectorError),
}

#[derive(Debug, thiserror::Error)]
pub enum KnowledgeBaseError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Entry not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid dimension")]
    InvalidDimension,
}

#[derive(Debug, thiserror::Error)]
pub enum DecisionHistoryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum RecommenderError {
    #[error("Model not loaded")]
    ModelNotLoaded,
    #[error("Inference failed: {0}")]
    InferenceFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("Model not loaded")]
    ModelNotLoaded,
    #[error("Encoding failed: {0}")]
    EncodingFailed(String),
    #[error("Model load failed: {0}")]
    LoadFailed(String),
    #[error("Inference failed: {0}")]
    InferenceFailed(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ModelManagerError {
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum LnnPredictorError {
    #[error("Model not loaded")]
    ModelNotLoaded,
    #[error("Prediction failed: {0}")]
    PredictionFailed(String),
}

#[derive(Debug, thiserror::Error)]
pub enum RlPolicyError {
    #[error("Policy not loaded")]
    PolicyNotLoaded,
    #[error("Evaluation failed: {0}")]
    EvaluationFailed(String),
    #[error("IO error: {0}")]
    Io(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SourceFetcherError {
    #[error("Source not found: {0}")]
    SourceNotFound(String),
    #[error("Source disabled: {0}")]
    SourceDisabled(String),
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("RPC error: {0}")]
    RpcError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum TrustScoringError {
    #[error("Model error: {0}")]
    ModelError(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ContentValidatorError {
    #[error("Model error: {0}")]
    ModelError(String),
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, thiserror::Error)]
pub enum PrivacyFilterError {
    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),
    #[error("Filter error: {0}")]
    FilterError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum KnowledgeIngestionError {
    #[error("Source fetch error: {0}")]
    FetchError(String),
    #[error("Content is toxic")]
    ContentToxic,
    #[error("Trust score too low: {0}")]
    TrustScoreTooLow(f32),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ApiGatewayError {
    #[error("Gateway error: {0}")]
    GatewayError(String),
    #[error("Port already in use")]
    PortInUse,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token expired")]
    TokenExpired,
    #[error("Authentication disabled")]
    AuthDisabled,
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, thiserror::Error)]
pub enum StateCacheError {
    #[error("Cache error: {0}")]
    CacheError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SihMainError {
    #[error("Initialization failed: {0}")]
    InitFailed(String),
    #[error("API error: {0}")]
    ApiError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SihLocalFailoverError {
    #[error("Failover error: {0}")]
    FailoverError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SihDegradedModeError {
    #[error("Degraded mode error: {0}")]
    Error(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SihSupportError {
    #[error("Support not active")]
    NotActive,
    #[error("Task error: {0}")]
    TaskError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SihSupervisorError {
    #[error("Supervisor not active")]
    NotActive,
    #[error("Health check failed")]
    HealthCheckFailed,
}

#[derive(Debug, thiserror::Error)]
pub enum SihAssistantError {
    #[error("Initialization failed: {0}")]
    InitFailed(String),
}
