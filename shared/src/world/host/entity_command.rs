use crate::{ComponentKind, EntityAuthStatus, GlobalEntity, HostEntity, RemoteEntity};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum EntityCommand {
    SpawnEntity(GlobalEntity),
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
