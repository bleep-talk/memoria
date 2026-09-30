use batch_recovery_decisions::{construct_memory_values, MemoryError, MemoryPath, Policy};

/// Parse a managed memory path before any repository effect.
pub fn parse_request(
    raw: &str,
    policy: &Policy,
    mutation: bool,
) -> Result<MemoryPath, MemoryError> {
    construct_memory_values(raw, policy, mutation)
}
