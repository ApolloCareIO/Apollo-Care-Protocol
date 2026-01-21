// programs/apollo_staking/src/instructions/mod.rs

pub mod initialize;
pub mod staking;
pub mod rewards;
pub mod slashing;

pub use initialize::*;
pub use staking::*;
pub use rewards::*;
pub use slashing::*;
