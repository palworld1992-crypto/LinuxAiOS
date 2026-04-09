/// Interface for components that can be managed by a supervisor
pub trait LocalManager: Send + Sync {
    /// Get current system potential (0.0-1.0)
    fn get_potential(&self) -> f32;

    /// Get current component state
    fn get_state(&self) -> &dyn std::any::Any;

    /// Check if component is in degraded mode
    fn is_degraded(&self) -> bool;

    /// Enter degraded mode (limited functionality)
    fn enter_degraded_mode(&mut self);

    /// Exit degraded mode (restore full functionality)
    fn exit_degraded_mode(&mut self);

    /// Get hardware collector (if applicable)
    fn get_hardware_collector(&self) -> Option<&dyn std::any::Any>;

    /// Get AI assistant (if applicable)
    fn get_assistant(&self) -> Option<&dyn std::any::Any>;

    /// Get API gateway (if applicable)
    fn get_api_gateway(&self) -> Option<&dyn std::any::Any>;

    /// Get state cache (if applicable)
    fn get_state_cache(&self) -> Option<&dyn std::any::Any>;

    /// Get knowledge base (if applicable)
    fn get_knowledge_base(&self) -> Option<&dyn std::any::Any>;

    /// Get decision history (if applicable)
    fn get_decision_history(&self) -> Option<&dyn std::any::Any>;
}
