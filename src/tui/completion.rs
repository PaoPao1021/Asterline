//! TUI completion is the shared contract completion. The catalog and matching
//! rules live in `crate::contract` so the desktop GUI offers identical
//! suggestions; this module only re-exports them for the TUI renderer.

pub use crate::contract::completion::{
    AgentSkill, Completion, CompletionItem, compute, compute_with_agent_skills,
};
