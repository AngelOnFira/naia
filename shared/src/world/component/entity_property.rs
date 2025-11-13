use std::hash::Hash;

use log::{info, warn};
use naia_serde::{BitCounter, BitReader, BitWrite, BitWriter, Serde, SerdeErr};

use crate::{
    world::entity::{
        entity_converters::{
            EntityAndGlobalEntityConverter, LocalEntityAndGlobalEntityConverter,
            LocalEntityAndGlobalEntityConverterMut,
        },
        global_entity::GlobalEntity,
        local_entity::OwnedLocalEntity,
    },
    EntityAuthAccessor, PropertyMutator, RemoteEntity,
};

use super::error::EntityPropertyError;

#[derive(Clone)]
enum EntityRelation {
    HostOwned(HostOwnedRelation),
    RemoteOwned(RemoteOwnedRelation),
    RemoteWaiting(RemoteWaitingRelation),
    RemotePublic(RemotePublicRelation),
    Delegated(DelegatedRelation),
    Local(LocalRelation),
    Invalid,
}

impl EntityRelation {
    fn clone_delegated(&self) -> Option<DelegatedRelation> {
        match self {
            EntityRelation::Delegated(inner) => Some(inner.clone()),
            _ => None,
        }
    }
    fn clone_public(&self) -> Option<RemotePublicRelation> {
        match self {
            EntityRelation::RemotePublic(inner) => Some(inner.clone()),
            _ => None,
        }
    }
    fn name(&self) -> &'static str {
        match self {
            EntityRelation::HostOwned(_) => "HostOwned",
            EntityRelation::RemoteOwned(_) => "RemoteOwned",
            EntityRelation::RemoteWaiting(_) => "RemoteWaiting",
            EntityRelation::RemotePublic(_) => "RemotePublic",
            EntityRelation::Delegated(_) => "Delegated",
            EntityRelation::Local(_) => "Local",
            EntityRelation::Invalid => "Invalid",
        }
    }
    fn try_write(
        &self,
        writer: &mut dyn BitWrite,
        converter: &mut dyn LocalEntityAndGlobalEntityConverterMut,
    ) -> Result<(), EntityPropertyError> {
        match self {
            EntityRelation::HostOwned(inner) => {
                inner.write(writer, converter);
                Ok(())
            }
            EntityRelation::RemotePublic(inner) => {
                inner.write(writer, converter);
                Ok(())
            }
            EntityRelation::Delegated(inner) => {
                inner.try_write(writer, converter)
            }
            EntityRelation::RemoteOwned(_)
            | EntityRelation::RemoteWaiting(_)
            | EntityRelation::Local(_)
            | EntityRelation::Invalid => {
                Err(EntityPropertyError::InvalidWriteOperation {
                    property_type: self.name(),
                })
            }
        }
    }

    fn write(
        &self,
        writer: &mut dyn BitWrite,
        converter: &mut dyn LocalEntityAndGlobalEntityConverterMut,
    ) {
        self.try_write(writer, converter).expect(&format!(
            "EntityProperty of inner type: `{:}` should never be written.",
            self.name()
        ))
    }
    fn try_set_mutator(&mut self, mutator: &PropertyMutator) -> Result<(), EntityPropertyError> {
        match self {
            EntityRelation::HostOwned(inner) => {
                inner.set_mutator(mutator);
                Ok(())
            }
            EntityRelation::RemoteOwned(_)
            | EntityRelation::RemoteWaiting(_)
            | EntityRelation::RemotePublic(_)
            | EntityRelation::Local(_)
            | EntityRelation::Delegated(_)
            | EntityRelation::Invalid => {
                Err(EntityPropertyError::InvalidMutatorOperation {
                    property_type: self.name(),
                })
            }
        }
    }

    fn set_mutator(&mut self, mutator: &PropertyMutator) {
        self.try_set_mutator(mutator).expect(&format!(
            "EntityProperty of inner type: `{:}` cannot call set_mutator()",
            self.name()
        ))
    }

    fn try_bit_length(&self, converter: &mut dyn LocalEntityAndGlobalEntityConverterMut) -> Result<u32, EntityPropertyError> {
        match self {
            EntityRelation::HostOwned(inner) => Ok(inner.bit_length(converter)),
            EntityRelation::Delegated(inner) => inner.try_bit_length(converter),
            EntityRelation::RemotePublic(inner) => Ok(inner.bit_length(converter)),
            EntityRelation::RemoteOwned(_)
            | EntityRelation::RemoteWaiting(_)
            | EntityRelation::Local(_)
            | EntityRelation::Invalid => {
                Err(EntityPropertyError::BitLengthNotSupported {
                    property_type: self.name(),
                })
            }
        }
    }

    fn bit_length(&self, converter: &mut dyn LocalEntityAndGlobalEntityConverterMut) -> u32 {
        self.try_bit_length(converter).expect(&format!(
            "EntityProperty of inner type: `{:}` should never be written, so no need for their bit length.", self.name()
        ))
    }
    fn get<E: Copy + Eq + Hash>(
        &self,
        converter: &dyn EntityAndGlobalEntityConverter<E>,
    ) -> Option<E> {
        let inner_global_entity = self.get_global_entity();

        if let Some(global_entity) = inner_global_entity {
            if let Ok(world_entity) = converter.global_entity_to_entity(&global_entity) {
                return Some(world_entity);
            } else {
                warn!("Could not find World Entity from Global Entity `{:?}`, in order to get the EntityRelation value!", global_entity);
                return None;
            }
        }
        warn!("Could not get EntityRelation value, because EntityRelation has no GlobalEntity!");
        return None;
    }
    fn try_set<E: Copy + Eq + Hash>(
        &mut self,
        converter: &dyn EntityAndGlobalEntityConverter<E>,
        entity: &E,
    ) -> Result<(), EntityPropertyError> {
        match self {
            EntityRelation::HostOwned(inner) => {
                inner.set(converter, entity);
                Ok(())
            }
            EntityRelation::Local(inner) => {
                inner.set(converter, entity);
                Ok(())
            }
            EntityRelation::Delegated(inner) => {
                inner.set(converter, entity);
                Ok(())
            }
            EntityRelation::RemoteOwned(_)
            | EntityRelation::RemoteWaiting(_)
            | EntityRelation::RemotePublic(_) => {
                Err(EntityPropertyError::RemotePropertyManualSet)
            }
            EntityRelation::Invalid => {
                Err(EntityPropertyError::InvalidPropertyManualSet)
            }
        }
    }

    fn set<E: Copy + Eq + Hash>(
        &mut self,
        converter: &dyn EntityAndGlobalEntityConverter<E>,
        entity: &E,
    ) {
        self.try_set(converter, entity)
            .expect("Remote EntityProperty should never be set manually.")
    }

    fn try_set_to_none(&mut self) -> Result<(), EntityPropertyError> {
        match self {
            EntityRelation::HostOwned(inner) => {
                inner.set_to_none();
                Ok(())
            }
            EntityRelation::Local(inner) => {
                inner.set_to_none();
                Ok(())
            }
            EntityRelation::Delegated(inner) => {
                inner.set_to_none();
                Ok(())
            }
            EntityRelation::RemoteOwned(_)
            | EntityRelation::RemoteWaiting(_)
            | EntityRelation::RemotePublic(_) => {
                Err(EntityPropertyError::RemotePropertyManualSet)
            }
            EntityRelation::Invalid => {
                Err(EntityPropertyError::InvalidPropertyManualSet)
            }
        }
    }

    fn set_to_none(&mut self) {
        self.try_set_to_none()
            .expect("Remote EntityProperty should never be set manually.")
    }
    fn try_mirror(&mut self, other: &EntityProperty) -> Result<(), EntityPropertyError> {
        match self {
            EntityRelation::HostOwned(inner) => match &other.inner {
                EntityRelation::HostOwned(other_inner) => {
                    inner.set_global_entity(&other_inner.global_entity);
                    Ok(())
                }
                EntityRelation::RemoteOwned(other_inner) => {
                    inner.set_global_entity(&other_inner.global_entity);
                    Ok(())
                }
                EntityRelation::RemotePublic(other_inner) => {
                    inner.set_global_entity(&other_inner.global_entity);
                    Ok(())
                }
                EntityRelation::Local(other_inner) => {
                    inner.set_global_entity(&other_inner.global_entity);
                    Ok(())
                }
                EntityRelation::Delegated(other_inner) => {
                    inner.set_global_entity(&other_inner.global_entity);
                    Ok(())
                }
                EntityRelation::RemoteWaiting(_) => {
                    inner.mirror_waiting();
                    Ok(())
                }
                EntityRelation::Invalid => {
                    Err(EntityPropertyError::InvalidPropertyMirror)
                }
            },
            EntityRelation::Local(inner) => match &other.inner {
                EntityRelation::HostOwned(other_inner) => {
                    inner.set_global_entity(&other_inner.global_entity);
                    Ok(())
                }
                EntityRelation::RemoteOwned(other_inner) => {
                    inner.set_global_entity(&other_inner.global_entity);
                    Ok(())
                }
                EntityRelation::RemotePublic(other_inner) => {
                    inner.set_global_entity(&other_inner.global_entity);
                    Ok(())
                }
                EntityRelation::Local(other_inner) => {
                    inner.set_global_entity(&other_inner.global_entity);
                    Ok(())
                }
                EntityRelation::Delegated(other_inner) => {
                    inner.set_global_entity(&other_inner.global_entity);
                    Ok(())
                }
                EntityRelation::RemoteWaiting(_) => {
                    inner.mirror_waiting();
                    Ok(())
                }
                EntityRelation::Invalid => {
                    Err(EntityPropertyError::InvalidPropertyMirror)
                }
            },
            EntityRelation::Delegated(inner) => match &other.inner {
                EntityRelation::HostOwned(other_inner) => {
                    inner.set_global_entity(&other_inner.global_entity);
                    Ok(())
                }
                EntityRelation::RemoteOwned(other_inner) => {
                    inner.set_global_entity(&other_inner.global_entity);
                    Ok(())
                }
                EntityRelation::RemotePublic(other_inner) => {
                    inner.set_global_entity(&other_inner.global_entity);
                    Ok(())
                }
                EntityRelation::Local(other_inner) => {
                    inner.set_global_entity(&other_inner.global_entity);
                    Ok(())
                }
                EntityRelation::Delegated(other_inner) => {
                    inner.set_global_entity(&other_inner.global_entity);
                    Ok(())
                }
                EntityRelation::RemoteWaiting(_) => {
                    inner.mirror_waiting();
                    Ok(())
                }
                EntityRelation::Invalid => {
                    Err(EntityPropertyError::InvalidPropertyMirror)
                }
            },
            EntityRelation::RemoteOwned(_)
            | EntityRelation::RemoteWaiting(_)
            | EntityRelation::RemotePublic(_) => {
                Err(EntityPropertyError::RemotePropertyManualSet)
            }
            EntityRelation::Invalid => {
                Err(EntityPropertyError::InvalidPropertyManualSet)
            }
        }
    }

    fn mirror(&mut self, other: &EntityProperty) {
        self.try_mirror(other)
            .expect("Remote EntityProperty should never be set manually.")
    }
    fn waiting_local_entity(&self) -> Option<RemoteEntity> {
        match self {
            EntityRelation::HostOwned(_)
            | EntityRelation::RemoteOwned(_)
            | EntityRelation::RemotePublic(_)
            | EntityRelation::Local(_)
            | EntityRelation::Delegated(_)
            | EntityRelation::Invalid => None,
            EntityRelation::RemoteWaiting(inner) => Some(inner.remote_entity),
        }
    }
    pub fn try_write_local_entity(
        &self,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        writer: &mut BitWriter,
    ) -> Result<(), EntityPropertyError> {
        match self {
            EntityRelation::RemoteOwned(inner) => {
                inner.write_local_entity(converter, writer);
                Ok(())
            }
            EntityRelation::RemotePublic(inner) => {
                inner.write_local_entity(converter, writer);
                Ok(())
            }
            EntityRelation::Delegated(inner) => {
                inner.write_local_entity(converter, writer);
                Ok(())
            }
            EntityRelation::HostOwned(_)
            | EntityRelation::RemoteWaiting(_)
            | EntityRelation::Local(_)
            | EntityRelation::Invalid => {
                Err(EntityPropertyError::WriteLocalEntityNotSupported {
                    property_type: self.name(),
                })
            }
        }
    }

    pub fn write_local_entity(
        &self,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        writer: &mut BitWriter,
    ) {
        self.try_write_local_entity(converter, writer).expect(&format!(
            "This type of EntityProperty: `{:?}` can't use this method",
            self.name()
        ))
    }

    fn get_global_entity(&self) -> Option<GlobalEntity> {
        match self {
            EntityRelation::HostOwned(inner) => inner.global_entity,
            EntityRelation::RemoteOwned(inner) => inner.global_entity,
            EntityRelation::RemotePublic(inner) => inner.global_entity,
            EntityRelation::Local(inner) => inner.global_entity,
            EntityRelation::Delegated(inner) => inner.global_entity,
            EntityRelation::RemoteWaiting(_) | EntityRelation::Invalid => None,
        }
    }
}

