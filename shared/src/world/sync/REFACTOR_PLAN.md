# Unified Stream‑Template Replication System

*(Refactor & Test‑Plan Spec)*

---

\## 0 · Preamble
This document is the single **source of truth** for the Naia networking refactor. It specifies **what** to build (functional spec), **why** (design rationale), and **how** to migrate from `entity_command_sender.rs` / `entity_message_receiver.rs` to the new *template‑driven* replication engine. Every invariant required for correctness is spelled out so that an automated coding agent can implement the system deterministically.

> **Scope guard** — The **packet header is frozen** (16‑bit `MessageIndex`). Only the **payload schema** may evolve.
>
> **Out of scope** — delta compression, partial reliability, player‑input prediction, encrypted transport, runtime tracing/metrics hooks.

---

\## 1 · Glossary & Notation

| Term/Type                  | Meaning / Contract                                                                                              |
| -------------------------- | --------------------------------------------------------------------------------------------------------------- |
| **Event**                  | `{ seq:u16, path:[PathSeg;≤2], kind:MsgKind, payload:Bytes }`                                                   |
| **Stream**                 | Ordered sequence of `Event`s that share a `template` and unique `path`.                                         |
| **Template**               | Compile‑time declarative description of allowed states & child templates.                                       |
| **Engine**                 | Dispatcher that routes `Event`s to `Stream`s and drives the FSM.                                                |
| **Guard band / Near‑wrap** | Region `seq ≥ FLUSH_THRESHOLD` where backlog is purged to avoid wrap ambiguity.                                 |
| **Generation gate**        | Per‑stream `spawn_seq`; drops packets older than the last `Spawn` verb.                                         |
| **MAX\_IN\_FLIGHT**        | Upper bound on un‑ACKed packets (32 767) which guarantees half‑range ordering.                                  |
| `ahead(a,b)`               | Half‑range comparison: `0 < (a-b) mod 65 536 < 32 768`.                                                         |

---

\## 2 · Objectives

| Goal                    | Success criterion                                                     | Edge‑cases covered                          |
| ----------------------- | --------------------------------------------------------------------- | ------------------------------------------- |
| **Deterministic order** | Receiver rebuilds world equal to sender’s causal timeline.            | Loss, duplication, re‑ordering, wrap‑around |
| **Safe EntityId reuse** | Respawned entity cannot be corrupted by late packets.                 | RTT‑delayed dupes, wrap‑around dupes        |
| **Declarative rules**   | FSM encoded in `const` tables; no handwritten `match`.                | Undefined transition ⇒ compile‑time error   |
| **O(1) per entity**     | Memory does not scale with history; tombstones purged on next wrap.   | Spawn‑despawn storms, late dupes            |
| **Tiny core**           | Engine generic part ≤ 250 LOC; complexity lives in data tables/tests. |                                             |
| **Proven by tests**     | Compile‑time, unit, and fuzz tests demonstrate invariants incl. wrap. |                                             |

---

\## 3 · Core Abstractions

\### 3.1 Data types

Engine operates directly on the existing wire-level
`EntityMessage<RemoteEntity>` enum that already encodes all semantic
variants (`SpawnEntity`, `DespawnEntity`, `InsertComponent`, …).  No
separate `Event` or `MsgKind` types are used anymore.  The only helper
needed is:

```rust
pub enum Path {
    Entity(RemoteEntity),
    EntityComponent(RemoteEntity, ComponentKind),
}
```

`Path` is an internal key; its construction happens inside the facade
when translating an `EntityMessage` into engine bookkeeping.  The
caller never sees it.

\### 3.2 Runtime objects

| Symbol                       | Fields                                                 | Notes                                                                                                                                 |
| ---------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------- |
| **Stream**                   | `{ template_id, state, last_seq, spawn_seq, backlog }` | `backlog` is a `VecDeque<EntityMessage>` pre-reserved to `MAX_IN_FLIGHT`. Push refuses if full (drops).                                       |
| **Engine<T: RootTemplate>** | `{ streams: HashMap<Path, Stream>, outgoing: Vec<EntityMessage> }` | `push(msg)` routes the message; delivered events are queued in `outgoing` and drained each tick. |

\### 3.3 Templates & Callbacks (stable)

