use std::{
    collections::HashMap,
    hash::Hash,
    net::SocketAddr,
    sync::{Arc, RwLock, RwLockReadGuard},
};

use crate::{ComponentKind, DiffMask, GlobalWorldManagerType};

use super::{error::WorldChannelError, global_diff_handler::GlobalDiffHandler, mut_channel::MutReceiver};

#[derive(Clone)]
pub struct UserDiffHandler<E: Copy + Eq + Hash> {
    receivers: HashMap<(E, ComponentKind), MutReceiver>,
    global_diff_handler: Arc<RwLock<GlobalDiffHandler<E>>>,
}

impl<E: Copy + Eq + Hash> UserDiffHandler<E> {
    pub fn new(global_world_manager: &dyn GlobalWorldManagerType<E>) -> Self {
        Self {
            receivers: HashMap::new(),
            global_diff_handler: global_world_manager.diff_handler(),
        }
    }

    // Component Registration
    pub fn register_component(
        &mut self,
        address: &Option<SocketAddr>,
        entity: &E,
        component_kind: &ComponentKind,
    ) {
        let Ok(global_handler) = self.global_diff_handler.as_ref().read() else {
            panic!("Be sure you can get self.global_diff_handler before calling this!");
        };
        let receiver = global_handler
            .receiver(address, entity, component_kind)
            .expect("GlobalDiffHandler has not yet registered this Component");
        self.receivers.insert((*entity, *component_kind), receiver);
    }

    pub fn deregister_component(&mut self, entity: &E, component_kind: &ComponentKind) {
        self.receivers.remove(&(*entity, *component_kind));
    }

    pub fn has_component(&self, entity: &E, component: &ComponentKind) -> bool {
        self.receivers.contains_key(&(*entity, *component))
    }

    // Diff masks
    pub fn diff_mask(
        &self,
        entity: &E,
        component_kind: &ComponentKind,
    ) -> RwLockReadGuard<DiffMask> {
        let Some(receiver) = self.receivers.get(&(*entity, *component_kind)) else {
            panic!("Should not call this unless we're sure there's a receiver");
        };
        return receiver.mask();
    }

    //    pub fn has_diff_mask(&self, component_key: &ComponentKey) -> bool {
    //        return self.receivers.contains_key(component_key);
    //    }

    pub fn diff_mask_is_clear(&self, entity: &E, component_kind: &ComponentKind) -> bool {
        let Some(receiver) = self.receivers.get(&(*entity, *component_kind)) else {
            panic!("Should not call this unless we're sure there's a receiver");
        };
        return receiver.diff_mask_is_clear();
    }

    pub fn or_diff_mask(
        &mut self,
        entity: &E,
        component_kind: &ComponentKind,
        other_mask: &DiffMask,
    ) {
        let Some(receiver) = self.receivers.get_mut(&(*entity, *component_kind)) else {
            panic!("Should not call this unless we're sure there's a receiver");
        };
        receiver.or_mask(other_mask);
    }

    pub fn clear_diff_mask(&mut self, entity: &E, component_kind: &ComponentKind) {
        let Some(receiver) = self.receivers.get_mut(&(*entity, *component_kind)) else {
            panic!("Should not call this unless we're sure there's a receiver");
        };
        receiver.clear_mask();
    }

    // Try versions that return Result instead of panicking

    pub fn try_register_component(
        &mut self,
        address: &Option<SocketAddr>,
        entity: &E,
        component_kind: &ComponentKind,
    ) -> Result<(), WorldChannelError> {
        let global_handler = self.global_diff_handler.as_ref().read()
            .map_err(|_| WorldChannelError::RwLockReentrant)?;

        let receiver = global_handler
            .receiver(address, entity, component_kind)
            .ok_or_else(|| WorldChannelError::ComponentNotRegistered {
                entity_id: "<entity>".to_string(),
                component_kind: format!("{:?}", component_kind),
            })?;

        self.receivers.insert((*entity, *component_kind), receiver);
        Ok(())
    }

    pub fn try_diff_mask(
        &self,
        entity: &E,
        component_kind: &ComponentKind,
    ) -> Result<RwLockReadGuard<DiffMask>, WorldChannelError> {
        let receiver = self.receivers.get(&(*entity, *component_kind))
            .ok_or_else(|| WorldChannelError::ReceiverNotFound {
                entity_id: "<entity>".to_string(),
                component_kind: format!("{:?}", component_kind),
            })?;
        receiver.try_mask()
    }

    pub fn try_diff_mask_is_clear(
        &self,
        entity: &E,
        component_kind: &ComponentKind,
    ) -> Result<bool, WorldChannelError> {
        let receiver = self.receivers.get(&(*entity, *component_kind))
            .ok_or_else(|| WorldChannelError::ReceiverNotFound {
                entity_id: "<entity>".to_string(),
                component_kind: format!("{:?}", component_kind),
            })?;
        receiver.try_diff_mask_is_clear()
    }

    pub fn try_or_diff_mask(
        &mut self,
        entity: &E,
        component_kind: &ComponentKind,
        other_mask: &DiffMask,
    ) -> Result<(), WorldChannelError> {
        let receiver = self.receivers.get_mut(&(*entity, *component_kind))
            .ok_or_else(|| WorldChannelError::ReceiverNotFound {
                entity_id: "<entity>".to_string(),
                component_kind: format!("{:?}", component_kind),
            })?;
        receiver.try_or_mask(other_mask)
    }

    pub fn try_clear_diff_mask(
        &mut self,
        entity: &E,
        component_kind: &ComponentKind,
    ) -> Result<(), WorldChannelError> {
        let receiver = self.receivers.get_mut(&(*entity, *component_kind))
            .ok_or_else(|| WorldChannelError::ReceiverNotFound {
                entity_id: "<entity>".to_string(),
                component_kind: format!("{:?}", component_kind),
            })?;
        receiver.try_clear_mask()
    }
}
