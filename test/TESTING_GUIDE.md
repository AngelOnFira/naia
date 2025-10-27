# Naia Testing Guide

## Overview

This guide explains the testing strategy for Naia, how to run tests, and how to write new tests. Our testing approach focuses on **quality over quantity** - catching real bugs through integration and regression tests rather than just achieving high line coverage.

## Test Categories

### 1. Unit Tests (Component Level)
**Location**: `shared/src/world/sync/tests/`

Test individual components in isolation:
- Entity channels
- Authority state machines
- Migration operations
- Serialization/deserialization

**Run**: `cargo test --package naia-shared --lib`

### 2. Regression Tests
**Location**: `test/tests/regression_bug_*.rs`

One test file per production bug. These tests reproduce the exact conditions that caused production failures.

**Purpose**: Prevent bugs from returning
**Run**: `cargo test --package naia-test`

### 3. Integration Tests
**Location**: `test/tests/integration_*.rs`

Test multiple components working together at the **LocalWorldManager level**, focusing on critical paths like delegation, authority management, and migration flows.

**Purpose**: Verify multiple components stay synchronized (addresses Gap #1, #2, #4 from TEST_COVERAGE_GAPS_AND_FIXES.md)

**Run**: `cargo test --package naia-test --test integration_`

#### Integration Test Suites

1. **LocalWorldManager Integration** (`integration_local_world_manager.rs`)
   - Tests complete authority lifecycle through LocalWorldManager
   - Verifies RemoteEntityChannel state machine correctness after migration
   - Tests multiple authority request/release cycles
   - **Critical Test**: `authority_lifecycle_through_local_world_manager` - Would have caught Bug #7

2. **Migration Flow Tests** (`integration_migration_flow.rs`)
   - Tests component state preservation during migration
   - Verifies entity redirects are correctly installed
   - Tests multiple entities migrating simultaneously
   - Verifies migrated entities start in correct delegated state

3. **Serialization Round-Trips** (`integration_serialization.rs`)
   - Tests that MigrateResponse preserves entity IDs (Bug #5 regression)
   - Tests EntityCommand variants preserve entity types
   - Tests entity redirects work with serialization
   - **Gap #4**: Tests entity references survive serialization/deserialization

4. **Authority Synchronization** (`integration_authority_sync.rs`)
   - Tests authority status stays synced through state transitions
   - Tests authority sync is maintained during migration
   - Tests multiple authority cycles don't cause drift
   - **Gap #2**: Verifies GlobalWorldManager and RemoteEntityChannel stay in sync

### 4. Property-Based Tests
**Location**: `test/tests/property_*.rs`

Use `proptest` to verify invariants hold across random inputs.

**Run**: `cargo test --package naia-test property`

---

## Running Tests

### All Tests
```bash
cargo test --workspace
```

### Specific Package
```bash
cargo test --package naia-shared --lib
cargo test --package naia-test
```

### Specific Test
```bash
cargo test --package naia-test bug_07_authority_request_after_release_cycle
```

### With Output
```bash
cargo test --package naia-test -- --nocapture
```

### Coverage (requires tarpaulin)
```bash
./scripts/test_coverage.sh
```

---

## Writing Regression Tests

Every production bug should get a regression test. Follow this template:

```rust
/// REGRESSION TEST FOR BUG #X: [Brief description]
/// 
/// THE BUG: [What went wrong]
///
/// ROOT CAUSE: [Why it happened]
///
/// THE SYMPTOM: [How it manifested]
///
/// This test would have caught the bug if it existed before production.

#[test]
fn bug_0X_descriptive_name() {
    // Setup: Create the conditions that triggered the bug
    
    // Action: Perform the operation that caused the panic/error
    
    // Assert: Verify it works now (would have panicked before fix)
    assert!(result.is_ok(), "BUG #X: [explanation]");
}
```

### Key Principles

1. **Reproduce the exact conditions** - Use the same sequence of operations that caused the production bug
2. **Test would have failed before fix** - Verify the test catches the bug
3. **Document thoroughly** - Future developers should understand what broke and why
4. **Keep it focused** - One test per bug, testing the specific failure mode

---

## Testing Lessons Learned

### From 7 Production Bugs

**All 7 bugs shared a common pattern:**
- ✅ Unit tests passed
- ❌ Integration failed

The bugs lived in the **gaps between components** where they interact.

### What Our Tests Missed

1. **Bug #7 (Authority Mismatch)**
   - Tests checked global authority tracker
   - Tests DID NOT check RemoteEntityChannel's internal AuthChannel
   - **Lesson**: Test internal state consistency, not just external APIs

2. **Bug #6 (EntityProperty)**
   - Tests covered `new_read()` code path
   - Tests DID NOT cover `waiting_complete()` code path
   - **Lesson**: Test ALL code paths that reach same outcome

3. **Bugs #1-5 (Command Validation, Sequencing)**
   - Tests used simplified scenarios
   - Tests DID NOT exercise actual server delegation flow
   - **Lesson**: Test realistic sequences, not just happy paths

### Test Coverage Guidelines

**High Priority** (>80% coverage target):
- Migration logic (`shared/src/world/local/local_world_manager.rs`)
- Authority management (`client/src/client.rs` MigrateResponse handler)
- EntityProperty (`shared/src/world/component/entity_property.rs`)
- Command validation (`shared/src/world/sync/auth_channel.rs`)

**Medium Priority** (>60% coverage target):
- Entity channels
- Serialization
- State machines

**Lower Priority**:
- Adapters (tested by integration)
- Demos (manual testing)

---

## Test Infrastructure

### Available Test Helpers

```rust
use naia_test::helpers::*;

// For component-level testing
let mut entity_map = LocalEntityMap::new(HostType::Client);
let converter = entity_map.entity_converter();

// For testing with redirects
entity_map.install_entity_redirect(old_entity, new_entity);

// For testing authority
let mut channel = RemoteEntityChannel::new_delegated(HostType::Client);
channel.update_auth_status(EntityAuthStatus::Granted);
```

### Limitations

**E2E Testing Constraints:**
- Full Client/Server E2E tests require a World implementation
- World implementations are game-specific (Bevy, Hecs, custom)
- Test infrastructure focuses on component-level testing instead

**Workaround:**
- Test at the `shared` level (world managers, channels, converters)
- Use integration tests in game projects for E2E validation
- Focus regression tests on specific bug reproduction

---

## Adding a New Regression Test

When a production bug is found:

1. **Create test file**: `test/tests/regression_bug_0X_short_name.rs`

2. **Document the bug**:
```rust
/// REGRESSION TEST FOR BUG #X: [title]
/// THE BUG: [what broke]
/// ROOT CAUSE: [why]
/// THE SYMPTOM: [how it manifested]
/// This test would have caught the bug if it existed before production.
```

3. **Write the test**:
```rust
#[test]
fn bug_0X_main_test() {
    // Reproduce exact conditions
    // Perform operation
    // Assert it works (would have panicked before)
}
```

4. **Verify the test**:
   - Temporarily undo the bug fix
   - Confirm test fails
   - Re-apply fix
   - Confirm test passes

5. **Update tracking**:
   - Add to `TEST_COVERAGE_GAPS_AND_FIXES.md`
   - Note in PR description

---

## Property-Based Testing

Use `proptest` for state machine invariants:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn authority_state_machine_invariants(
        operations in prop::collection::vec(auth_operation_strategy(), 0..100)
    ) {
        // Verify invariants hold across random operation sequences
        // 1. Only one client has Granted at a time
        // 2. State transitions follow valid paths
        // 3. No state corruption between cycles
    }
}
```

---

## Coverage Reports

### Generate Coverage
```bash
./scripts/test_coverage.sh
```

This creates `coverage/index.html` with:
- Line coverage by file
- Uncovered lines highlighted
- Branch coverage statistics

### Interpreting Results

**Good coverage (>80%)**:
- Critical paths (migration, delegation, authority)
- Bug-prone areas (serialization, state machines)

**Acceptable coverage (60-80%)**:
- Supporting infrastructure
- Less critical paths

**Low priority (<60%)**:
- Demo code
- Adapters (tested via integration)
- Error handling for impossible states

---

## Best Practices

### DO

✅ Write regression tests for every production bug
✅ Test internal state consistency, not just external APIs
✅ Test ALL code paths, especially alternate ones
✅ Use descriptive test names explaining what's being tested
✅ Document WHY the test exists (link to bug report)
✅ Test realistic sequences, not just isolated operations
✅ Verify tests would fail without the fix

### DON'T

❌ Test for coverage percentage alone
❌ Mock/fake critical components (use real ones)
❌ Test only the happy path
❌ Assume "tests pass" means "production ready"
❌ Skip documenting test purpose
❌ Write tests that can't fail
❌ Ignore integration test failures

---

## Questions?

**Test failures:** Check if recent changes broke existing functionality
**Coverage questions:** Focus on critical paths first
**New test patterns:** Follow existing regression test templates
**Integration testing:** See examples in `shared/src/world/sync/tests/`

---

## Summary

**Testing Philosophy:**
- **Quality > Quantity**: 10 good integration tests > 100 shallow unit tests
- **Regression-Focused**: Every bug gets a test
- **State Verification**: Check internal consistency, not just APIs
- **Path Coverage**: Test ALL ways to reach an outcome
- **Real Scenarios**: Use actual operation sequences, not simplified ones

**Success Criteria:**
- All 7 production bugs have regression tests
- Critical paths have >80% coverage
- New bugs caught by tests before production
- Zero tolerance for bugs tests should catch