* `Template::on_apply(&Event, &mut Context)` — may **push** commands via `Context`, MUST NOT mutate `stream.state`.

---

\## 4 · Static Configuration (`config.rs`)

```rust
pub const MAX_IN_FLIGHT: u16 = 32_767;                    // half‑range window (must stay < 32_768)
pub const FLUSH_THRESHOLD: u16 = 65_536 - MAX_IN_FLIGHT;  // 32 769 — do NOT edit directly
```

---

\## 5 · Engine Semantics

1. **Locate stream** by `path`
2. **Sequence check**  `if !sequence_greater_than(seq, stream.last_seq) { drop; return; }` (half‑range strict).
3. **Generation gate**  `if sequence_less_than(seq, stream.spawn_seq) { drop; return; }`.
4. **Apply or buffer**

   * **Valid transition** – mutate `state`, set `last_seq`, invoke `on_apply`.
   * **Spawn ↔ Despawn race** – when a `Spawn` or `Despawn` arrives, scan backlog and keep only the newest of the two kinds per path.
   * **Otherwise** – buffer event; backlog is bounded to `MAX_IN_FLIGHT`.
5. **Guard band flush**

   * On each delivery: if `last_seq ≥ FLUSH_THRESHOLD` set `near_wrap=true` and drop any backlog event with `seq < FLUSH_THRESHOLD`.
   * When `near_wrap && seq < FLUSH_THRESHOLD`, clear `near_wrap` (wrap finished).
6. **Backlog drain** — attempt to deliver now‑unblocked buffered events in order.
7. **Tombstone GC** — When stream becomes terminal *and* `near_wrap` just cleared *and* backlog empty, remove stream; no timers.

---

\## 6 · Sequence‑Number Wrap Handling (pure‑u16)

* Sender window ≤ `MAX_IN_FLIGHT` (< 32 768).
* Half‑range helpers guarantee unambiguous order.
* Guard‑band flush purges stale packets and tombstones.

Edge‑cases handled: duplicate packets, burst across wrap, RTT‑delayed old‑epoch packets.

---

\## 7 · Sender Responsibilities

* Maintain `MessageIndex`, write `MessageIndex` to packet.
* Block send when un‑ACKed window ≥ `MAX_IN_FLIGHT`; `debug_assert!` enforces in debug builds.
* No ordering/FSM logic on sender.

---

\## 8 · Refactor Roadmap

| Phase | Work item                                                                                   | Notes |
| ----- | ------------------------------------------------------------------------------------------- | ----- |
| P0    | Add enums/const tables & `config.rs`;                       |       |
| P1    | Implement `engine.rs` with guard‑band & Spawn/Despawn race rule.                            |       |
| P2    | Add concrete templates (`templates/entity_template.rs`, `templates/component_template.rs`). |       |
| P3    | Integrate tests & CI.                                                                       |       |

---

\## 9 · Testing Strategy

| Layer        | Framework                 | Mandatory cases                                                                                                                                                                        |
| ------------ | ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Compile‑time | `static_assertions`       | Transition uniqueness; enum exhaustiveness;`MAX_IN_FLIGHT<32_768`.                                                                                                    |
| Unit         | `#[test]`                 | Each transition row; Spawn/Despawn race; guard‑band flush; wrap trace `65 530→65 535→0→1`.                                                                                             |
| Fuzz / Prop  | `proptest` / `cargo-fuzz` | Generate ≤ `MAX_IN_FLIGHT` causal traces, permute loss/dup/OOO, force wrap; replay causal order in new engine and assert `(path,state,last_seq,spawn_seq)` equality (backlog ignored). |

**TDD quick loop:**

Run only the sync-module tests with minimal noise:

```bash
RUSTFLAGS="-Awarnings" cargo test -q -p naia-shared --lib world::sync::tests
```

This compiles quietly (warnings suppressed), executes just the `shared/src/world/sync/tests` suite, and prints only the pass/fail summary.

---

\## 11 · Deliverables & Exit Criteria

* **Code**: all files compile under `#![deny(warnings, missing_docs)]`.
* **Docs**: `README.md` (overview\.diagram), `CONTRIB_TESTS.md`.
* **Exit**: unit + fuzz tests pass; legacy path deleted.

---

\## 12 · Appendix A — Formal Proof Sketch for Wrap‑Correctness

