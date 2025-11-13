use thiserror::Error;

/// Errors that can occur during WorldChannel operations
///
/// WorldChannel is the most critical module in naia - it handles ALL entity replication
/// between server and clients. These errors protect against state machine violations,
/// packet manipulation, and out-of-order operations that could crash the server.
#[derive(Debug, Clone, Error)]
pub enum WorldChannelError {
    // Entity Spawn/Despawn Errors

    /// Attempted to spawn an entity that already exists in the channel
    #[error("Entity {entity_id} already spawned on channel")]
    EntityAlreadySpawned { entity_id: String },

    /// Attempted to operate on an entity that doesn't exist in the channel
    #[error("Entity {entity_id} not found on channel")]
    EntityNotFound { entity_id: String },

    /// Attempted to despawn entity but it has no channel
    #[error("Entity {entity_id} has no channel")]
    EntityHasNoChannel { entity_id: String },

    /// Attempted to despawn an entity that is already despawning
    #[error("Entity {entity_id} already despawned or is despawning")]
    EntityAlreadyDespawned { entity_id: String },

    /// Attempted to perform an operation on an entity that hasn't been spawned
    #[error("Cannot {operation} on entity {entity_id} - entity not spawned")]
    EntityNotSpawned {
        entity_id: String,
        operation: &'static str,
    },

    // Component Errors

    /// Component not found on entity during remove operation
    #[error("Component {component_kind} not found on entity {entity_id}")]
    ComponentNotFound {
        entity_id: String,
        component_kind: String,
    },

    /// Component already exists on entity during insert operation
    #[error("Component {component_kind} already exists on entity {entity_id}")]
    ComponentAlreadyExists {
        entity_id: String,
        component_kind: String,
    },

    // Remote Entity Tracking Errors

    /// Remote entity is already being tracked
    #[error("Remote entity {entity_id} already tracked")]
    RemoteEntityAlreadyTracked { entity_id: String },

    /// Remote entity is not being tracked
    #[error("Remote entity {entity_id} not tracked")]
    RemoteEntityNotTracked { entity_id: String },

    /// Attempted to track entity component before tracking the entity itself
    #[error("Cannot track component on entity {entity_id} - must spawn entity first")]
    MustSpawnBeforeTracking { entity_id: String },

    // Remote Action Processing Errors (State Machine Violations)

    /// Remote spawn received but entity already exists in remote world
    #[error("Cannot spawn entity {entity_id} in remote world - already exists")]
    RemoteEntityAlreadyExists { entity_id: String },

    /// Remote despawn received but entity doesn't exist in remote world
    #[error("Cannot despawn entity {entity_id} from remote world - not found")]
    RemoteEntityNotFoundForDespawn { entity_id: String },

    /// Remote spawn/despawn received but entity channel is in wrong state
    #[error("Entity {entity_id} channel in wrong state for {operation} - expected {expected_state}, got {actual_state}")]
    InvalidChannelState {
        entity_id: String,
        operation: &'static str,
        expected_state: &'static str,
        actual_state: &'static str,
    },

    /// Remote component operation received for non-existent entity
    #[error("Cannot {operation} component {component_kind} on entity {entity_id} - entity not in remote world")]
    RemoteComponentEntityNotFound {
        entity_id: String,
        component_kind: String,
        operation: &'static str,
    },

    /// Remote component already exists during insert
    #[error("Cannot insert component {component_kind} on entity {entity_id} - component already exists in remote world")]
    RemoteComponentAlreadyExists {
        entity_id: String,
        component_kind: String,
    },

    /// Remote component not found during remove
    #[error("Cannot remove component {component_kind} from entity {entity_id} - component not found in remote world")]
    RemoteComponentNotFound {
        entity_id: String,
        component_kind: String,
    },

    /// Remote component operation when entity channel doesn't exist
    #[error("Cannot {operation} component {component_kind} on entity {entity_id} - entity channel not initialized")]
    RemoteComponentNoChannel {
        entity_id: String,
        component_kind: String,
        operation: &'static str,
    },

    /// Remote component insert when component channel not initialized
    #[error("Cannot insert component {component_kind} on entity {entity_id} - component channel not initialized")]
    RemoteComponentChannelNotInitialized {
        entity_id: String,
        component_kind: String,
    },

    /// Remote component remove when component is not in removing state
    #[error("Cannot remove component {component_kind} from entity {entity_id} - component not in removing state")]
    RemoteComponentNotRemoving {
        entity_id: String,
        component_kind: String,
    },

    // CheckedMap/CheckedSet Internal Structure Errors