#[derive(Clone)]
pub struct EntityProperty {
    inner: EntityRelation,
}

impl EntityProperty {
    // Should only be used by Messages
    pub fn new() -> Self {
        Self {
            inner: EntityRelation::HostOwned(HostOwnedRelation::new()),
        }
    }

    // Should only be used by Components
    pub fn host_owned(mutator_index: u8) -> Self {
        Self {
            inner: EntityRelation::HostOwned(HostOwnedRelation::with_mutator(mutator_index)),
        }
    }

    // Read and create from Remote host
    pub fn new_read(
        reader: &mut BitReader,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
    ) -> Result<Self, SerdeErr> {
        let exists = bool::de(reader)?;
        if exists {
            // LocalEntity is reversed on write, don't worry here
            let local_entity = OwnedLocalEntity::de(reader)?;

            if let Ok(global_entity) = local_entity.convert_to_global(converter) {
                let mut new_impl = RemoteOwnedRelation::new_empty();
                new_impl.global_entity = Some(global_entity);

                let new_self = Self {
                    inner: EntityRelation::RemoteOwned(new_impl),
                };

                Ok(new_self)
            } else {
                if let OwnedLocalEntity::Remote(remote_entity_id) = local_entity {
                    let new_impl = RemoteWaitingRelation::new(RemoteEntity::new(remote_entity_id));

                    let new_self = Self {
                        inner: EntityRelation::RemoteWaiting(new_impl),
                    };

                    Ok(new_self)
                } else {
                    Ok(Self {
                        inner: EntityRelation::Invalid,
                    })
                }
            }
        } else {
            let mut new_impl = RemoteOwnedRelation::new_empty();
            new_impl.global_entity = None;

            let new_self = Self {
                inner: EntityRelation::RemoteOwned(new_impl),
            };

            Ok(new_self)
        }
    }

