use naia_serde::SerdeInternal;

use crate::{ComponentKind, EntityMessage};

// Enum used as a shared network protocol, representing various message types
// related to Entities/Components
#[derive(Copy, PartialEq, Clone, SerdeInternal, Debug)]
pub enum EntityMessageType {
    // Action indicating an Entity to be created
    SpawnEntity,
    // Action indicating an Entity to be deleted
    DespawnEntity,
    // Action indicating a Component to be added to an Entity
    InsertComponent,
    // Action indicating a Component to be deleted
    RemoveComponent,
    // Former SystemChannel messages - now unified in EntityActionType
    // Action indicating an Entity to be published
    PublishEntity,
    // Action indicating an Entity to be unpublished
    UnpublishEntity,
    // Action indicating delegation to be enabled for an Entity
    EnableDelegationEntity,
    // Action indicating delegation enable response
    EnableDelegationEntityResponse,
    // Action indicating delegation to be disabled for an Entity
    DisableDelegationEntity,
    // Action requesting authority for an Entity
    RequestAuthority,
    // Action releasing authority for an Entity
    ReleaseAuthority,
    // Action updating authority status for an Entity
    UpdateAuthority,
    // Action responding to entity migration
    EntityMigrateResponse,
    // Action indicating a non-operation
    Noop,
}

impl EntityMessageType {
    pub fn with_component_kind(&self, component_kind: &ComponentKind) -> EntityMessage<()> {
        match self {
            Self::InsertComponent => EntityMessage::InsertComponent((), *component_kind),
            Self::RemoveComponent => EntityMessage::RemoveComponent((), *component_kind),
            t => panic!("Cannot apply component kind to message type: {:?}", t),
        }
    }
}