*missing!*

---

## 13 · Directory Layout & Naming Conventions  *(NEW)*

All new replication types live under the `sync` sub-module to keep a clean boundary with legacy code until full cut-over.

```
shared/
  src/
    world/
      sync/
        mod.rs                  // pub use {config, event, engine, templates::*}
        config.rs               // MAX_IN_FLIGHT, FLUSH_THRESHOLD
        event.rs                // `Event`, `PathSeg`, `MsgKind`, helpers
        path.rs                 // `PathKey` 64-bit hash util
        stream.rs               // `Stream` struct & impl
        engine.rs               // `Engine<T: RootTemplate>`
        templates/
          mod.rs
          entity_template.rs    // depth 0 rules
          component_template.rs // depth 1 rules
        tests/
          mod.rs
          engine.rs             // invariants & state transition table
          wrap.rs               // near-wrap & guard-band behaviour
          fuzz.rs               // cargo-fuzz harness (optional)
```

---

## 14 · Thin Facade: `EntityMessageReceiver`

`EntityMessageReceiver<E>` remains the public entry-point exposed to higher-level crates but internally delegates **all** ordering & FSM logic to `sync::Engine`.

````rust
pub struct EntityMessageReceiver<E: Copy + Hash + Eq> {
    inner: sync::Engine<templates::Root>,
    receiver: ReliableReceiver<EntityMessage<E>>,
}
````

### 14.1  Responsibilities

- Use `ReliableReceiver` to collect and **de-duplicate** incoming `EntityMessage<E>` packets (enforcing strict `MessageIndex` order).
- Translate each `(index, msg)` produced by the receiver into a `sync::Event`:
  * `seq`  ← `MessageIndex`
  * `path` ← `[PathSeg::Entity(id)]` _(depth 0)_ **or** `[Entity, CompKind]` _(depth 1)_
  * Event variant itself conveys the semantic (`SpawnEntity`, `InsertComponent`, etc.) – no separate kind enum.
  * `payload` ← empty `Bytes` for now – actual component diff bytes will be added later.
- Push the event into `inner.push(event)`.
- Each call to `receive_messages()` drains `inner.context().drain_commands()` and returns the vector, preserving current API.
- No per-entity state is stored here any more – memory leak fixed by engine’s tombstone GC.

---

## 15 · Test-Driven Refactor Plan  *(NEW – supersedes §8 table)*

| Step | Description | Target Path | Test File | Status |
| ---- | ----------- | ----------- | --------- | ------ |
| **S0** | Create `sync/` module |
| **S1** | (removed) Wrap-around helpers already thoroughly tested in `wrapping_number.rs`; no additional tests required | - | - | ✔ |
| **S2** | Implement `Path` key hashing & collision tests | `path.rs` | `tests/path.rs` | ☐ |
| **S3** | Implement `Stream` data structure with backlog & guard-band logic | `stream.rs` | `tests/stream.rs` | ☐ |
| **S4** | Implement minimal `Engine` routing + backlog drain | `engine.rs` | `tests/engine_spawn.rs` | ☐ |
| **S5** | Flesh out component-level template + race rules | `templates/component_template.rs` | `tests/component.rs` | ☐ |
| **S6** | Fuzz harness exercising ≤ `MAX_IN_FLIGHT` traces | `tests/fuzz.rs` | - | ☐ |
| **S7** | Delete dead code paths, update docs, activate CI gate | repo-wide | - | ☐ |

*Check-box table will be updated during implementation PRs.*

---

## 16 · Additional Notes

* The new `sync` module is `#![no_std]`-compatible except for `HashMap`/`VecDeque`; gating with the existing `std` feature is acceptable.
* Engine exposes a plain `outgoing_events: Vec<EntityMessage>`; no extra context struct.
* Eventually `EntityCommandSender`'s window enforcement will consume `MAX_IN_FLIGHT` directly from `sync::config` to keep values DRY.

---

## 17 · Session Progress Snapshot — 2025-01-28

### 17.1 Current Code State

* `sync::engine` exists but is a stub – it naïvely pushes every inbound message straight to `outgoing_events`.  
  * Lacks: half-range ordering, backlog, guard-band, generation gate, path routing, etc.