    pub fn read_write(reader: &mut BitReader, writer: &mut BitWriter) -> Result<(), SerdeErr> {
        let exists = bool::de(reader)?;
        exists.ser(writer);
        if exists {
            OwnedLocalEntity::de(reader)?.ser(writer);
        }
        Ok(())
    }

    pub fn read(
        &mut self,
        reader: &mut BitReader,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
    ) -> Result<(), SerdeErr> {
        let exists = bool::de(reader)?;
        let local_entity_opt = if exists {
            Some(OwnedLocalEntity::de(reader)?)
        } else {
            None
        };

        let eval = (
            self.inner.clone_public(),
            self.inner.clone_delegated(),
            local_entity_opt,
            local_entity_opt.map(|local_entity| local_entity.convert_to_global(converter)),
        );
        self.inner = match eval {
            (None, None, None, None) => {
                EntityRelation::RemoteOwned(RemoteOwnedRelation::new_empty())
            }
            (None, None, Some(local_entity), Some(Err(_))) => {
                info!("1 setting inner to RemoteWaiting");
                EntityRelation::RemoteWaiting(RemoteWaitingRelation::new(
                    local_entity.take_remote(),
                ))
            }
            (None, None, Some(_), Some(Ok(global_entity))) => EntityRelation::RemoteOwned(
                RemoteOwnedRelation::new_with_value(Some(global_entity)),
            ),
            (Some(public_relation), None, None, None) => EntityRelation::RemotePublic(
                RemotePublicRelation::new(None, public_relation.index, &public_relation.mutator),
            ),
            (Some(public_relation), None, Some(local_entity), Some(Err(_))) => {
                EntityRelation::RemoteWaiting(RemoteWaitingRelation::new_public(
                    local_entity.take_remote(),
                    public_relation.index,
                    &public_relation.mutator,
                ))
            }
            (Some(public_relation), None, Some(_), Some(Ok(global_entity))) => {
                EntityRelation::RemotePublic(RemotePublicRelation::new(
                    Some(global_entity),
                    public_relation.index,
                    &public_relation.mutator,
                ))
            }
            (None, Some(delegated_relation), None, None) => {
                EntityRelation::Delegated(delegated_relation.read_none())
            }
            (None, Some(delegated_relation), Some(local_entity), Some(Err(_))) => {
                info!("3 setting inner to RemoteWaiting");
                EntityRelation::RemoteWaiting(RemoteWaitingRelation::new_delegated(
                    local_entity.take_remote(),
                    &delegated_relation.auth_accessor,
                    &delegated_relation.mutator,
                    delegated_relation.index,
                ))
            }
            (None, Some(delegate_relation), Some(_), Some(Ok(global_entity))) => {
                EntityRelation::Delegated(delegate_relation.read_some(global_entity))
            }
            _ => {
                warn!("Unknown read case for EntityProperty - this indicates corrupted state");
                return Err(SerdeErr);
            }
        };

        Ok(())
    }

