use thiserror::Error;

/// Errors that can occur during remote world management operations
///
/// RemoteWorldManager handles client-side entity and component replication.
/// These errors protect against malformed network data, state machine violations,
/// and internal inconsistencies that could crash the client.
#[derive(Debug, Clone, Error)]
pub enum RemoteWorldError {
    // Component Data Errors (Network Data Issues)

    /// Component data missing from incoming components map during spawn
    #[error("Component {component_kind} missing from spawn data for entity {entity_id}")]
    ComponentDataMissingDuringSpawn {
        entity_id: String,
        component_kind: String,
    },

    /// Component data missing from incoming components map during insert
    #[error("Component {component_kind} missing from insert data for entity {entity_id}")]
    ComponentDataMissingDuringInsert {
        entity_id: String,
        component_kind: String,
    },

    /// Component update split into neither waiting nor ready parts (malformed update)
    #[error("Malformed component update for entity {entity_id}: split into neither waiting nor ready parts")]
    MalformedComponentUpdate {
        entity_id: String,
        component_kind: String,
    },

    // Waitlist Internal Consistency Errors

    /// Waitlist handle missing from required entities map
    #[error("Waitlist handle {handle} not found in required entities map")]
    WaitlistHandleMissing {
        handle: u16,
    },

    /// Component field missing from update waitlist map
    #[error("Component field missing from update waitlist for entity {entity_id}, component {component_kind}")]
    UpdateWaitlistMapInconsistency {
        entity_id: String,
        component_kind: String,
    },

    /// Handle TTL queue empty when expected to have items
    #[error("Handle TTL queue unexpectedly empty during timeout check")]
    HandleTtlQueueEmpty,

    /// Item missing from waitlist store during collection
    #[error("Item with handle {handle} missing from waitlist store")]
    WaitlistItemMissing {
        handle: u16,
    },

    // Entity State Errors

    /// Entity not found when expected to exist
    #[error("Entity {entity_id} not found in remote world")]
    EntityNotFound {
        entity_id: String,
    },

    /// Operation requires entity to be spawned first
    #[error("Cannot {operation} on entity {entity_id} - entity not spawned yet")]
    EntityNotSpawned {
        entity_id: String,
        operation: &'static str,
    },
}