* `sync::config` contains constants and default `EngineConfig` with `max_in_flight=32767` and `flush_threshold=65 536-max_in_flight`.
* No `path.rs`, `stream.rs`, or `templates/` yet – only placeholders in the plan.
* All unit tests in `sync/tests/engine.rs` **currently fail** (they compile though)

### 17.2 Outstanding TODOs Analysis

**Comprehensive TODO audit completed** - discovered 12 scattered TODOs across the codebase that must be addressed as part of the refactor:

**Entity Channel Management (5 TODOs) - HIGHEST PRIORITY**
* `shared/src/world/entity/entity_message_sender.rs:52-84` - Channel open/close operations
  * ✅ Infrastructure layer - foundational for other systems
  * ✅ Self-contained implementation scope
  * ✅ Clear interface already defined in `EntityChannelSender`

**World Manager Operations (2 TODOs) - SECOND PRIORITY** 
* `shared/src/world/local_world_manager.rs:288,295` - Redundant remote entity tracking
  * Used during authority delegation handover process
  * Pattern established in existing `add_redundant_remote_entity_to_host` function

**Message Processing (1 TODO) - THIRD PRIORITY**
* `shared/src/world/sync/receiver_engine.rs:71` - Authority delegation message engine
  * Requires separate engine for delegation response handling
  * Builds on channel management infrastructure

**Entity Tracking and Migration (4 TODOs) - FINAL PRIORITY**
* `client/src/client.rs:1168,1185` - Client-side entity migration tracking
* `server/src/server/world_server.rs:1578,2070` - Server-side entity migration tracking
  * Complex integration points affecting authority delegation
  * Depends on all previous implementations

### 17.3 Revised Implementation Roadmap (Priority-Based)

**Phase 1: Entity Channel Management (Est: 1-2 days)**
```rust
// Implement in EntityChannelSender & SenderEngine
impl EntityChannelSender {
    pub fn open_entity_channel(&mut self) -> bool { /* track channel state */ }
    pub fn close_entity_channel(&mut self) -> bool { /* cleanup state */ }
    pub fn open_component_channel(&mut self, kind: ComponentKind) -> bool { /* ... */ }
    pub fn close_component_channel(&mut self, kind: ComponentKind) -> bool { /* ... */ }
}
```

**Phase 2: World Manager Operations (Est: 1 day)**
```rust
// Implement redundant entity tracking for authority handover
impl LocalWorldManager {
    pub fn track_hosts_redundant_remote_entity(&mut self, remote_entity: &RemoteEntity, component_kinds: &Vec<ComponentKind>) {
        self.remote.track_redundant_entity(remote_entity, component_kinds);
    }
    pub fn untrack_hosts_redundant_remote_entity(&mut self, remote_entity: &RemoteEntity) {
        self.remote.untrack_redundant_entity(remote_entity);
    }
}
```

**Phase 3: Message Processing (Est: 1 day)**
```rust
// Extract authority delegation messages to separate engine
if matches!(msg.get_type(), EntityMessageType::EnableDelegationResponse | /*...*/) {
    self.auth_engine.accept_message(id, msg);
    return;
}
```

**Phase 4: Entity Migration Integration (Est: 2-3 days)**
* Client/server authority tracking integration
* Complete migration handover logic
* End-to-end authority delegation testing

**Parallel Track: Core Sync Engine (Independent)**
Following original roadmap from §17.3 - can proceed independently:
1. Implement `Stream` (`stream.rs`)
2. Flesh out `Engine::push` with path routing
3. Satisfy `tests/engine.rs` test suite

### 17.4 Next Immediate Steps

1. **Start with Phase 1** - Entity Channel Management implementation provides immediate foundation
2. **Validate with existing patterns** - Use `EntityChannelReceiver` as implementation reference
3. **Test incrementally** - Each phase can be tested independently before proceeding
4. **Coordinate with sync engine** - Channel management will integrate with new sync system

### 17.5 Risk Assessment

* **Low Risk**: Entity channel management (self-contained)
* **Medium Risk**: World manager operations (clear patterns exist)  
* **High Risk**: Entity migration TODOs (complex integration, affects core authority flow)

**Recommendation**: Complete Phases 1-3 before tackling migration TODOs to establish stable foundation.

---

*(Session snapshot appended automatically – remove/replace in later PRs.)*