    pub fn try_waiting_complete(&mut self, converter: &dyn LocalEntityAndGlobalEntityConverter) -> Result<(), EntityPropertyError> {
        match &mut self.inner {
            EntityRelation::RemoteOwned(_)
            | EntityRelation::RemotePublic(_)
            | EntityRelation::Delegated(_) => {
                // already complete! this is intended behavior:
                // waiting Component/Message only sets EntityProperty to RemoteWaiting if it doesn't have an entity in-scope
                // but the entire Component/Message is put on the waitlist if even one of it's EntityProperties is RemoteWaiting
                // and `waiting_complete` is called on all of them, so we skip the already in-scope ones here
                Ok(())
            }
            EntityRelation::RemoteWaiting(inner) => {
                let new_global_entity = {
                    if let Ok(global_entity) =
                        converter.remote_entity_to_global_entity(&inner.remote_entity)
                    {
                        Some(global_entity)
                    } else {
                        return Err(EntityPropertyError::WaitingConversionFailed);
                    }
                };

                if let Some((index, mutator)) = &inner.will_publish {
                    if let Some(accessor) = &inner.will_delegate {
                        // will publish and delegate
                        let mut new_impl =
                            DelegatedRelation::new(new_global_entity, accessor, mutator, *index);
                        new_impl.global_entity = new_global_entity;
                        self.inner = EntityRelation::Delegated(new_impl);
                    } else {
                        // will publish but not delegate
                        let new_impl =
                            RemotePublicRelation::new(new_global_entity, *index, mutator);
                        self.inner = EntityRelation::RemotePublic(new_impl);
                    }
                } else {
                    // will not publish or delegate
                    let mut new_impl = RemoteOwnedRelation::new_empty();
                    new_impl.global_entity = new_global_entity;
                    self.inner = EntityRelation::RemoteOwned(new_impl);
                }
                Ok(())
            }
            EntityRelation::HostOwned(_) | EntityRelation::Local(_) | EntityRelation::Invalid => {
                Err(EntityPropertyError::InvalidWaitingComplete {
                    property_type: self.inner.name(),
                })
            }
        }
    }

    pub fn waiting_complete(&mut self, converter: &dyn LocalEntityAndGlobalEntityConverter) {
        self.try_waiting_complete(converter).expect(&format!(
            "Can't complete EntityProperty of type: `{:?}`!",
            self.inner.name()
        ))
    }

    /// Migrate Remote Property to Public version (non-panicking version)
    pub fn try_remote_publish(&mut self, mutator_index: u8, mutator: &PropertyMutator) -> Result<(), EntityPropertyError> {
        match &mut self.inner {
            EntityRelation::RemoteOwned(inner) => {
                let inner_value = inner.global_entity.clone();
                self.inner = EntityRelation::RemotePublic(RemotePublicRelation::new(
                    inner_value,
                    mutator_index,
                    mutator,
                ));
                Ok(())
            }
            EntityRelation::RemoteWaiting(inner) => {
                inner.remote_publish(mutator_index, mutator);
                Ok(())
            }
            EntityRelation::HostOwned(_)
            | EntityRelation::RemotePublic(_)
            | EntityRelation::Local(_)
            | EntityRelation::Delegated(_)
            | EntityRelation::Invalid => {
                Err(EntityPropertyError::InvalidStateTransition {
                    property_type: self.inner.name(),
                    operation: "be made public twice",
                })
            }
        }
    }

