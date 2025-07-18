# Naia Refactor: Eliminate `SystemChannel` and unify all entity-affecting messages

## 0. Goal
Remove cross-channel ordering races by making **one** totally-ordered, reliable stream for *all* entity / component traffic.  All current `SystemChannel` messages will travel through the existing **Reliable-Actions** stream.

## 1. Rationale (the “why”) 
1. Two reliable channels (Actions + System) provided *in-order* delivery **per channel** but **no causal ordering between channels**.  In practice a `EnableDelegationEntity` could arrive before/after an `InsertComponent` that was already in flight, causing logic errors (see `GlobalDiffHandler` panic).
2. One ordered stream makes causal assumptions explicit, preventing an entire class of bugs.  Latency impacts are acceptable for correctness.
3. We keep the panic in `register_component()` – duplicate registration still indicates a logic error.

## 2. Scope (the “what”) 
* Delete `SystemChannel` **entirely**.
* Extend `EntityActionEvent` and `EntityActionType` to include all former System messages:
  - `PublishEntity` / `UnpublishEntity`
  - `EnableDelegationEntity` / `DisableDelegationEntity`
  - `RequestAuthority(u16)` / `ReleaseAuthority`
  - `UpdateAuthority(EntityAuthStatus)`
  - `EntityMigrateResponse(u16)`
* All send-sites that formerly used
  `send_message::<SystemChannel, EntityEventMessage>(…)` now push a new `EntityActionEvent` into `world_channel.outgoing_actions`.
* Serialise / deserialise the new variants in `HostWorldWriter::write_action` and matching reader logic in `WorldChannel`.
* Delete every reference to `SystemChannel`, its registration in `shared/src/protocol.rs`, and the special-case receive branches in `connection.rs`.
* Maintain a **single** packet writer/queue per connection.

## 3. Detailed step list
1. **Remove type & protocol registration**  
   • Delete `shared/src/messages/channels/system_channel.rs`.  
   • In `shared/src/protocol.rs` & `shared/src/lib.rs` remove imports and `channel_kinds.add_channel::<SystemChannel>(…)`.

2. **Enum extensions**  
   Modify the following files:  
   • `shared/src/world/entity/entity_action_type.rs` – add new discriminants.  
   • `shared/src/world/host/entity_action_event.rs`  
   • `shared/src/world/remote/entity_action_event.rs`  
   (Both host & remote variants must stay in sync.)

3. **Serialisation**  
   `shared/src/world/host/host_world_writer.rs::write_action()` – add `match` arms to emit tag + payload for each new variant.  Payload encoding mirrors existing `EntityEventMessage` logic.

4. **Deserialisation & application**  
   *Reader path* inside `WorldChannel` (`on_remote_*` helpers):  
   add handlers for every new variant to translate the action into the same in-memory behaviour currently triggered by `Client::process_response_events` / `Server::process_response_events`.

5. **Send-site rewrites**  
   Replace each `send_message::<SystemChannel, …>` call:  
   ```rust
   let msg = EntityEventMessage::new_enable_delegation(&converter, world_entity);
   let action: EntityActionEvent = msg.action.into();
   connection.base.host_world_manager
             .world_channel
             .outgoing_actions
             .send_message(action);
   ```
   Do this in `client/src/client.rs` and `server/src/server/world_server.rs` at every logged location.

6. **Connection receive branches**  
   In both client & server `connection.rs`, delete the special case that parsed `SystemChannel` packets.

7. **Clean-up**  
   `git grep SystemChannel` must return **0** results.  All unused `use` lines removed.

8. **Bit-width check**  
   If `EntityActionType` exceeds 8 variants, ensure the on-wire bitfield is wide enough (was 3-bit; bump to 4-bit if ≥8 variants).

## 4. Testing / validation strategy (the “how not to break things”)
1. **Compile everything** – expect many errors until replacements are done; iterate until `cargo test`/`cargo check` passes across workspace.
2. **Local integration test**  
   • Run `demos/basic` server & client on localhost, observe logs while:
     – spawning entity, publishing, delegating, inserting component, despawning.  
   • Add artificial packet loss/re-order via existing Conditioner (set e.g. 30 % loss, 100 ms jitter).  Logs must remain ordered, no panics.
3. **Stress test**  
   • Spawn thousands of entities/components quickly; ensure control messages (delegation, authority) still arrive and apply.
4. **Bandwidth sanity**  
   Observe outgoing packet sizes; verify no unexpected blow-up after merge.
5. **Guardrail remains**  
   Confirm that duplicate-registration panic still triggers if we force-send a duplicate action (unit test can inject two identical InsertComponent actions).

## 5. Risk areas & mitigations
| Risk | Mitigation |
|------|------------|
| Missed send-site still tries to use SystemChannel | `cargo check` & `grep SystemChannel` after deletion will fail build / reveal leftovers |
| Deserialiser tag mismatch | Keep `EntityActionType` enum in one file and re-export – compile-time mismatch impossible if both sides share crate |
| Packet overflow due to extra variants | Fragmentation already in writer; ensure tag size bump if needed |
| Latency regression | Acceptable for this refactor – revisit if gameplay impact observed |

## 6. Done criteria
* Build passes, SystemChannel absent.  
* End-to-end demo runs with packet loss, no ordering-related panics.  
* Document (this file) checked into VCS along with code changes. 