//! Hierarchical test suite for the new sync engine.
//! Each sub-module corresponds to a step in the test-driven refactor plan
//! described in `REFACTOR_PLAN.md`.

#![cfg(test)]

mod engine;
mod migration;
mod bulletproof_migration;
mod integration_migration;
