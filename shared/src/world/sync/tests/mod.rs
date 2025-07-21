// Tests for the new sync module
#![cfg(test)]

use crate::world::sync::{Engine, Event, PathSeg, MsgKind};

#[test]
fn engine_basic() {
    struct DummyRoot;
    let mut engine: Engine<DummyRoot> = Engine::new();

    // Feed two simple events
    let ev1 = Event::new(1, vec![PathSeg::Entity(1)], MsgKind::Spawn, vec![]);
    let ev2 = Event::new(2, vec![PathSeg::Entity(1), PathSeg::Comp(5)], MsgKind::Insert, vec![]);
    engine.push(ev1);
    engine.push(ev2);

    // Drain context – stub engine emits nothing for now
    let out = engine.context().drain();
    assert!(out.is_empty(), "Stub engine should not output events yet");
} 