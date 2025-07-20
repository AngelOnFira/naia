use crate::{ComponentKind, EntityAuthStatus, GlobalEntity, HostEntity, RemoteEntity, Replicate, Tick};

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