    /// Migrate Remote Property to Public version
    pub fn remote_publish(&mut self, mutator_index: u8, mutator: &PropertyMutator) {
        self.try_remote_publish(mutator_index, mutator).expect(&format!(
            "EntityProperty of type: `{:?}` should never be made public twice.",
            self.inner.name()
        ))
    }

    /// Migrate Remote Property from Public to Private version (non-panicking)
    pub fn try_remote_unpublish(&mut self) -> Result<(), EntityPropertyError> {
        match &mut self.inner {
            EntityRelation::RemotePublic(inner) => {
                let inner_value = inner.global_entity.clone();
                self.inner = EntityRelation::RemoteOwned(RemoteOwnedRelation {
                    global_entity: inner_value,
                });
                Ok(())
            }
            EntityRelation::RemoteWaiting(inner) => {
                inner.remote_unpublish();
                Ok(())
            }
            EntityRelation::HostOwned(_)
            | EntityRelation::RemoteOwned(_)
            | EntityRelation::Local(_)
            | EntityRelation::Delegated(_)
            | EntityRelation::Invalid => {
                Err(EntityPropertyError::InvalidStateTransition {
                    property_type: self.inner.name(),
                    operation: "be unpublished",
                })
            }
        }
    }

    /// Migrate Remote Property to Public version
    pub fn remote_unpublish(&mut self) {
        self.try_remote_unpublish().expect(&format!(
            "EntityProperty of type: `{:?}` should never be unpublished.",
            self.inner.name()
        ))
    }

    /// Migrate Host/RemotePublic Property to Delegated version (non-panicking)
    pub fn try_enable_delegation(
        &mut self,
        accessor: &EntityAuthAccessor,
        mutator_opt: Option<(u8, &PropertyMutator)>,
    ) -> Result<(), EntityPropertyError> {
        let inner_value = self.inner.get_global_entity();

        let (mutator_index, mutator) = {
            if let Some((mutator_index, mutator)) = mutator_opt {
                // with mutator
                match &mut self.inner {
                    EntityRelation::RemoteOwned(_) => (mutator_index, mutator),
                    EntityRelation::RemoteWaiting(inner) => {
                        inner.remote_delegate(accessor);
                        return Ok(());
                    }
                    EntityRelation::Local(_)
                    | EntityRelation::RemotePublic(_)
                    | EntityRelation::HostOwned(_)
                    | EntityRelation::Delegated(_)
                    | EntityRelation::Invalid => {
                        return Err(EntityPropertyError::InvalidDelegationEnable {
                            property_type: self.inner.name(),
                        });
                    }
                }
            } else {
                // without mutator
                match &mut self.inner {
                    EntityRelation::HostOwned(inner) => (
                        inner.index,
                        inner
                            .mutator
                            .as_ref()
                            .ok_or(EntityPropertyError::MutatorNotInitialized)?,
                    ),
                    EntityRelation::RemotePublic(inner) => (inner.index, &inner.mutator),
                    EntityRelation::Local(_)
                    | EntityRelation::RemoteOwned(_)
                    | EntityRelation::RemoteWaiting(_)
                    | EntityRelation::Delegated(_)
                    | EntityRelation::Invalid => {
                        return Err(EntityPropertyError::InvalidDelegationEnable {
                            property_type: self.inner.name(),
                        });
                    }
                }
            }
        };

        self.inner = EntityRelation::Delegated(DelegatedRelation::new(
            inner_value,
            accessor,
            mutator,
            mutator_index,
        ));
        Ok(())
    }

    /// Migrate Host/RemotePublic Property to Delegated version
    pub fn enable_delegation(
        &mut self,
        accessor: &EntityAuthAccessor,
        mutator_opt: Option<(u8, &PropertyMutator)>,
    ) {
        self.try_enable_delegation(accessor, mutator_opt).expect(&format!(
            "EntityProperty of type `{:?}` should never enable delegation.",
            self.inner.name()
        ))
    }

    /// Migrate Delegated Property to Host-Owned (Public) version (non-panicking)
    pub fn try_disable_delegation(&mut self) -> Result<(), EntityPropertyError> {
        match &mut self.inner {
            EntityRelation::Delegated(inner) => {
                let inner_value = inner.global_entity.clone();
                let mut new_inner = HostOwnedRelation::with_mutator(inner.index);
                new_inner.set_mutator(&inner.mutator);
                new_inner.global_entity = inner_value;
                self.inner = EntityRelation::HostOwned(new_inner);
                Ok(())
            }
            EntityRelation::RemoteWaiting(inner) => {
                inner.remote_undelegate();
                Ok(())
            }
            EntityRelation::HostOwned(_)
            | EntityRelation::RemoteOwned(_)
            | EntityRelation::RemotePublic(_)
            | EntityRelation::Local(_)
            | EntityRelation::Invalid => {
                Err(EntityPropertyError::InvalidDelegationDisable {
                    property_type: self.inner.name(),
                })
            }
        }
    }

