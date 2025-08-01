use crate::{EntityAuthStatus, HostEntity, RemoteEntity, world::component::component_kinds::ComponentKind, EntityMessageType};

// Keep E here! TODO: remove
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum EntityMessage<E: Copy + Eq + PartialEq> {
    SpawnEntity(E),
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
            Self::SpawnEntity(entity) => Some(*entity),
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
    
    pub fn component_kind(&self) -> Option<ComponentKind> {
        match self {
            Self::InsertComponent(_, component_kind) => Some(*component_kind),
            Self::RemoveComponent(_, component_kind) => Some(*component_kind),
            _ => None,
        }
    }
    
    pub fn strip_entity(self) -> EntityMessage<()> {
        match self {
            Self::SpawnEntity(_) => EntityMessage::SpawnEntity(()),
            Self::DespawnEntity(_) => EntityMessage::DespawnEntity(()),
            Self::InsertComponent(_, component_kind) => EntityMessage::InsertComponent((), component_kind),
            Self::RemoveComponent(_, component_kind) => EntityMessage::RemoveComponent((), component_kind),
            Self::PublishEntity(_) => EntityMessage::PublishEntity(()),
            Self::UnpublishEntity(_) => EntityMessage::UnpublishEntity(()),
            Self::EnableDelegationEntity(_) => EntityMessage::EnableDelegationEntity(()),
            Self::EnableDelegationEntityResponse(_) => EntityMessage::EnableDelegationEntityResponse(()),
            Self::DisableDelegationEntity(_) => EntityMessage::DisableDelegationEntity(()),
            Self::EntityRequestAuthority(_, other_entity) => EntityMessage::EntityRequestAuthority((), other_entity),
            Self::EntityReleaseAuthority(_) => EntityMessage::EntityReleaseAuthority(()),
            Self::EntityUpdateAuthority(_, status) => EntityMessage::EntityUpdateAuthority((), status),
            Self::EntityMigrateResponse(_, other_entity) => EntityMessage::EntityMigrateResponse((), other_entity),
            Self::Noop => panic!("Cannot strip entity from a Noop message"),
        }
    }
    
    pub fn get_type(&self) -> EntityMessageType {
        match self {
            Self::SpawnEntity(_) => EntityMessageType::SpawnEntity,
            Self::DespawnEntity(_) => EntityMessageType::DespawnEntity,
            Self::InsertComponent(_, _) => EntityMessageType::InsertComponent,
            Self::RemoveComponent(_, _) => EntityMessageType::RemoveComponent,
            Self::PublishEntity(_) => EntityMessageType::PublishEntity,
            Self::UnpublishEntity(_) => EntityMessageType::UnpublishEntity,
            Self::EnableDelegationEntity(_) => EntityMessageType::EnableDelegationEntity,
            Self::EnableDelegationEntityResponse(_) => EntityMessageType::EnableDelegationEntityResponse,
            Self::DisableDelegationEntity(_) => EntityMessageType::DisableDelegationEntity,
            Self::EntityRequestAuthority(_, _) => EntityMessageType::RequestAuthority,
            Self::EntityReleaseAuthority(_) => EntityMessageType::ReleaseAuthority,
            Self::EntityUpdateAuthority(_, _) => EntityMessageType::UpdateAuthority,
            Self::EntityMigrateResponse(_, _) => EntityMessageType::EntityMigrateResponse,
            Self::Noop => EntityMessageType::Noop,
        }
    }
}

impl EntityMessage<()> {
    pub fn with_entity<E: Copy + Eq + PartialEq>(self, entity: E) -> EntityMessage<E> {
        match self {
            EntityMessage::SpawnEntity(_) => EntityMessage::SpawnEntity(entity),
            EntityMessage::DespawnEntity(_) => EntityMessage::DespawnEntity(entity),
            EntityMessage::InsertComponent(_, component_kind) => EntityMessage::InsertComponent(entity, component_kind),
            EntityMessage::RemoveComponent(_, component_kind) => EntityMessage::RemoveComponent(entity, component_kind),
            EntityMessage::PublishEntity(_) => EntityMessage::PublishEntity(entity),
            EntityMessage::UnpublishEntity(_) => EntityMessage::UnpublishEntity(entity),
            EntityMessage::EnableDelegationEntity(_) => EntityMessage::EnableDelegationEntity(entity),
            EntityMessage::EnableDelegationEntityResponse(_) => EntityMessage::EnableDelegationEntityResponse(entity),
            EntityMessage::DisableDelegationEntity(_) => EntityMessage::DisableDelegationEntity(entity),
            EntityMessage::EntityRequestAuthority(_, other_entity) => EntityMessage::EntityRequestAuthority(entity, other_entity),
            EntityMessage::EntityReleaseAuthority(_) => EntityMessage::EntityReleaseAuthority(entity),
            EntityMessage::EntityUpdateAuthority(_, status) => EntityMessage::EntityUpdateAuthority(entity, status),
            EntityMessage::EntityMigrateResponse(_, other_entity) => EntityMessage::EntityMigrateResponse(entity, other_entity),
            EntityMessage::Noop => panic!("Cannot add entity to a Noop message"),
        }
    }
}
