use bevy_ecs::schedule::SystemSet;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct HandleTickEvents;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct HandleWorldEvents;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct ReceivePackets;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct HostSyncChangeTracking;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct BeforeHostSyncChangeTracking;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct SendPackets;
