use crate::{ComponentKind, EntityAuthStatus, EntityMessageType, GlobalEntity, HostEntity, RemoteEntity, Replicate, Tick};

pub enum EntityEvent {
    // ECS Lifecycle Events
    SpawnEntity(GlobalEntity),
    DespawnEntity(GlobalEntity),
    InsertComponent(GlobalEntity, ComponentKind),
    RemoveComponent(GlobalEntity, Box<dyn Replicate>),
    UpdateComponent(Tick, GlobalEntity, ComponentKind),

    PublishEntity(GlobalEntity),
    UnpublishEntity(GlobalEntity),
    EnableDelegationEntity(GlobalEntity),
    EnableDelegationEntityResponse(GlobalEntity),
    DisableDelegationEntity(GlobalEntity),
    EntityRequestAuthority(GlobalEntity, RemoteEntity),
    EntityReleaseAuthority(GlobalEntity),
    EntityUpdateAuthority(GlobalEntity, EntityAuthStatus),
    EntityMigrateResponse(GlobalEntity, HostEntity),
}

impl EntityEvent {

    pub fn to_type(&self) -> Option<EntityMessageType> {
        match self {
            Self::SpawnEntity(_) => Some(EntityMessageType::SpawnEntity),
            Self::DespawnEntity(_) => Some(EntityMessageType::DespawnEntity),
            Self::InsertComponent(_, _) => Some(EntityMessageType::InsertComponent),
            Self::RemoveComponent(_, _) => Some(EntityMessageType::RemoveComponent),
            Self::PublishEntity(_) => Some(EntityMessageType::PublishEntity),
            Self::UnpublishEntity(_) => Some(EntityMessageType::UnpublishEntity),
            Self::EnableDelegationEntity(_) => Some(EntityMessageType::EnableDelegationEntity),
            Self::EnableDelegationEntityResponse(_) => Some(EntityMessageType::EnableDelegationEntityResponse),
            Self::DisableDelegationEntity(_) => Some(EntityMessageType::DisableDelegationEntity),
            Self::EntityRequestAuthority(_, _) => Some(EntityMessageType::RequestAuthority),
            Self::EntityReleaseAuthority(_) => Some(EntityMessageType::ReleaseAuthority),
            Self::EntityUpdateAuthority(_, _) => Some(EntityMessageType::UpdateAuthority),
            Self::EntityMigrateResponse(_, _) => Some(EntityMessageType::EntityMigrateResponse),
            Self::UpdateComponent(_, _, _) => None, // UpdateComponent is not a message type
        }
    }

    pub fn entity(&self) -> GlobalEntity {
        match self {
            Self::SpawnEntity(entity) => *entity,
            Self::DespawnEntity(entity) => *entity,
            Self::InsertComponent(entity, _) => *entity,
            Self::RemoveComponent(entity, _) => *entity,
            Self::UpdateComponent(_, entity, _) => *entity,
            Self::PublishEntity(entity) => *entity,
            Self::UnpublishEntity(entity) => *entity,
            Self::EnableDelegationEntity(entity) => *entity,
            Self::EnableDelegationEntityResponse(entity) => *entity,
            Self::DisableDelegationEntity(entity) => *entity,
            Self::EntityRequestAuthority(entity, _) => *entity,
            Self::EntityReleaseAuthority(entity) => *entity,
            Self::EntityUpdateAuthority(entity, _) => *entity,
            Self::EntityMigrateResponse(entity, _) => *entity,
        }
    }

    pub fn log(&self) -> String {
        let entity = self.entity();
        if let Some(ev_type) = self.to_type() {
            format!("{:?} {:?}", ev_type, entity)
        } else {
            format!("UpdateComponent {:?}", entity)
        }
    }
}
