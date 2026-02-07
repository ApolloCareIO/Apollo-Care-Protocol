// programs/apollo_claims/src/instructions/mod.rs

pub mod ai_processing;
pub mod attestation;
pub mod fast_lane;
pub mod initialize;
pub mod oracle;
pub mod resolution;
pub mod submission;

pub use attestation::*;
pub use initialize::*;
pub use resolution::*;
pub use submission::*;
// Note: fast_lane and oracle exports removed to avoid name collision with ai_processing
// Use module:: prefix to access specific types (fast_lane::, oracle::)
pub use ai_processing::*;