    /// Migrate Delegated Property to Host-Owned (Public) version
    pub fn disable_delegation(&mut self) {
        self.try_disable_delegation().expect(&format!(
            "EntityProperty of type: `{:?}` should never disable delegation.",
            self.inner.name()
        ))
    }

    /// Migrate Host Property to Local version (non-panicking)
    pub fn try_localize(&mut self) -> Result<(), EntityPropertyError> {
        match &mut self.inner {
            EntityRelation::HostOwned(inner) => {
                let inner_value = inner.global_entity.clone();
                self.inner = EntityRelation::Local(LocalRelation::new(inner_value));
                Ok(())
            }
            EntityRelation::Delegated(inner) => {
                let inner_value = inner.global_entity.clone();
                self.inner = EntityRelation::Local(LocalRelation::new(inner_value));
                Ok(())
            }
            EntityRelation::RemoteOwned(_)
            | EntityRelation::RemotePublic(_)
            | EntityRelation::RemoteWaiting(_)
            | EntityRelation::Local(_)
            | EntityRelation::Invalid => {
                Err(EntityPropertyError::InvalidLocalization {
                    property_type: self.inner.name(),
                })
            }
        }
    }

    /// Migrate Host Property to Local version
    pub fn localize(&mut self) {
        self.try_localize().expect(&format!(
            "EntityProperty of type: `{:?}` should never be made local.",
            self.inner.name()
        ))
    }

    // Pass-through

    pub fn set_mutator(&mut self, mutator: &PropertyMutator) {
        self.inner.set_mutator(mutator);
    }

    // Serialization / deserialization

    pub fn bit_length(&self, converter: &mut dyn LocalEntityAndGlobalEntityConverterMut) -> u32 {
        self.inner.bit_length(converter)
    }

    pub fn write(
        &self,
        writer: &mut dyn BitWrite,
        converter: &mut dyn LocalEntityAndGlobalEntityConverterMut,
    ) {
        self.inner.write(writer, converter);
    }

    pub fn get<E: Copy + Eq + Hash>(
        &self,
        converter: &dyn EntityAndGlobalEntityConverter<E>,
    ) -> Option<E> {
        self.inner.get(converter)
    }

    pub fn set<E: Copy + Eq + Hash>(
        &mut self,
        converter: &dyn EntityAndGlobalEntityConverter<E>,
        entity: &E,
    ) {
        self.inner.set(converter, entity);
    }

    pub fn set_to_none(&mut self) {
        self.inner.set_to_none();
    }

    pub fn mirror(&mut self, other: &EntityProperty) {
        self.inner.mirror(other);
    }

    pub fn waiting_local_entity(&self) -> Option<RemoteEntity> {
        self.inner.waiting_local_entity()
    }

    // used for writing out ready local entity value when splitting updates
    pub fn write_local_entity(
        &self,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        writer: &mut BitWriter,
    ) {
        self.inner.write_local_entity(converter, writer);
    }
}

// HostOwnedRelation
#[derive(Clone)]
struct HostOwnedRelation {
    global_entity: Option<GlobalEntity>,
    mutator: Option<PropertyMutator>,
    index: u8,
}

impl HostOwnedRelation {
    pub fn new() -> Self {
        Self {
            global_entity: None,
            mutator: None,
            index: 0,
        }
    }

    pub fn with_mutator(mutate_index: u8) -> Self {
        Self {
            global_entity: None,
            mutator: None,
            index: mutate_index,
        }
    }

    pub fn set_mutator(&mut self, mutator: &PropertyMutator) {
        self.mutator = Some(mutator.clone_new());
    }

    pub fn write(
        &self,
        writer: &mut dyn BitWrite,
        converter: &mut dyn LocalEntityAndGlobalEntityConverterMut,
    ) {
        let Some(global_entity) = &self.global_entity else {
            false.ser(writer);
            return;
        };
        let Ok(owned_local_entity) = converter.get_or_reserve_entity(global_entity) else {
            false.ser(writer);
            return;
        };

        // Must reverse the LocalEntity because the Host<->Remote
        // relationship inverts after this data goes over the wire
        let reversed_local_entity = owned_local_entity.to_reversed();

        true.ser(writer);
        reversed_local_entity.ser(writer);
    }

    pub fn bit_length(&self, converter: &mut dyn LocalEntityAndGlobalEntityConverterMut) -> u32 {
        let mut bit_counter = BitCounter::new(0, 0, u32::MAX);
        self.write(&mut bit_counter, converter);
        return bit_counter.bits_needed();
    }

    pub fn set<E: Copy + Eq + Hash>(
        &mut self,
        converter: &dyn EntityAndGlobalEntityConverter<E>,
        world_entity: &E,
    ) {
        if let Ok(new_global_entity) = converter.entity_to_global_entity(world_entity) {
            self.global_entity = Some(new_global_entity);
            self.mutate();
        } else {
            warn!("Could not find Global Entity from World Entity, in order to set the EntityRelation value!");
            return;
        }
    }

    pub fn set_to_none(&mut self) {
        self.global_entity = None;
        self.mutate();
    }

    pub fn mirror_waiting(&mut self) {
        self.global_entity = None;
        self.mutate();
    }

