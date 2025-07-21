# Unified Stream‑Template Replication System

*(Refactor & Test‑Plan Spec)*

---

\## 0 · Preamble
This document is the single **source of truth** for the Naia networking refactor. It specifies **what** to build (functional spec), **why** (design rationale), and **how** to migrate from `entity_command_sender.rs` / `entity_message_receiver.rs` to the new *template‑driven* replication engine. Every invariant required for correctness is spelled out so that an automated coding agent can implement the system deterministically.

> **Scope guard** — The **packet header is frozen** (16‑bit `MessageIndex`, path depth ≤ 2). Only the **payload schema** (e.g. adding/changing `MsgKind` discriminants or payload bytes) may evolve.
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
| **MAX\_DEPTH**             | **Locked to 2**. Depth 0 = entity, depth 1 = component.                                                         |
| **Context**                | Value passed into `Template::on_apply(&Event,&mut Context)`; exposes `cmd_queue.push(Command)` API (push‑only). |
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

```rust
pub struct Event {
    pub seq: u16,                     // MessageIndex on the wire
    pub path: SmallVec<[PathSeg; 2]>, // EntityId [, ComponentKind]
    pub kind: MsgKind,               // part of payload schema (may evolve)
    pub payload: Bytes,               // opaque to Engine
}

#[derive(Copy,Clone,Eq,PartialEq)]
pub enum PathSeg {
    Entity(EntityId),
    Comp(ComponentKind),
}
```

\### 3.2 Runtime objects

| Symbol                       | Fields                                                 | Notes                                                                                                                                 |
| ---------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------- |
| **Stream**                   | `{ template_id, state, last_seq, spawn_seq, backlog }` | `backlog` is a `VecDeque<Event>` pre‑reserved to `MAX_IN_FLIGHT`. Push refuses if full (drops).                                       |
| **Engine\<T: RootTemplate>** | `HashMap<PathKey, Stream>`                             | `PathKey` = 64‑bit hash of `(depth, EntityId, CompKind?)`; collision‑safe because authoritative servers may store full tuple instead. |

\### 3.3 Templates & Callbacks (stable)

* `Template::on_apply(&Event, &mut Context)` — may **push** commands via `Context`, MUST NOT mutate `stream.state`.

---

\## 4 · Static Configuration (`config.rs`)

```rust
pub const MAX_IN_FLIGHT: u16 = 32_767;                    // half‑range window (must stay < 32_768)
pub const FLUSH_THRESHOLD: u16 = 65_536 - MAX_IN_FLIGHT;  // 32 769 — do NOT edit directly
pub const MAX_DEPTH: usize = 2;                           // compile‑time lock
```

> **Compile‑time guards** — `static_assert!(MAX_IN_FLIGHT < 32_768);`

---

\## 5 · Engine Semantics

1. **Locate stream** by `path` (depth ≤ `MAX_DEPTH`). Reject deeper paths.
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

* Maintain `global_seq: u64`, write `(global_seq as u16)` to packet.
* Block send when un‑ACKed window ≥ `MAX_IN_FLIGHT`; `debug_assert!` enforces in debug builds.
* No ordering/FSM logic on sender.

---

\## 8 · Refactor Roadmap

| Phase | Work item                                                                                   | Notes |
| ----- | ------------------------------------------------------------------------------------------- | ----- |
| P0    | Add enums/const tables & `config.rs`; include compile‑time asserts.                         |       |
| P1    | Implement `engine.rs` with guard‑band & Spawn/Despawn race rule.                            |       |
| P2    | Add concrete templates (`templates/entity_template.rs`, `templates/component_template.rs`). |       |
| P3    | Connect `EntityMessageReceiver` → `Engine`.                                                 |       |
| P4    | Trim `EntityCommandSender`, enforce window cap.                                             |       |
| P5    | Delete legacy code, add docs.                                                               |       |
| P6    | Integrate tests & CI.                                                                       |       |

---

\## 9 · Testing Strategy

| Layer        | Framework                 | Mandatory cases                                                                                                                                                                        |
| ------------ | ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Compile‑time | `static_assertions`       | Transition uniqueness; enum exhaustiveness; `MAX_DEPTH==2`; `MAX_IN_FLIGHT<32_768`.                                                                                                    |
| Unit         | `#[test]`                 | Each transition row; Spawn/Despawn race; guard‑band flush; wrap trace `65 530→65 535→0→1`.                                                                                             |
| Fuzz / Prop  | `proptest` / `cargo-fuzz` | Generate ≤ `MAX_IN_FLIGHT` causal traces, permute loss/dup/OOO, force wrap; replay causal order in new engine and assert `(path,state,last_seq,spawn_seq)` equality (backlog ignored). |

**TDD quick loop:**

