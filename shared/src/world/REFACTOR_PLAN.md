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
| Fuzz / Prop  | `proptest` / `cargo‑fuzz` | Generate ≤ `MAX_IN_FLIGHT` causal traces, permute loss/dup/OOO, force wrap; replay causal order in new engine and assert `(path,state,last_seq,spawn_seq)` equality (backlog ignored). |

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