    pub fn set_global_entity(&mut self, other_global_entity: &Option<GlobalEntity>) {
        self.global_entity = other_global_entity.clone();
        self.mutate();
    }

    fn mutate(&mut self) {
        let _success = if let Some(mutator) = &mut self.mutator {
            mutator.mutate(self.index)
        } else {
            false
        };
    }
}

// RemoteOwnedRelation
#[derive(Clone, Debug)]
struct RemoteOwnedRelation {
    global_entity: Option<GlobalEntity>,
}

impl RemoteOwnedRelation {
    fn new_empty() -> Self {
        Self {
            global_entity: None,
        }
    }

    fn new_with_value(global_entity: Option<GlobalEntity>) -> Self {
        Self { global_entity }
    }

    pub fn write_local_entity(
        &self,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        writer: &mut BitWriter,
    ) {
        let Some(global_entity) = &self.global_entity else {
            false.ser(writer);
            return;
        };
        let Ok(owned_entity) = converter.global_entity_to_owned_entity(&global_entity) else {
            warn!("Could not find Local Entity from Global Entity, in order to write the EntityRelation value! This should not happen.");
            false.ser(writer);
            return;
        };
        true.ser(writer);
        owned_entity.ser(writer);
    }
}

// RemoteWaitingRelation
#[derive(Clone)]
struct RemoteWaitingRelation {
    remote_entity: RemoteEntity,
    will_publish: Option<(u8, PropertyMutator)>,
    will_delegate: Option<EntityAuthAccessor>,
}

impl RemoteWaitingRelation {
    fn new(remote_entity: RemoteEntity) -> Self {
        Self {
            remote_entity,
            will_publish: None,
            will_delegate: None,
        }
    }
    fn new_public(remote_entity: RemoteEntity, index: u8, mutator: &PropertyMutator) -> Self {
        Self {
            remote_entity,
            will_publish: Some((index, mutator.clone_new())),
            will_delegate: None,
        }
    }
    fn new_delegated(
        local_entity: RemoteEntity,
        auth_accessor: &EntityAuthAccessor,
        mutator: &PropertyMutator,
        index: u8,
    ) -> Self {
        Self {
            remote_entity: local_entity,
            will_publish: Some((index, mutator.clone_new())),
            will_delegate: Some(auth_accessor.clone()),
        }
    }
    pub(crate) fn remote_publish(&mut self, index: u8, mutator: &PropertyMutator) {
        self.will_publish = Some((index, mutator.clone_new()));
    }
    pub(crate) fn remote_unpublish(&mut self) {
        self.will_publish = None;
    }
    pub(crate) fn remote_delegate(&mut self, accessor: &EntityAuthAccessor) {
        self.will_delegate = Some(accessor.clone());
    }
    pub(crate) fn remote_undelegate(&mut self) {
        self.will_delegate = None;
    }
}

// RemoteOwnedRelation
#[derive(Clone)]
struct RemotePublicRelation {
    global_entity: Option<GlobalEntity>,
    mutator: PropertyMutator,
    index: u8,
}

impl RemotePublicRelation {
    pub fn new(global_entity: Option<GlobalEntity>, index: u8, mutator: &PropertyMutator) -> Self {
        Self {
            global_entity,
            mutator: mutator.clone_new(),
            index,
        }
    }

    pub fn bit_length(&self, converter: &mut dyn LocalEntityAndGlobalEntityConverterMut) -> u32 {
        let mut bit_counter = BitCounter::new(0, 0, u32::MAX);
        self.write(&mut bit_counter, converter);
        return bit_counter.bits_needed();
    }

    pub fn write(
        &self,
        writer: &mut dyn BitWrite,
        converter: &mut dyn LocalEntityAndGlobalEntityConverterMut,
    ) {
        let Some(global_entity) = &self.global_entity else {
            false.ser(writer);
            return;
        };
        let Ok(local_entity) = converter.get_or_reserve_entity(global_entity) else {
            false.ser(writer);
            return;
        };

        // Must reverse the LocalEntity because the Host<->Remote
        // relationship inverts after this data goes over the wire
        let reversed_local_entity = local_entity.to_reversed();

        true.ser(writer);
        reversed_local_entity.ser(writer);
    }

    pub fn write_local_entity(
        &self,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        writer: &mut BitWriter,
    ) {
        let Some(global_entity) = &self.global_entity else {
            false.ser(writer);
            return;
        };
        let Ok(owned_entity) = converter.global_entity_to_owned_entity(&global_entity) else {
            warn!("Could not find Local Entity from Global Entity, in order to write the EntityRelation value! This should not happen.");
            false.ser(writer);
            return;
        };
        true.ser(writer);
        owned_entity.ser(writer);
    }
}

// DelegatedRelation
#[derive(Clone)]
struct DelegatedRelation {
    global_entity: Option<GlobalEntity>,
    auth_accessor: EntityAuthAccessor,
    mutator: PropertyMutator,
    index: u8,
}

impl DelegatedRelation {
    /// Create a new DelegatedRelation
    pub fn new(
        global_entity: Option<GlobalEntity>,
        auth_accessor: &EntityAuthAccessor,
        mutator: &PropertyMutator,
        index: u8,
    ) -> Self {
        Self {
            global_entity,
            auth_accessor: auth_accessor.clone(),
            mutator: mutator.clone_new(),
            index,
        }
    }

