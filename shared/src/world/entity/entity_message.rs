use crate::{EntityAuthStatus, HostEntity, RemoteEntity, world::component::component_kinds::ComponentKind};

// Keep E here! TODO: remove
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum EntityMessage<E: Copy + Eq + PartialEq> {
    SpawnEntity(E, Vec<ComponentKind>),
    DespawnEntity(E),
    InsertComponent(E, ComponentKind),
    RemoveComponent(E, ComponentKind),
    PublishEntity(E),
    UnpublishEntity(E),
    EnableDelegationEntity(E),
    EnableDelegationEntityResponse(E),
    DisableDelegationEntity(E),
    EntityRequestAuthority(E, RemoteEntity),
    EntityReleaseAuthority(E),
    EntityUpdateAuthority(E, EntityAuthStatus),
    EntityMigrateResponse(E, HostEntity),

    Noop,
}

impl<E: Copy + Eq + PartialEq> EntityMessage<E> {
    pub fn entity(&self) -> Option<E> {
        match self {
            Self::SpawnEntity(entity, _) => Some(*entity),
            Self::DespawnEntity(entity) => Some(*entity),
            Self::InsertComponent(entity, _) => Some(*entity),
            Self::RemoveComponent(entity, _) => Some(*entity),
            Self::PublishEntity(entity) => Some(*entity),
            Self::UnpublishEntity(entity) => Some(*entity),
            Self::EnableDelegationEntity(entity) => Some(*entity),
            Self::EnableDelegationEntityResponse(entity) => Some(*entity),
            Self::DisableDelegationEntity(entity) => Some(*entity),
            Self::EntityRequestAuthority(entity, _) => Some(*entity),
            Self::EntityReleaseAuthority(entity) => Some(*entity),
            Self::EntityUpdateAuthority(entity, _) => Some(*entity),
            Self::EntityMigrateResponse(entity, _) => Some(*entity),
            Self::Noop => None,
        }
    }
}
