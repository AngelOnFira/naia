use crate::{ComponentKind, EntityAuthStatus, EntityMessageType, GlobalEntity, HostEntity, RemoteEntity};

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

impl EntityCommand {
    pub fn entity(&self) -> GlobalEntity {
        match self {
            Self::Spawn(entity) => *entity,
            Self::Despawn(entity) => *entity,
            Self::InsertComponent(entity, _) => *entity,
            Self::RemoveComponent(entity, _) => *entity,
            Self::Publish(entity) => *entity,
            Self::Unpublish(entity) => *entity,
            Self::EnableDelegation(entity) => *entity,
            Self::DisableDelegation(entity) => *entity,
            Self::SetAuthority(entity, _) => *entity,
            Self::RequestAuthority(entity, _) => *entity,
            Self::ReleaseAuthority(entity) => *entity,
            Self::EnableDelegationResponse(entity) => *entity,
            Self::MigrateResponse(entity, _) => *entity,
        }
    }
    
    pub fn component_kind(&self) -> Option<ComponentKind> {
        match self {
            Self::InsertComponent(_, component_kind) => Some(*component_kind),
            Self::RemoveComponent(_, component_kind) => Some(*component_kind),
            _ => None,
        }
    }
    
    pub fn get_type(&self) -> EntityMessageType {
        match self {
            Self::Spawn(_) => EntityMessageType::Spawn,
            Self::Despawn(_) => EntityMessageType::Despawn,
            Self::InsertComponent(_, _) => EntityMessageType::InsertComponent,
            Self::RemoveComponent(_, _) => EntityMessageType::RemoveComponent,
            Self::Publish(_) => EntityMessageType::Publish,
            Self::Unpublish(_) => EntityMessageType::Unpublish,
            Self::EnableDelegation(_) => EntityMessageType::EnableDelegation,
            Self::DisableDelegation(_) => EntityMessageType::DisableDelegation,
            Self::SetAuthority(_, _) => EntityMessageType::SetAuthority,
            Self::RequestAuthority(_, _) => EntityMessageType::RequestAuthority,
            Self::ReleaseAuthority(_) => EntityMessageType::ReleaseAuthority,
            Self::EnableDelegationResponse(_) => EntityMessageType::EnableDelegationResponse,
            Self::MigrateResponse(_, _) => EntityMessageType::MigrateResponse,
        }
    }
}