Run only the sync-module tests with minimal noise:

```bash
RUSTFLAGS="-Awarnings" cargo test -q -p naia-shared --lib world::sync::tests
```

This compiles quietly (warnings suppressed), executes just the `shared/src/world/sync/tests` suite, and prints only the pass/fail summary.

---

\## 10 · Future‑Proof Notes

* New gameplay semantics can be expressed by authoring new templates; engine unchanged.
* `MAX_DEPTH` hard‑coded at 2; raising it requires packet header change, thus major version.
* Callbacks push commands via `Context`; they never mutate internal stream state.

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
        config.rs               // MAX_IN_FLIGHT, FLUSH_THRESHOLD … (+ static_assert!)
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

*Unit tests live **next to the code** under `sync/tests/` so `cargo test -p shared` keeps everything co-located.*

---

## 14 · Thin Facade: `EntityMessageReceiver`  *(NEW)*

`EntityMessageReceiver<E>` remains the public entry-point exposed to higher-level crates but internally delegates **all** ordering & FSM logic to `sync::Engine`.

````rust
pub struct EntityMessageReceiver<E: Copy + Hash + Eq> {
    inner: sync::Engine<templates::Root>, // new hotness
    // Legacy reliable channel stays for now
    receiver: ReliableReceiver<EntityMessage<E>>, // unchanged
}
````

### 14.1  Responsibilities

- Use `ReliableReceiver` to collect and **de-duplicate** incoming `EntityMessage<E>` packets (enforcing strict `MessageIndex` order).
- Translate each `(index, msg)` produced by the receiver into a `sync::Event`:
  * `seq`  ← `MessageIndex`
  * `path` ← `[PathSeg::Entity(id)]` _(depth 0)_ **or** `[Entity, CompKind]` _(depth 1)_
  * `kind` ← derived `MsgKind` (`Spawn`, `Despawn`, `Insert`, `Remove`, …)
  * `payload` ← empty `Bytes` for now – actual component diff bytes will be added later.
- Push the event into `inner.push(event)`.
- Each call to `receive_messages()` drains `inner.context().drain_commands()` and returns the vector, preserving current API.
- No per-entity state is stored here any more – memory leak fixed by engine’s tombstone GC.

### 14.2 Migration Strategy

* Step-zero: keep the existing `EntityMessageReceiver` tests GREEN by wrapping the new engine.  The behaviour must be byte-for-byte identical to legacy rules.
* Once confidence is gained, swap call-sites to use the richer `sync` API directly and delete the facade.

---

## 15 · Test-Driven Refactor Plan  *(NEW – supersedes §8 table)*

| Step | Description | Target Path | Test File | Status |
| ---- | ----------- | ----------- | --------- | ------ |
| **S0** | Create `sync/` module, add `config.rs` + compile-time asserts | `config.rs` | `tests/config.rs` | ☐ |
| **S1** | Implement pure functions: `sequence_greater_than`, `ahead(a,b)`; unit test all edge cases incl. wrap | `event.rs` | `tests/seq.rs` | ☐ |
| **S2** | Add `Event`, `PathSeg`, `PathKey`; verify hashing collision rules | `event.rs`, `path.rs` | `tests/event.rs` | ☐ |
| **S3** | Implement `Stream` data structure with backlog & guard-band logic | `stream.rs` | `tests/stream.rs` | ☐ |
| **S4** | Implement minimal `Engine` with `Spawn`/`Despawn` + backlog drain | `engine.rs` | `tests/engine_spawn.rs` | ☐ |
| **S5** | Port legacy `EntityMessageReceiver` tests to `sync/tests/legacy_parity.rs` running through facade | `sync/tests` | same | ☐ |
| **S6** | Flesh out component-level template + race rules | `templates/component_template.rs` | `tests/component.rs` | ☐ |
| **S7** | Fuzz harness exercising ≤ `MAX_IN_FLIGHT` traces | `tests/fuzz.rs` | - | ☐ |
| **S8** | Delete dead code paths, update docs, activate CI gate | repo-wide | - | ☐ |

*Check-box table will be updated during implementation PRs.*

---

## 16 · Additional Notes  *(NEW)*

* The new `sync` module is `#![no_std]`-compatible except for `HashMap`/`VecDeque`; gating with the existing `std` feature is acceptable.
* Use `smallvec::SmallVec` to avoid `Vec` allocations on the hot path.
* `PathKey` hashing uses `xxhash_rust::xxh3::xxh3_64_with_seed` with a compile-time salt to keep CPU predictable.
* `Context` exposes only **push** operations – no getters – enforcing unidirectional data-flow.
* Eventually `EntityCommandSender`'s window enforcement will consume `MAX_IN_FLIGHT` directly from `sync::config` to keep values DRY.

---

*(End of additions)*