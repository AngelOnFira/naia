# Entity Migration: Development & Testing Strategy

**Status:** Ready for execution  
**Based on:** `MIGRATION_FEATURE_SPEC.md` and `MIGRATION_IMPLEMENTATION_PLAN.md`  
**Objective:** Define iron-clad, incremental development approach with continuous validation

---

## Table of Contents

1. [Testing Infrastructure Overview](#testing-infrastructure-overview)
2. [Test-Driven Development Philosophy](#test-driven-development-philosophy)
3. [Incremental Development Stages](#incremental-development-stages)
4. [Test Execution Commands](#test-execution-commands)
5. [Quality Gates & Validation](#quality-gates--validation)
6. [Development Workflow](#development-workflow)
7. [Rollback Strategy](#rollback-strategy)

---

## Testing Infrastructure Overview

### Test Organization

Naia uses a three-tiered test structure:

```
naia/
├── test/                           # Integration tests (cross-component)
│   ├── src/                        # Test helpers and shared types
│   │   ├── lib.rs
│   │   └── auth.rs                # Auth message for testing
│   └── tests/
│       └── handshake.rs           # End-to-end handshake test
│
├── shared/
│   ├── src/
│   │   └── world/sync/tests/      # Unit tests for sync engine
│   │       ├── mod.rs
│   │       └── engine.rs          # 32 comprehensive engine tests
│   └── tests/                     # Integration tests for shared
│       └── derive_*.rs            # Derive macro tests
│
└── <other_crates>/
    └── src/
        └── *.rs                    # Inline unit tests with #[cfg(test)]
```

### Existing Test Patterns

**1. Unit Tests (Engine Pattern)**
```rust
#[test]
fn test_name() {
    // Setup
    let mut engine = RemoteEngine::new(HostType::Server);
    let entity = RemoteEntity::new(1);
    
    // Execute
    engine.receive_message(1, EntityMessage::Spawn(entity));
    
    // Assert
    let events = engine.take_incoming_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], EntityMessage::Spawn(entity));
}
```

**2. Integration Tests (E2E Pattern)**
```rust
#[test]
fn end_to_end_workflow() {
    // Setup client and server
    let mut client = ClientHandshakeManager::new(...);
    let mut server = ServerHandshakeManager::new();
    
    // Simulate message exchange
    let bytes = client.write_request();
    let response = server.receive_request(&bytes);
    let final_state = client.receive_response(&response);
    
    // Assert end state
    assert_eq!(client.state, ExpectedState);
}
```

**3. Property-Based Tests (Guard-Band Pattern)**
```rust
#[test]
fn guard_band_flush() {
    let mut engine = RemoteEngine::new(HostType::Client);
    let near_flush_seq = engine.config.flush_threshold - 2;
    let wrap_beyond_seq = engine.config.flush_threshold + 1;
    
    engine.receive_message(near_flush_seq, msg1);
    engine.receive_message(wrap_beyond_seq, msg2);
    
    // Only later message should be processed
    assert_eq!(engine.take_incoming_events().len(), 1);
}
```

### Test Execution Commands

```bash
# Run all tests
cargo test

# Run specific package tests
cargo test --package naia-shared
cargo test --package naia-client
cargo test --package naia-server
cargo test --package naia-test

# Run specific test file
cargo test --package naia-shared --test engine

# Run tests matching pattern
cargo test migrate
cargo test redirect

# Run with output
cargo test -- --nocapture

# Run single test
cargo test --package naia-shared test_name -- --exact

# Check compilation without running tests
cargo check --all-targets

# Run tests in release mode (faster for heavy tests)
cargo test --release
```

---

## Test-Driven Development Philosophy

### Red-Green-Refactor Cycle

For each development stage:

1. **RED:** Write failing test that defines expected behavior
2. **GREEN:** Implement minimal code to make test pass
3. **REFACTOR:** Clean up while keeping tests green
4. **VALIDATE:** Run full test suite to ensure no regressions

### Test-First Benefits

1. **Design clarity:** Tests force you to think about API before implementation
2. **Confidence:** Green tests provide immediate feedback
3. **Regression protection:** Existing tests catch breaks
4. **Documentation:** Tests serve as executable specifications

### Testing Principles for Migration

1. **Test state transitions explicitly**
   - RemoteEntity exists → migration → HostEntity exists
   - Verify old entity no longer reachable

2. **Test data preservation**
   - Component state before/after
   - Buffered operations not lost

3. **Test edge cases**
   - Empty buffers
   - Full buffers
   - In-flight messages
   - Multiple concurrent entities

4. **Test failure modes**
   - Entity doesn't exist
   - Already migrated
   - Invalid state

---

## Incremental Development Stages

### Stage 0: Baseline (Current State)

**Objective:** Fix existing compilation errors, establish green baseline

**Tasks:**
1. Fix `remote_entity` typo in `local_world_manager.rs:134`
2. Add placeholder implementation for `remove_entity_channel` that panics
3. Ensure all existing tests pass

**Test Strategy:**
```bash
# Must pass before proceeding
cargo check
cargo test --package naia-shared --lib
cargo test --package naia-test
```

**Success Criteria:**
- [ ] `cargo check` passes with no errors
- [ ] All existing tests pass (may need HostType argument fixes in test code)
- [ ] No warnings in critical paths

**Time Estimate:** 30 minutes

---

### Stage 1: Core Channel Methods

**Objective:** Add extraction and inspection methods to entity channels

**Implementation Order:**
1. `RemoteComponentChannel::is_inserted()` + test
2. `RemoteEntityChannel::get_state()` + test
3. `RemoteEntityChannel::extract_inserted_component_kinds()` + test
4. `HostEntityChannel::new_with_components()` + test

**Test File:** `shared/src/world/sync/tests/migration.rs` (NEW)

**Test Template:**
```rust
#[cfg(test)]
mod migration_tests {
    use super::*;
    
    #[test]
    fn remote_component_channel_extract_inserted() {
        // Create RemoteEntityChannel with some components
        let mut channel = RemoteEntityChannel::new(HostType::Server);
        let comp1 = component_kind::<1>();
        let comp2 = component_kind::<2>();
        
        // Simulate spawn and component inserts
        channel.receive_message(1, EntityMessage::Spawn(...));
        channel.receive_message(2, EntityMessage::InsertComponent(..., comp1));
        channel.receive_message(3, EntityMessage::InsertComponent(..., comp2));
        
        // Extract
        let kinds = channel.extract_inserted_component_kinds();
        
        // Assert
        assert_eq!(kinds.len(), 2);
        assert!(kinds.contains(&comp1));
        assert!(kinds.contains(&comp2));
    }
    
    #[test]
    fn host_entity_channel_new_with_components() {
        let comp1 = component_kind::<1>();
        let comp2 = component_kind::<2>();
        let mut kinds = HashSet::new();
        kinds.insert(comp1);
        kinds.insert(comp2);
        
        let channel = HostEntityChannel::new_with_components(
            HostType::Server,
            kinds.clone()
        );
        
        assert_eq!(channel.component_kinds(), &kinds);
    }
}
```

**Test Execution:**
```bash
cargo test --package naia-shared migration
cargo check --package naia-shared
```

**Success Criteria:**
- [ ] All new methods compile
- [ ] New unit tests pass
- [ ] No regressions in existing tests
- [ ] Methods correctly expose internal state

**Time Estimate:** 2 hours

---

### Stage 2: Force-Drain Implementation

**Objective:** Implement buffer draining for RemoteEntityChannel

**Implementation Order:**
1. `RemoteComponentChannel::force_drain_buffers()` + test
2. `RemoteEntityChannel::force_drain_all_buffers()` + test
3. `RemoteEngine::get_world_mut()` + test
4. `RemoteWorldManager::force_drain_entity_buffers()` + test

**Test Cases:**
```rust
#[test]
fn force_drain_resolves_all_buffers() {
    let mut channel = RemoteEntityChannel::new(HostType::Client);
    let entity = RemoteEntity::new(1);
    let comp = component_kind::<1>();
    
    // Setup: spawn + buffer some out-of-order operations
    channel.receive_message(1, EntityMessage::Spawn(entity));
    channel.receive_message(4, EntityMessage::RemoveComponent(entity, comp));
    channel.receive_message(3, EntityMessage::InsertComponent(entity, comp));
    
    // Before drain: remove is buffered (can't remove non-existent)
    let events_before = channel.take_incoming_events();
    assert_eq!(events_before.len(), 1); // Only spawn
    
    // Force drain
    channel.force_drain_all_buffers();
    
    // After drain: all operations resolved
    let events_after = channel.take_incoming_events();
    assert_eq!(events_after.len(), 2); // Insert + Remove
    
    // Verify buffers empty
    let events_final = channel.take_incoming_events();
    assert_eq!(events_final.len(), 0);
}

#[test]
fn force_drain_preserves_component_state() {
    let mut channel = RemoteEntityChannel::new(HostType::Server);
    let comp = component_kind::<1>();
    
    // Setup with buffered operations
    setup_channel_with_buffers(&mut channel, comp);
    
    // Force drain
    channel.force_drain_all_buffers();
    
    // Verify final state matches expected after all ops applied
    let kinds = channel.extract_inserted_component_kinds();
    assert!(kinds.contains(&comp)); // Component should be inserted
}
```

**Test Execution:**
```bash
cargo test --package naia-shared force_drain
cargo test --package naia-shared migration
```

**Success Criteria:**
- [ ] Force-drain processes all buffered messages
- [ ] Component state correctly reflects final operations
- [ ] No panics with empty buffers
- [ ] No panics with full buffers

**Time Estimate:** 3 hours

---

### Stage 3: Entity Redirect System

**Objective:** Implement redirect tables for handling in-flight messages

**Implementation Order:**
1. Add `entity_redirects` field to `LocalEntityMap`
2. `LocalEntityMap::install_entity_redirect()` + test
3. `LocalEntityMap::apply_entity_redirect()` + test
4. `LocalWorldManager::update_sent_command_entity_refs()` + test

**Test Cases:**
```rust
#[test]
fn install_and_apply_redirect() {
    let mut entity_map = LocalEntityMap::new(HostType::Server);
    
    let old_entity = OwnedLocalEntity::Remote(42);
    let new_entity = OwnedLocalEntity::Host(100);
    
    // Install redirect
    entity_map.install_entity_redirect(old_entity, new_entity);
    
    // Apply redirect
    let redirected = entity_map.apply_entity_redirect(&old_entity);
    assert_eq!(redirected, new_entity);
    
    // Non-redirected entity returns itself
    let other_entity = OwnedLocalEntity::Remote(99);
    let not_redirected = entity_map.apply_entity_redirect(&other_entity);
    assert_eq!(not_redirected, other_entity);
}

#[test]
fn update_sent_command_references() {
    let mut world_manager = LocalWorldManager::new(...);
    
    // Setup: send some commands for an entity
    let global_entity = GlobalEntity::from_u64(1);
    world_manager.insert_component(&global_entity, &component_kind::<1>());
    
    // Capture sent commands before redirect
    let old_entity = OwnedLocalEntity::Remote(42);
    let new_entity = OwnedLocalEntity::Host(100);
    
    // Update references
    world_manager.update_sent_command_entity_refs(
        &global_entity,
        old_entity,
        new_entity
    );
    
    // Verify: retransmitted messages use new entity
    // (This requires inspecting sent_command_packets)
    // Implementation detail: may need helper to inspect
}
```

**Test Execution:**
```bash
cargo test --package naia-shared redirect
cargo test --package naia-shared migration
```

**Success Criteria:**
- [ ] Redirects installed correctly
- [ ] Apply redirect works for both redirected and non-redirected entities
- [ ] sent_command_packets updated correctly
- [ ] No memory leaks (redirects should eventually be cleaned up)

**Time Estimate:** 4 hours

---

### Stage 4: Server-Side Migration (Core)

**Objective:** Fix and complete `migrate_entity_remote_to_host`

**Implementation Order:**
1. Add `RemoteEngine::remove_entity_channel()` + test
2. Add `RemoteEngine::insert_entity_channel()` + test
3. Fix `LocalWorldManager::migrate_entity_remote_to_host()` with full implementation
4. Add comprehensive migration test

**Test Cases:**
```rust
#[test]
fn migrate_entity_remote_to_host_success() {
    // Setup
    let mut world_manager = LocalWorldManager::new(...);
    let global_entity = GlobalEntity::from_u64(1);
    
    // Create RemoteEntity with components
    let remote_entity = RemoteEntity::new(42);
    world_manager.entity_map.insert_with_remote_entity(global_entity, remote_entity);
    
    // Add some components
    let comp1 = component_kind::<1>();
    let comp2 = component_kind::<2>();
    world_manager.remote.spawn_entity(&remote_entity);
    // ... insert components ...
    
    // Migrate
    let new_host_entity = world_manager.migrate_entity_remote_to_host(&global_entity);
    
    // Verify: RemoteEntity no longer exists
    assert!(!world_manager.has_remote_entity(&remote_entity));
    
    // Verify: HostEntity now exists
    assert!(world_manager.has_host_entity(&new_host_entity));
    
    // Verify: GlobalEntity maps to new HostEntity
    let mapped_host = world_manager.entity_converter()
        .global_entity_to_host_entity(&global_entity)
        .unwrap();
    assert_eq!(mapped_host, new_host_entity);
    
    // Verify: Component state preserved
    // (Need to check HostEntityChannel has same components)
}

#[test]
fn migrate_with_buffered_operations() {
    // Setup entity with pending buffered operations
    let mut world_manager = setup_with_buffers();
    let global_entity = GlobalEntity::from_u64(1);
    
    // Buffer some operations that haven't been processed
    // ... (out-of-order messages)
    
    // Migrate (should force-drain first)
    let new_host_entity = world_manager.migrate_entity_remote_to_host(&global_entity);
    
    // Verify: all operations were applied (not lost)
    // Component state should reflect final state after drain
}

#[test]
#[should_panic(expected = "does not exist")]
fn migrate_nonexistent_entity_panics() {
    let mut world_manager = LocalWorldManager::new(...);
    let fake_entity = GlobalEntity::from_u64(999);
    
    world_manager.migrate_entity_remote_to_host(&fake_entity);
}

#[test]
#[should_panic(expected = "not remote-owned")]
fn migrate_host_entity_panics() {
    let mut world_manager = LocalWorldManager::new(...);
    let global_entity = GlobalEntity::from_u64(1);
    
    // Insert as HostEntity
    let host_entity = HostEntity::new(10);
    world_manager.entity_map.insert_with_host_entity(global_entity, host_entity);
    
    // Try to migrate (should panic - it's already host)
    world_manager.migrate_entity_remote_to_host(&global_entity);
}
```

**Test Execution:**
```bash
cargo test --package naia-shared migrate_entity_remote_to_host
cargo test --package naia-shared migration
cargo check --package naia-server  # Ensure server compiles
```

**Success Criteria:**
- [ ] Migration completes without panic
- [ ] Old RemoteEntity removed from all data structures
- [ ] New HostEntity inserted with correct component state
- [ ] Redirect installed correctly
- [ ] sent_command_packets updated
- [ ] Buffered operations drained before migration

**Time Estimate:** 5 hours

---

### Stage 5: Client-Side Migration

**Objective:** Implement client-side migration in MigrateResponse handler

**Implementation Order:**
1. Add helper methods to `LocalWorldManager` (extract_host_commands, etc.)
2. Add methods to create RemoteEntityChannel with components
3. Implement full `MigrateResponse` handler in `client.rs`
4. Add client-side migration tests

**Test Cases:**

Since client.rs is harder to unit test (requires full Client setup), we'll write integration tests:

```rust
// In test/tests/migration.rs (NEW FILE)

use naia_test::*;
use naia_client::Client;
use naia_server::Server;

#[test]
fn client_receives_migrate_response() {
    // Setup client with HostEntity
    let mut client = setup_test_client();
    let world_entity = spawn_test_entity(&mut client);
    
    // Client publishes entity
    client.publish_entity(&world_entity);
    
    // Simulate server sending MigrateResponse
    let migrate_response = create_migrate_response_message(...);
    
    // Client processes message
    let events = client.process_incoming(migrate_response);
    
    // Verify: entity now exists as RemoteEntity
    let global_entity = client.entity_to_global(&world_entity).unwrap();
    
    // Should be able to get RemoteEntity
    let remote_entity = client.connection.world_manager
        .entity_converter()
        .global_entity_to_remote_entity(&global_entity);
    assert!(remote_entity.is_ok());
    
    // Should NOT be able to get HostEntity
    let host_entity = client.connection.world_manager
        .entity_converter()
        .global_entity_to_host_entity(&global_entity);
    assert!(host_entity.is_err());
    
    // Verify: AuthGrant event emitted
    assert!(events.iter().any(|e| matches!(e, WorldEvent::AuthGrant(_))));
}

#[test]
fn client_migration_preserves_components() {
    let mut client = setup_test_client();
    let world_entity = spawn_entity_with_components(&mut client);
    
    // Get component state before migration
    let components_before = get_entity_components(&client, &world_entity);
    
    // Trigger migration
    simulate_delegation_flow(&mut client, &world_entity);
    
    // Get component state after migration
    let components_after = get_entity_components(&client, &world_entity);
    
    // Verify: same components exist
    assert_eq!(components_before, components_after);
}

#[test]
fn client_migration_replays_buffered_commands() {
    let mut client = setup_test_client();
    let world_entity = spawn_test_entity(&mut client);
    
    // Queue some commands
    client.insert_component(&world_entity, TestComponent::new());
    client.remove_component(&world_entity, OtherComponent);
    
    // Trigger migration while commands in flight
    simulate_migration(&mut client, &world_entity);
    
    // Verify: commands were replayed and eventually sent
    // (Check outgoing message queue)
}
```

**Test Execution:**
```bash
cargo test --package naia-client migration
cargo test --package naia-test migration
cargo check --package naia-client
```

**Success Criteria:**
- [ ] Client successfully processes MigrateResponse
- [ ] Entity migrates from Host to Remote
- [ ] Component state preserved
- [ ] Buffered commands replayed
- [ ] Invalid commands filtered out
- [ ] AuthGrant event emitted

**Time Estimate:** 6 hours

---

### Stage 6: Serialization & Redirect Handling

**Objective:** Apply redirects when reading/writing entity commands

**Implementation Order:**
1. Modify `world_writer.rs` to apply redirects before serialization
2. Modify `world_reader.rs` to apply redirects after deserialization
3. Add redirect handling tests

**Test Cases:**
```rust
#[test]
fn writer_applies_redirect_to_entity_reference() {
    let mut world_manager = LocalWorldManager::new(...);
    let global_entity = GlobalEntity::from_u64(1);
    
    // Setup entity with redirect
    let old_entity = OwnedLocalEntity::Remote(42);
    let new_entity = OwnedLocalEntity::Host(100);
    world_manager.entity_map.install_entity_redirect(old_entity, new_entity);
    
    // Create command referencing global_entity
    let command = EntityCommand::InsertComponent(global_entity, component_kind::<1>());
    
    // Serialize
    let mut writer = BitWriter::new();
    write_entity_command(&mut writer, &world_manager, command);
    let bytes = writer.to_bytes();
    
    // Deserialize and verify new entity ID was written
    let mut reader = BitReader::new(&bytes);
    let written_entity = OwnedLocalEntity::de(&mut reader).unwrap();
    
    assert_eq!(written_entity, new_entity);
}

#[test]
fn reader_applies_redirect_when_reading() {
    let mut world_manager = LocalWorldManager::new(...);
    
    // Setup redirect
    let old_entity = OwnedLocalEntity::Host(42);
    let new_entity = OwnedLocalEntity::Remote(100);
    world_manager.entity_map.install_entity_redirect(old_entity, new_entity);
    
    // Simulate receiving message with old entity ID
    let mut writer = BitWriter::new();
    old_entity.ser(&mut writer);
    let bytes = writer.to_bytes();
    
    // Read and apply redirect
    let mut reader = BitReader::new(&bytes);
    let mut read_entity = OwnedLocalEntity::de(&mut reader).unwrap();
    read_entity = world_manager.entity_map.apply_entity_redirect(&read_entity);
    
    assert_eq!(read_entity, new_entity);
}
```

**Test Execution:**
```bash
cargo test --package naia-shared world_writer
cargo test --package naia-shared world_reader
cargo test --package naia-shared redirect
```

**Success Criteria:**
- [ ] Redirects applied during serialization
- [ ] Redirects applied during deserialization
- [ ] Old entity IDs never hit the wire post-migration
- [ ] Late-arriving messages handled correctly

**Time Estimate:** 3 hours

---

### Stage 7: End-to-End Integration Tests

**Objective:** Test complete migration flow from client creation to delegation

**Test File:** `test/tests/entity_migration.rs` (NEW)

**Test Cases:**
```rust
#[test]
fn full_server_migration_flow() {
    // Setup
    let mut server = create_test_server();
    let mut client_connection = create_test_client_connection(&mut server);
    
    // Client creates entity
    let entity = client_creates_entity(&mut client_connection);
    
    // Client publishes entity
    client_publishes_entity(&mut client_connection, &entity);
    
    // Client requests delegation
    client_requests_delegation(&mut client_connection, &entity);
    
    // Server processes EnableDelegationResponse
    server.process_client_messages(&mut client_connection);
    
    // Verify: Server migrated entity from Remote to Host
    let global_entity = get_global_entity(&server, &entity);
    assert!(server_has_host_entity(&server, &global_entity));
    assert!(!server_has_remote_entity(&server, &global_entity));
    
    // Verify: Server sent MigrateResponse to client
    let outgoing = server.get_outgoing_messages(&client_connection);
    assert!(contains_migrate_response(&outgoing));
}

#[test]
fn full_client_migration_flow() {
    // Setup
    let (mut server, mut client) = create_connected_pair();
    
    // Client creates and publishes entity
    let world_entity = client.spawn_entity();
    client.publish_entity(&world_entity);
    
    // Synchronize (server receives entity)
    exchange_messages(&mut client, &mut server);
    
    // Client requests delegation
    client.enable_delegation(&world_entity);
    exchange_messages(&mut client, &mut server);
    
    // Server enables delegation (triggers migration)
    // Server sends MigrateResponse
    exchange_messages(&mut client, &mut server);
    
    // Verify: Client migrated entity from Host to Remote
    let global_entity = client.world_to_global(&world_entity).unwrap();
    assert!(client_has_remote_entity(&client, &global_entity));
    assert!(!client_has_host_entity(&client, &global_entity));
    
    // Verify: Client received AuthGrant
    let events = client.take_events();
    assert!(events.iter().any(|e| matches!(e, WorldEvent::AuthGrant(_))));
}

#[test]
fn migration_with_in_flight_messages() {
    let (mut server, mut client) = create_connected_pair();
    let world_entity = setup_delegated_entity(&mut client, &mut server);
    
    // Client sends component update BEFORE migration response arrives
    client.insert_component(&world_entity, TestComponent::new());
    let queued_messages = client.get_outgoing_messages();
    
    // Server performs migration and sends MigrateResponse
    trigger_migration(&mut server, &world_entity);
    exchange_migration_messages(&mut client, &mut server);
    
    // Client receives MigrateResponse
    // (Migration happens, redirect installed)
    
    // Previously queued messages are sent with OLD entity ID
    // But redirect table maps them to new ID
    
    // Server receives messages with old ID
    // Applies redirect, finds new entity
    server.process_client_messages(queued_messages);
    
    // Verify: Component update was applied to migrated entity
    let global_entity = get_global_entity(&server, &world_entity);
    assert!(server.entity_has_component(&global_entity, TestComponent::KIND));
}

#[test]
fn migration_preserves_component_state_e2e() {
    let (mut server, mut client) = create_connected_pair();
    
    // Setup entity with multiple components
    let world_entity = client.spawn_entity();
    client.insert_component(&world_entity, PositionComponent::new(1.0, 2.0));
    client.insert_component(&world_entity, VelocityComponent::new(0.5, -0.5));
    client.insert_component(&world_entity, NameComponent::new("TestEntity"));
    client.publish_entity(&world_entity);
    
    // Synchronize
    exchange_messages(&mut client, &mut server);
    
    // Capture component state before migration
    let components_before = get_all_components(&client, &world_entity);
    
    // Perform delegation (triggers migration)
    client.enable_delegation(&world_entity);
    exchange_all_messages_until_stable(&mut client, &mut server);
    
    // Capture component state after migration
    let components_after = get_all_components(&client, &world_entity);
    
    // Verify: exact same components with same values
    assert_eq!(components_before, components_after);
}

#[test]
fn stress_test_multiple_concurrent_migrations() {
    let (mut server, mut client) = create_connected_pair();
    
    // Create 10 entities
    let entities: Vec<_> = (0..10)
        .map(|_| create_published_entity(&mut client, &mut server))
        .collect();
    
    // Request delegation for all entities simultaneously
    for entity in &entities {
        client.enable_delegation(entity);
    }
    
    // Process all migrations
    exchange_all_messages_until_stable(&mut client, &mut server);
    
    // Verify: all entities migrated successfully
    for entity in &entities {
        let global_entity = client.world_to_global(entity).unwrap();
        assert!(client_has_remote_entity(&client, &global_entity));
        assert!(server_has_host_entity(&server, &global_entity));
    }
}
```

**Test Execution:**
```bash
cargo test --package naia-test
cargo test --package naia-test migration --nocapture
```

**Success Criteria:**
- [ ] Full server migration flow works
- [ ] Full client migration flow works
- [ ] In-flight messages handled correctly
- [ ] Component state preserved across migration
- [ ] Multiple concurrent migrations work
- [ ] No race conditions or deadlocks

**Time Estimate:** 8 hours

---

## Quality Gates & Validation

### Per-Stage Gates

After each stage, ALL of the following must pass:

```bash
# 1. Compilation
cargo check --all-targets

# 2. Clippy (linter)
cargo clippy --all-targets -- -D warnings

# 3. Format check
cargo fmt -- --check

# 4. Unit tests
cargo test --package naia-shared --lib

# 5. Integration tests
cargo test --package naia-test

# 6. Doc tests
cargo test --doc

# 7. Examples compile
cargo check --examples
```

### Gate Script

Create `scripts/quality_gate.sh`:

```bash
#!/bin/bash
set -e

echo "=== Quality Gate Check ==="
echo ""

echo "→ Checking compilation..."
cargo check --all-targets

echo "→ Running clippy..."
cargo clippy --all-targets -- -D warnings

echo "→ Checking formatting..."
cargo fmt -- --check

echo "→ Running unit tests..."
cargo test --package naia-shared --lib

echo "→ Running integration tests..."
cargo test --package naia-test

echo "→ Running doc tests..."
cargo test --doc

echo "→ Checking examples..."
cargo check --examples

echo ""
echo "✓ All quality gates passed!"
```

Usage:
```bash
chmod +x scripts/quality_gate.sh
./scripts/quality_gate.sh
```

### Continuous Validation

**After every significant change:**
1. Run `cargo check` (30 seconds)
2. Run relevant package tests (1-2 minutes)
3. Full gate before committing (5-10 minutes)

**Before merging/completing stage:**
1. Full quality gate (mandatory)
2. Visual code review
3. Update documentation

---

## Development Workflow

### Workflow Pattern (Per Stage)

```
┌─────────────────────────────────────────────┐
│ 1. READ IMPLEMENTATION PLAN FOR STAGE      │
│    - Understand what to build               │
│    - Review test cases                      │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│ 2. WRITE TESTS FIRST (RED)                 │
│    - Create test file if needed             │
│    - Write failing test for feature         │
│    - Run: cargo test <test_name>            │
│    - Verify test fails (RED)                │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│ 3. IMPLEMENT FEATURE (GREEN)                │
│    - Write minimal code to pass test        │
│    - Run: cargo test <test_name>            │
│    - Iterate until test passes (GREEN)      │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│ 4. REFACTOR & CLEAN UP                      │
│    - Improve code quality                   │
│    - Add comments/documentation             │
│    - Keep tests green                       │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│ 5. RUN FULL TEST SUITE                      │
│    - cargo test --package naia-shared       │
│    - Ensure no regressions                  │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│ 6. RUN QUALITY GATE                         │
│    - ./scripts/quality_gate.sh              │
│    - Fix any issues                         │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│ 7. COMMIT & PROCEED TO NEXT STAGE           │
│    - git add <files>                        │
│    - git commit -m "Stage X: description"   │
└─────────────────────────────────────────────┘
```

### Example: Stage 1 Execution

```bash
# 1. Create test file
touch shared/src/world/sync/tests/migration.rs

# 2. Write test (RED)
# Edit migration.rs with test_extract_component_kinds()
cargo test --package naia-shared test_extract_component_kinds
# FAILS ✗

# 3. Implement method (GREEN)
# Edit remote_entity_channel.rs, add extract_inserted_component_kinds()
cargo test --package naia-shared test_extract_component_kinds
# PASSES ✓

# 4. Refactor
# Clean up, add docs, improve names
cargo test --package naia-shared test_extract_component_kinds
# STILL PASSES ✓

# 5. Full test suite
cargo test --package naia-shared
# ALL PASS ✓

# 6. Quality gate
./scripts/quality_gate.sh
# ALL PASS ✓

# 7. Commit
git add shared/src/world/sync/remote_entity_channel.rs
git add shared/src/world/sync/tests/migration.rs
git commit -m "Stage 1: Add extract_inserted_component_kinds() method"
```

---

## Rollback Strategy

### If Stage Fails

**Option 1: Fix Forward**
- If problem is minor, fix and re-run tests
- Iterate until green

**Option 2: Rollback Stage**
```bash
# Revert uncommitted changes
git restore .

# Revert last commit
git reset --soft HEAD~1

# Revert multiple commits
git reset --soft HEAD~N
```

**Option 3: Create Fix Branch**
```bash
# If complex issue, branch off
git checkout -b fix-stage-N-issue
# Work on fix
# Once fixed, merge back
git checkout main
git merge fix-stage-N-issue
```

### If Tests Break Unexpectedly

1. **Identify failing tests**
   ```bash
   cargo test 2>&1 | grep "FAILED"
   ```

2. **Run single test with output**
   ```bash
   cargo test test_name -- --nocapture
   ```

3. **Check git diff**
   ```bash
   git diff
   ```

4. **Revert suspect changes**
   ```bash
   git restore <file>
   ```

### Nuclear Option

If everything breaks and you need to start over:

```bash
# Stash all changes
git stash

# Return to last known good commit
git log --oneline  # Find good commit
git reset --hard <good_commit_hash>

# Or return to beginning of migration work
git reset --hard release-0.25.0-c
```

---

## Test Coverage Tracking

### Coverage Report

```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --package naia-shared --out Html

# View report
open tarpaulin-report.html
```

### Target Coverage

- **Core migration functions:** 100% line coverage
- **Helper methods:** 90%+ line coverage
- **Edge cases:** All explicitly tested
- **Panic paths:** All have `#[should_panic]` tests

---

## Success Criteria Summary

### Stage-by-Stage Checklist

- [ ] **Stage 0:** Baseline green, no compilation errors
- [ ] **Stage 1:** Channel methods work, tests pass
- [ ] **Stage 2:** Force-drain works, buffers cleared
- [ ] **Stage 3:** Redirects work, messages updated
- [ ] **Stage 4:** Server migration works, state preserved
- [ ] **Stage 5:** Client migration works, commands replayed
- [ ] **Stage 6:** Serialization applies redirects
- [ ] **Stage 7:** End-to-end tests pass

### Final Validation (All Must Pass)

```bash
# Full test suite
cargo test

# No warnings in release mode
cargo build --release 2>&1 | grep warning
# Should output nothing

# Clippy clean
cargo clippy --all-targets -- -D warnings

# Format clean
cargo fmt -- --check

# Examples compile
cargo check --examples

# Documentation builds
cargo doc --no-deps

# Manual testing with demo
cargo run --package naia-bevy-server-demo &
cargo run --package naia-bevy-client-demo
# Test entity creation → publish → delegate → mutate
```

---

## Timeline Estimate

| Stage | Description | Time | Cumulative |
|-------|-------------|------|------------|
| 0 | Baseline | 0.5h | 0.5h |
| 1 | Core channel methods | 2h | 2.5h |
| 2 | Force-drain | 3h | 5.5h |
| 3 | Redirect system | 4h | 9.5h |
| 4 | Server migration | 5h | 14.5h |
| 5 | Client migration | 6h | 20.5h |
| 6 | Serialization | 3h | 23.5h |
| 7 | E2E tests | 8h | 31.5h |
| **TOTAL** | | **~32 hours** | |

**With buffer for debugging:** 40-45 hours

**Suggested schedule:**
- Day 1-2: Stages 0-2 (foundation)
- Day 3-4: Stages 3-4 (server migration)
- Day 5-6: Stages 5-6 (client migration)
- Day 7-8: Stage 7 (integration + polish)

---

## Emergency Contacts

If stuck on any stage:

1. **Check existing similar code** in `shared/src/world/sync/tests/engine.rs`
2. **Review architecture docs** in module-level comments
3. **Examine entity channel FSM** in `remote_entity_channel.rs` header
4. **Reference spec** in `MIGRATION_FEATURE_SPEC.md`
5. **Reference plan** in `MIGRATION_IMPLEMENTATION_PLAN.md`

**Key principle:** When in doubt, write a test first!

---

**END OF DEVELOPMENT STRATEGY**

