#[derive(Clone, Debug, Default)]
pub struct SihSupportContext {
    pub embedding: bool,
    pub decision_history: bool,
    pub hardware_collection: bool,
}

impl SihSupportContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_embedding(mut self, enabled: bool) -> Self {
        self.embedding = enabled;
        self
    }

    pub fn with_decision_history(mut self, enabled: bool) -> Self {
        self.decision_history = enabled;
        self
    }

    pub fn with_hardware_collection(mut self, enabled: bool) -> Self {
        self.hardware_collection = enabled;
        self
    }

    pub fn is_empty(&self) -> bool {
        !self.embedding && !self.decision_history && !self.hardware_collection
    }

    pub fn enable_all(&mut self) {
        self.embedding = true;
        self.decision_history = true;
        self.hardware_collection = true;
    }
}
