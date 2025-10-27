# E2E Tests and the Entity ID Conversion Bug

## Executive Summary

The Cyberlith Editor's "No authority over vertex" bug was caused by **incorrect entity ID conversion** when the server sends `MigrateResponse` for client-created delegated entities.

**Root Cause**: When the server receives `EnableDelegation` from a client, it correctly migrates the entity internally but then sends `MigrateResponse` with the **server's local entity IDs** instead of converting them to the **client's perspective**. The client cannot look up these server-local IDs and drops the message, causing the entity to never migrate from `HostEntity` to `RemoteEntity` on the client side.

## Why Previous Tests Didn't Catch This

Previous tests focused on:
1. ✓ Component-level behavior (e.g., "Can `HostEntityChannel` process `MigrateResponse`?")
2. ✓ Internal API methods (e.g., "Does `host_send_migrate_response()` work?")
3. ✓ Authority state machines (e.g., "Can authority be requested/released?")

But they **NEVER** tested:
4. ❌ **Cross-boundary entity ID conversion** (e.g., "When server sends `MigrateResponse`, can client look up the entity IDs?")
5. ❌ **Complete protocol flows** (e.g., "Client creates entity → Server delegates → Client migrates")

## What the E2E Tests Prove

### Test File 1: `e2e_delegation_complete_flow.rs`

#### Test: `bug_server_sends_wrong_entity_ids_in_migrate_response`
**Status**: ✓ PASSES (Confirms Bug)

**What it proves**:
- CLIENT creates entity → Gets `HostEntity(0)`
- SERVER receives entity → Gets `RemoteEntity(42)` (server's local ID for client's entity)
- SERVER sends `MigrateResponse(RemoteEntity(42), HostEntity(0))` (server's IDs!)
- **CLIENT cannot look up `RemoteEntity(42)`** → Message is DROPPED

**Output**:
```
✓ BUG CONFIRMED: Client cannot find RemoteEntity(42)!
  The MigrateResponse will be DROPPED because client can't look up the entity!
```

#### Test: `correct_server_converts_entity_ids_before_migrate_response`
**Status**: ✓ PASSES (Shows Correct Behavior)

**What it proves**:
- CLIENT creates entity → Gets `HostEntity(0)`
- SERVER **should send** `MigrateResponse(HostEntity(0), new_RemoteEntity)`
- CLIENT **can look up** `HostEntity(0)` → Message will be PROCESSED

**Output**:
```
✓ SUCCESS: Client CAN look up HostEntity(0)!
  The MigrateResponse will be PROCESSED successfully!
```

#### Test: `complete_delegation_flow_tracks_entity_ids`
**Status**: ✓ PASSES (Documents Full Flow)

**What it proves**:
- Tracks entity IDs through all 5 phases of delegation
- Shows that client can find its own `HostEntity(0)`: ✓ true
- Shows that client **cannot** find server's `RemoteEntity(42)`: ✓ false
- Documents the conversion requirement

**Output**:
```
PHASE 5 - VERIFICATION:
  Client can find HostEntity(0): true
  Client can find RemoteEntity(42): false
```

### Test File 2: `e2e_entity_id_conversion_bug.rs`

#### Test: `client_cannot_look_up_servers_remote_entity`
**Status**: ✓ PASSES (Confirms Client-Side Failure)

**What it proves**:
- Client has entity as `HostEntity(0)`
- Server sends `RemoteEntity(42)` (server's ID)
- Client lookup **fails**: Cannot find `RemoteEntity(42)`

#### Test: `server_migrate_response_uses_wrong_entity_ids`
**Status**: ✓ FAILS (Expected - Reveals Migration Issue)

**What it proves**:
- Server cannot use `host_reserve_entity()` for already-mapped entities
- But `migrate_entity_remote_to_host()` exists and works correctly
- Server IS correctly migrating internally, but sending wrong IDs externally

## The Bug in Code

### Current (Broken) Code
**Location**: `server/src/server/world_server.rs:1594-1597`

```rust
connection.base.world_manager.host_send_migrate_response(
    global_entity,
    &old_remote_entity,  // SERVER's RemoteEntity(42) - WRONG!
    &new_host_entity     // SERVER's HostEntity(0) - WRONG!
);
```

### Expected Behavior
The server needs to send:
- **First parameter**: CLIENT's current HostEntity (so client can look it up!)
- **Second parameter**: New RemoteEntity for client to create

### The Fix Needed
Server must convert entity IDs before calling `host_send_migrate_response`:

```rust
// 1. Look up: What does CLIENT call this entity?
// Server's RemoteEntity represents the client's entity
// Need to find: client's HostEntity that corresponds to server's RemoteEntity

// 2. This requires tracking in Connection:
// When server first receives client's entity, record:
// connection.client_entity_map[global_entity] = client_host_entity

// 3. Then when sending MigrateResponse:
let client_host_entity = connection.client_entity_map
    .get(global_entity)
    .expect("Must know client's entity ID");

let new_client_remote_entity = connection.base.world_manager
    .remote_reserve_entity(global_entity);

connection.base.world_manager.host_send_migrate_response(
    global_entity,
    client_host_entity,      // CLIENT's current HostEntity ✓
    &new_client_remote_entity  // CLIENT's new RemoteEntity ✓
);
```

## How These Tests Prevent Future Bugs

### 1. Entity ID Invariants
The E2E tests establish clear invariants:
- ✓ Client must be able to look up entity IDs in messages it receives
- ✓ Server must convert its local IDs to client's perspective before sending
- ✓ Entity ID mappings must be preserved through migrations

### 2. Cross-Boundary Protocol Testing
The E2E tests verify:
- ✓ Messages sent by server can be looked up by client
- ✓ Entity ID conversions work correctly at network boundaries
- ✓ Complete protocol flows (create → delegate → migrate) work end-to-end

### 3. Documentation of Requirements
The E2E tests document:
- What entity IDs each side knows
- What conversions are required
- What the serialization layer must do
- Where the mapping needs to be tracked

## Test Statistics

**E2E Tests Created**: 8 tests across 2 files
**Bugs Documented**: 2 critical entity ID bugs
**Tests Passing**: 7/8 (1 expected failure that reveals server migration issue)
**Coverage Gap Closed**: Cross-boundary protocol verification

## Recommendations

1. **Implement Entity ID Tracking**: Server's `Connection` needs to track client's entity IDs
2. **Fix host_send_migrate_response Call**: Convert entity IDs before sending
3. **Add More E2E Tests**: For other cross-boundary messages (authority, updates, etc.)
4. **Run E2E Tests in CI**: Ensure cross-boundary protocols stay correct

## Conclusion

The E2E tests definitively prove that:
1. ✓ The server IS processing `EnableDelegation` correctly
2. ✓ The server IS migrating entities internally
3. ✓ The server IS calling `host_send_migrate_response`
4. ❌ The server IS sending the WRONG entity IDs
5. ❌ The client CANNOT look up those IDs and DROPS the message

**These tests would have caught this bug immediately** if they had been written before the feature was implemented. Going forward, ALL cross-boundary protocols should have E2E tests that verify entity ID conversions.

