use crate::{ComponentKind, EntityAuthStatus, GlobalEntity, HostEntity, RemoteEntity};

#[derive(Clone, PartialEq, Eq)]
pub enum EntityCommand {
    SpawnEntity(GlobalEntity, Vec<ComponentKind>),
    DespawnEntity(GlobalEntity),
    InsertComponent(GlobalEntity, ComponentKind),
    RemoveComponent(GlobalEntity, ComponentKind),
    
    // Former SystemChannel messages
    PublishEntity(GlobalEntity),
    UnpublishEntity(GlobalEntity),
    EnableDelegationEntity(GlobalEntity),
    EnableDelegationEntityResponse(GlobalEntity),
    DisableDelegationEntity(GlobalEntity),
    RequestAuthority(GlobalEntity, RemoteEntity),
    ReleaseAuthority(GlobalEntity),
    UpdateAuthority(GlobalEntity, EntityAuthStatus),
    EntityMigrateResponse(GlobalEntity, HostEntity),
}
