use crate::{ComponentKind, EntityAuthStatus, GlobalEntity, HostEntity, RemoteEntity};

// command to sync entities from host -> remote
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum EntityCommand {
    Spawn(GlobalEntity),
    Despawn(GlobalEntity),
    InsertComponent(GlobalEntity, ComponentKind),
    RemoveComponent(GlobalEntity, ComponentKind),
    
    // Former SystemChannel messages
    Publish(GlobalEntity),
    Unpublish(GlobalEntity),
    EnableDelegation(GlobalEntity),
    DisableDelegation(GlobalEntity),
    SetAuthority(GlobalEntity, EntityAuthStatus),

    // These aren't commands, they are something else
    RequestAuthority(GlobalEntity, RemoteEntity),
    ReleaseAuthority(GlobalEntity),
    EnableDelegationResponse(GlobalEntity),
    MigrateResponse(GlobalEntity, HostEntity),
}