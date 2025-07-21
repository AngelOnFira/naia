//! Hierarchical test suite for the new sync engine.
//! Each sub-module corresponds to a step in the test-driven refactor plan
//! described in `REFACTOR_PLAN.md`.

#![cfg(test)]

// mod seq; // redundant; logic tested elsewhere
mod stream;
mod engine_spawn;
mod component;
