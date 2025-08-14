use crate::{EntityAuthStatus, HostEntity, RemoteEntity, world::component::component_kinds::ComponentKind, EntityMessageType, EntityEvent, OwnedLocalEntity, LocalEntityMap};

// Raw entity sync messages sent over the wire
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum EntityMessage<E: Copy + Eq + PartialEq> {
    Spawn(E),
    Despawn(E),
    InsertComponent(E, ComponentKind),
    RemoveComponent(E, ComponentKind),
    Publish(E),
    Unpublish(E),
    EnableDelegation(E),
    DisableDelegation(E),
    SetAuthority(E, EntityAuthStatus),
    
    // These are not commands, they are something else
    RequestAuthority(E, RemoteEntity),
    ReleaseAuthority(OwnedLocalEntity),
    EnableDelegationResponse(E),
    MigrateResponse(E, HostEntity),

    Noop,
}

impl<E: Copy + Eq + PartialEq> EntityMessage<E> {
    pub fn entity(&self) -> Option<E> {
        match self {
            Self::Spawn(entity) => Some(*entity),
            Self::Despawn(entity) => Some(*entity),
            Self::InsertComponent(entity, _) => Some(*entity),
            Self::RemoveComponent(entity, _) => Some(*entity),
            Self::Publish(entity) => Some(*entity),
            Self::Unpublish(entity) => Some(*entity),
            Self::EnableDelegation(entity) => Some(*entity),
            Self::EnableDelegationResponse(entity) => Some(*entity),
            Self::DisableDelegation(entity) => Some(*entity),
            Self::RequestAuthority(entity, _) => Some(*entity),
            Self::ReleaseAuthority(_) => panic!("EntityReleaseAuthority should not call `entity()`"),
            Self::SetAuthority(entity, _) => Some(*entity),
            Self::MigrateResponse(entity, _) => Some(*entity),
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
            Self::Spawn(_) => EntityMessage::Spawn(()),
            Self::Despawn(_) => EntityMessage::Despawn(()),
            Self::InsertComponent(_, component_kind) => EntityMessage::InsertComponent((), component_kind),
            Self::RemoveComponent(_, component_kind) => EntityMessage::RemoveComponent((), component_kind),
            Self::Publish(_) => EntityMessage::Publish(()),
            Self::Unpublish(_) => EntityMessage::Unpublish(()),
            Self::EnableDelegation(_) => EntityMessage::EnableDelegation(()),
            Self::EnableDelegationResponse(_) => EntityMessage::EnableDelegationResponse(()),
            Self::DisableDelegation(_) => EntityMessage::DisableDelegation(()),
            Self::RequestAuthority(_, other_entity) => EntityMessage::RequestAuthority((), other_entity),
            Self::ReleaseAuthority(_) => panic!("EntityReleaseAuthority should not call `strip_entity()`"),
            Self::SetAuthority(_, status) => EntityMessage::SetAuthority((), status),
            Self::MigrateResponse(_, other_entity) => EntityMessage::MigrateResponse((), other_entity),
            Self::Noop => panic!("Cannot strip entity from a Noop message"),
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
            Self::EnableDelegationResponse(_) => EntityMessageType::EnableDelegationResponse,
            Self::DisableDelegation(_) => EntityMessageType::DisableDelegation,
            Self::RequestAuthority(_, _) => EntityMessageType::RequestAuthority,
            Self::ReleaseAuthority(_) => EntityMessageType::ReleaseAuthority,
            Self::SetAuthority(_, _) => EntityMessageType::SetAuthority,
            Self::MigrateResponse(_, _) => EntityMessageType::MigrateResponse,
            Self::Noop => EntityMessageType::Noop,
        }
    }
}

impl EntityMessage<()> {
    pub fn with_entity<E: Copy + Eq + PartialEq>(self, entity: E) -> EntityMessage<E> {
        match self {
            EntityMessage::Spawn(_) => EntityMessage::Spawn(entity),
            EntityMessage::Despawn(_) => EntityMessage::Despawn(entity),
            EntityMessage::InsertComponent(_, component_kind) => EntityMessage::InsertComponent(entity, component_kind),
            EntityMessage::RemoveComponent(_, component_kind) => EntityMessage::RemoveComponent(entity, component_kind),
            EntityMessage::Publish(_) => EntityMessage::Publish(entity),
            EntityMessage::Unpublish(_) => EntityMessage::Unpublish(entity),
            EntityMessage::EnableDelegation(_) => EntityMessage::EnableDelegation(entity),
            EntityMessage::EnableDelegationResponse(_) => EntityMessage::EnableDelegationResponse(entity),
            EntityMessage::DisableDelegation(_) => EntityMessage::DisableDelegation(entity),
            EntityMessage::RequestAuthority(_, other_entity) => EntityMessage::RequestAuthority(entity, other_entity),
            EntityMessage::ReleaseAuthority(_) => panic!("EntityReleaseAuthority should not call `with_entity()`"),
            EntityMessage::SetAuthority(_, status) => EntityMessage::SetAuthority(entity, status),
            EntityMessage::MigrateResponse(_, other_entity) => EntityMessage::MigrateResponse(entity, other_entity),
            EntityMessage::Noop => panic!("Cannot add entity to a Noop message"),
        }
    }
}