    pub fn set<E: Copy + Eq + Hash>(
        &mut self,
        converter: &dyn EntityAndGlobalEntityConverter<E>,
        world_entity: &E,
    ) {
        if let Ok(new_global_entity) = converter.entity_to_global_entity(world_entity) {
            self.global_entity = Some(new_global_entity);
            self.mutate();
        } else {
            warn!("Could not find Global Entity from World Entity, in order to set the EntityRelation value!");
            return;
        }
    }

    pub fn set_to_none(&mut self) {
        self.global_entity = None;
        self.mutate();
    }

    pub fn set_global_entity(&mut self, other_global_entity: &Option<GlobalEntity>) {
        self.global_entity = other_global_entity.clone();
        self.mutate();
    }

    pub fn mirror_waiting(&mut self) {
        self.global_entity = None;
        self.mutate();
    }

    pub fn read_none(mut self) -> Self {
        if self.can_read() {
            self.global_entity = None;
            self.mutate();
        }

        self
    }

    pub fn read_some(mut self, global_entity: GlobalEntity) -> Self {
        if self.can_read() {
            self.global_entity = Some(global_entity);
            self.mutate();
        }

        self
    }

    pub fn try_bit_length(&self, converter: &mut dyn LocalEntityAndGlobalEntityConverterMut) -> Result<u32, EntityPropertyError> {
        if !self.can_write() {
            return Err(EntityPropertyError::InsufficientAuthority);
        }
        let mut bit_counter = BitCounter::new(0, 0, u32::MAX);
        self.try_write(&mut bit_counter, converter)?;
        Ok(bit_counter.bits_needed())
    }

    pub fn bit_length(&self, converter: &mut dyn LocalEntityAndGlobalEntityConverterMut) -> u32 {
        self.try_bit_length(converter)
            .expect("Must have Authority over Entity before performing this operation.")
    }

    pub fn try_write(
        &self,
        writer: &mut dyn BitWrite,
        converter: &mut dyn LocalEntityAndGlobalEntityConverterMut,
    ) -> Result<(), EntityPropertyError> {
        if !self.can_write() {
            return Err(EntityPropertyError::InsufficientAuthority);
        }

        let Some(global_entity) = &self.global_entity else {
            false.ser(writer);
            return Ok(());
        };
        let Ok(local_entity) = converter.get_or_reserve_entity(global_entity) else {
            false.ser(writer);
            return Ok(());
        };

        // Must reverse the LocalEntity because the Host<->Remote
        // relationship inverts after this data goes over the wire
        let reversed_local_entity = local_entity.to_reversed();

        true.ser(writer);
        reversed_local_entity.ser(writer);
        Ok(())
    }

    pub fn write(
        &self,
        writer: &mut dyn BitWrite,
        converter: &mut dyn LocalEntityAndGlobalEntityConverterMut,
    ) {
        self.try_write(writer, converter)
            .expect("Must have Authority over Entity before performing this operation.")
    }

    pub fn write_local_entity(
        &self,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        writer: &mut BitWriter,
    ) {
        let Some(global_entity) = &self.global_entity else {
            false.ser(writer);
            return;
        };
        let Ok(host_entity) = converter.global_entity_to_owned_entity(&global_entity) else {
            warn!("Could not find Local Entity from Global Entity, in order to write the EntityRelation value! This should not happen.");
            false.ser(writer);
            return;
        };
        true.ser(writer);
        host_entity.ser(writer);
    }

    fn try_mutate(&mut self) -> Result<(), EntityPropertyError> {
        if !self.can_mutate() {
            return Err(EntityPropertyError::MutationNotAuthorized);
        }
        let _success = self.mutator.mutate(self.index);
        Ok(())
    }

    fn mutate(&mut self) {
        self.try_mutate()
            .expect("Must request authority to mutate a Delegated EntityProperty.")
    }

    fn can_mutate(&self) -> bool {
        self.auth_accessor.auth_status().can_mutate()
    }

    fn can_read(&self) -> bool {
        self.auth_accessor.auth_status().can_read()
    }

    fn can_write(&self) -> bool {
        self.auth_accessor.auth_status().can_write()
    }
}

// LocalRelation
#[derive(Clone, Debug)]
struct LocalRelation {
    global_entity: Option<GlobalEntity>,
}

impl LocalRelation {
    pub fn new(global_entity: Option<GlobalEntity>) -> Self {
        Self { global_entity }
    }

    pub fn set<E: Copy + Eq + Hash>(
        &mut self,
        converter: &dyn EntityAndGlobalEntityConverter<E>,
        world_entity: &E,
    ) {
        if let Ok(new_global_entity) = converter.entity_to_global_entity(world_entity) {
            self.global_entity = Some(new_global_entity);
        } else {
            warn!("Could not find Global Entity from World Entity, in order to set the EntityRelation value!");
            return;
        }
    }

    pub fn set_to_none(&mut self) {
        self.global_entity = None;
    }

    pub fn mirror_waiting(&mut self) {
        self.global_entity = None;
    }

    pub fn set_global_entity(&mut self, other_global_entity: &Option<GlobalEntity>) {
        self.global_entity = other_global_entity.clone();
    }
}