    /// Duplicate key insertion in checked map
    #[error("Duplicate key in checked map: {key}")]
    DuplicateMapKey { key: String },

    /// Key not found during removal in checked map
    #[error("Key not found in checked map: {key}")]
    MapKeyNotFound { key: String },

    /// Duplicate value insertion in checked set
    #[error("Duplicate value in checked set: {value}")]
    DuplicateSetValue { value: String },

    /// Value not found during removal in checked set
    #[error("Value not found in checked set: {value}")]
    SetValueNotFound { value: String },

    // EntityChannel State Machine Errors

    /// Attempted to complete spawning on entity not in spawning state
    #[error("Cannot complete spawning - entity {entity_id} not in spawning state")]
    EntityNotSpawningState { entity_id: String },

    /// Attempted to complete spawning when entity has pending components
    #[error("Entity {entity_id} has pending components during spawn complete")]
    EntityHasPendingComponents { entity_id: String },

    /// Attempted to despawn entity that is not in spawned state
    #[error("Cannot despawn entity {entity_id} - not in spawned state (must be spawned first)")]
    EntityNotYetSpawned { entity_id: String },

    /// Attempted to insert component into entity that is not spawned
    #[error("Cannot insert component {component_kind} into entity {entity_id} - entity not spawned")]
    ComponentInsertEntityNotSpawned {
        entity_id: String,
        component_kind: String,
    },

    /// Attempted to remove component that is currently being inserted
    #[error("Cannot remove component {component_kind} from entity {entity_id} - component is still being inserted")]
    ComponentRemoveWhileInserting {
        entity_id: String,
        component_kind: String,
    },

    /// Component operation completed in wrong state
    #[error("Cannot complete component {operation} - component in wrong state: expected {expected}, got {actual}")]
    ComponentOperationWrongState {
        operation: &'static str,
        expected: &'static str,
        actual: &'static str,
    },

    /// Attempted to send message after authority was released
    #[error("Cannot send message for entity {entity_id} - authority was released")]
    AuthorityReleased { entity_id: String },

    /// Failed to send authority release message
    #[error("Failed to send auth release message for entity {entity_id} - channel dropped while waiting")]
    AuthReleaseMessageFailed { entity_id: String },

    // Packet Overflow Errors (HostWorldWriter)

    /// Entity spawn packet overflow - entity with components too large for MTU
    #[error("Entity spawn packet overflow: entity {entity_id} with components [{component_names}] requires {bits_needed} bits, but packet only has {bits_free} bits available")]
    EntitySpawnPacketOverflow {
        entity_id: String,
        component_names: String,
        bits_needed: u32,
        bits_free: u32,
    },

    /// Component insertion packet overflow - component too large for MTU
    #[error("Component insert packet overflow: component {component_kind} on entity {entity_id} requires {bits_needed} bits, but packet only has {bits_free} bits available")]
    ComponentInsertPacketOverflow {
        entity_id: String,
        component_kind: String,
        bits_needed: u32,
        bits_free: u32,
    },

    /// Component update packet overflow - component update too large for MTU
    #[error("Component update packet overflow: component {component_kind} requires {bits_needed} bits, but packet only has {bits_free} bits available")]
    ComponentUpdatePacketOverflow {
        component_kind: String,
        bits_needed: u32,
        bits_free: u32,
    },

    /// Generic action packet overflow (should never happen)
    #[error("Action packet overflow: action requires {bits_needed} bits, but packet only has {bits_free} bits available")]
    ActionPacketOverflow {
        bits_needed: u32,
        bits_free: u32,
    },

    /// Component not found during packet write operation
    #[error("Component {component_kind} not found on entity {entity_id} during packet write")]
    ComponentNotFoundDuringWrite {
        entity_id: String,
        component_kind: String,
    },

    // MutChannel/DiffHandler Errors

    /// RwLock is already held on current thread (reentrant lock attempt)
    #[error("RwLock is already held on current thread")]
    RwLockReentrant,

    /// Receiver not found for entity/component
    #[error("Receiver not found for entity {entity_id}, component {component_kind}")]
    ReceiverNotFound {
        entity_id: String,
        component_kind: String,
    },

    /// Component not registered in global handler
    #[error("Component {component_kind} for entity {entity_id} not registered")]
    ComponentNotRegistered {
        entity_id: String,
        component_kind: String,
    },

    /// Component already registered in global handler
    #[error("Component {component_kind} for entity {entity_id} already registered")]
    ComponentAlreadyRegistered {
        entity_id: String,
        component_kind: String,
    },
}
