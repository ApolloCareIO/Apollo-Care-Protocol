// programs/apollo_staking/src/instructions/mod.rs

pub mod initialize;
pub mod rewards;
pub mod slashing;
pub mod staking;

pub use initialize::*;
pub use rewards::*;
pub use slashing::*;
pub use staking::*;