impl EntityMessage<RemoteEntity> {
    
    pub fn to_host_message(self) -> EntityMessage<HostEntity> {
        match self {
            EntityMessage::EnableDelegationResponse(entity) => {
                EntityMessage::EnableDelegationResponse(entity.to_host())
            }
            EntityMessage::MigrateResponse(entity, other_entity) => {
                EntityMessage::MigrateResponse(entity.to_host(), other_entity)
            }
            EntityMessage::RequestAuthority(entity, other_entity) => {
                EntityMessage::RequestAuthority(entity.to_host(), other_entity)
            }
            EntityMessage::ReleaseAuthority(_) => panic!("EntityReleaseAuthority should not call `to_host_message()`"),
            msg => {
                panic!("No reason to convert message {:?} to HostEntity", msg);
            }
        }
    }
    
    pub fn to_event(self, local_entity_map: &LocalEntityMap) -> EntityEvent {
        let remote_entity = self.entity().unwrap();
        let global_entity = *(local_entity_map.global_entity_from_remote(&remote_entity).unwrap());
        match self {
            EntityMessage::Publish(_) => EntityEvent::Publish(global_entity),
            EntityMessage::Unpublish(_) => EntityEvent::Unpublish(global_entity),
            EntityMessage::EnableDelegation(_) => EntityEvent::EnableDelegation(global_entity),
            EntityMessage::EnableDelegationResponse(_) => EntityEvent::EnableDelegationResponse(global_entity),
            EntityMessage::DisableDelegation(_) => EntityEvent::DisableDelegation(global_entity),
            EntityMessage::RequestAuthority(_, other_entity) => EntityEvent::RequestAuthority(global_entity, other_entity),
            EntityMessage::ReleaseAuthority(_) => EntityEvent::ReleaseAuthority(global_entity),
            EntityMessage::SetAuthority(_, status) => EntityEvent::SetAuthority(global_entity, status),
            EntityMessage::MigrateResponse(_, other_entity) => EntityEvent::MigrateResponse(global_entity, other_entity),
            EntityMessage::Spawn(_) | EntityMessage::Despawn(_) |
            EntityMessage::InsertComponent(_, _) | EntityMessage::RemoveComponent(_, _) => panic!("Handled elsewhere"),
            EntityMessage::Noop => panic!("Cannot convert Noop message to an event"),
        }
    }
}

impl EntityMessage<HostEntity> {
    pub fn to_event(self, local_entity_map: &LocalEntityMap) -> EntityEvent {
        let host_entity = self.entity().unwrap();
        let global_entity = *(local_entity_map.global_entity_from_host(&host_entity).unwrap());
        match self {
            EntityMessage::Publish(_) => EntityEvent::Publish(global_entity),
            EntityMessage::Unpublish(_) => EntityEvent::Unpublish(global_entity),
            EntityMessage::EnableDelegation(_) => EntityEvent::EnableDelegation(global_entity),
            EntityMessage::EnableDelegationResponse(_) => EntityEvent::EnableDelegationResponse(global_entity),
            EntityMessage::DisableDelegation(_) => EntityEvent::DisableDelegation(global_entity),
            EntityMessage::RequestAuthority(_, other_entity) => EntityEvent::RequestAuthority(global_entity, other_entity),
            EntityMessage::ReleaseAuthority(_) => EntityEvent::ReleaseAuthority(global_entity),
            EntityMessage::SetAuthority(_, status) => EntityEvent::SetAuthority(global_entity, status),
            EntityMessage::MigrateResponse(_, other_entity) => EntityEvent::MigrateResponse(global_entity, other_entity),
            EntityMessage::Spawn(_) | EntityMessage::Despawn(_) |
            EntityMessage::InsertComponent(_, _) | EntityMessage::RemoveComponent(_, _) => panic!("Handled elsewhere"),
            EntityMessage::Noop => panic!("Cannot convert Noop message to an event"),
        }
    }
}