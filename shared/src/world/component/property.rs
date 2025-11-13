use log::warn;
use std::ops::{Deref, DerefMut};
use thiserror::Error;

use naia_serde::{BitReader, BitWrite, BitWriter, Serde, SerdeErr};

use crate::world::{
    component::property_mutate::PropertyMutator, delegation::auth_channel::EntityAuthAccessor,
};

/// Errors that can occur during Property operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PropertyError {
    /// Attempted to set a mutator on a property type that doesn't support it
    #[error("{property_type} Property should never {operation}")]
    InvalidMutatorOperation {
        property_type: &'static str,
        operation: &'static str,
    },

    /// Attempted to write a property that should not be written
    #[error("{property_type} Property should never be written")]
    InvalidWriteOperation {
        property_type: &'static str,
    },

    /// Attempted to read a property that should not be read
    #[error("{property_type} Property should never read")]
    InvalidReadOperation {
        property_type: &'static str,
    },

    /// Attempted to mirror (manually set) a property that should not be set manually
    #[error("{property_type} Property should never be set manually")]
    InvalidMirrorOperation {
        property_type: &'static str,
    },

    /// Attempted an invalid state transition
    #[error("{from_state} Property should never {operation}. (Cannot transition {from_state} -> {to_state})")]
    InvalidStateTransition {
        from_state: &'static str,
        to_state: &'static str,
        operation: &'static str,
    },

    /// Attempted to mutate a property without proper authority
    #[error("Must have authority over Entity before {operation}. Current authority: {current_authority}")]
    InsufficientAuthority {
        operation: &'static str,
        current_authority: String,
    },

    /// Attempted to access a property in an invalid way through DerefMut
    #[error("{property_type} Property should never be mutably accessed")]
    InvalidMutableAccess {
        property_type: &'static str,
    },
}

#[derive(Clone)]
enum PropertyImpl<T: Serde> {
    HostOwned(HostOwnedProperty<T>),
    RemoteOwned(RemoteOwnedProperty<T>),
    RemotePublic(RemotePublicProperty<T>),
    Delegated(DelegatedProperty<T>),
    Local(LocalProperty<T>),
}

impl<T: Serde> PropertyImpl<T> {
    fn name(&self) -> &str {
        match self {
            PropertyImpl::HostOwned(_) => "HostOwned",
            PropertyImpl::RemoteOwned(_) => "RemoteOwned",
            PropertyImpl::RemotePublic(_) => "RemotePublic",
            PropertyImpl::Delegated(_) => "Delegated",
            PropertyImpl::Local(_) => "Local",
        }
    }
}

/// A Property of an Component/Message, that contains data
/// which must be tracked for updates
#[derive(Clone)]
pub struct Property<T: Serde> {
    inner: PropertyImpl<T>,
}

// should be shared
impl<T: Serde> Property<T> {
    /// Create a new Local Property
    pub fn new_local(value: T) -> Self {
        Self {
            inner: PropertyImpl::Local(LocalProperty::new(value)),
        }
    }

    /// Create a new host-owned Property
    pub fn host_owned(value: T, mutator_index: u8) -> Self {
        Self {
            inner: PropertyImpl::HostOwned(HostOwnedProperty::new(value, mutator_index)),
        }
    }

    /// Given a cursor into incoming packet data, initializes the Property with
    /// the synced value
    pub fn new_read(reader: &mut BitReader) -> Result<Self, SerdeErr> {
        let inner_value = Self::read_inner(reader)?;

        Ok(Self {
            inner: PropertyImpl::RemoteOwned(RemoteOwnedProperty::new(inner_value)),
        })
    }

    /// Set an PropertyMutator to track changes to the Property
    ///
    /// # Panics
    ///
    /// Panics if called on RemoteOwned, RemotePublic, Delegated, or Local properties.
    /// Consider using `try_set_mutator` for non-panicking error handling.
    pub fn set_mutator(&mut self, mutator: &PropertyMutator) {
        self.try_set_mutator(mutator)
            .expect("set_mutator called on invalid property type")
    }

    /// Try to set an PropertyMutator to track changes to the Property
    ///
    /// Returns an error if called on property types that don't support mutators:
    /// RemoteOwned, RemotePublic, Delegated, or Local properties.
    pub fn try_set_mutator(&mut self, mutator: &PropertyMutator) -> Result<(), PropertyError> {
        match &mut self.inner {
            PropertyImpl::HostOwned(inner) => {
                inner.set_mutator(mutator);
                Ok(())
            }
            PropertyImpl::RemoteOwned(_) | PropertyImpl::RemotePublic(_) => {
                Err(PropertyError::InvalidMutatorOperation {
                    property_type: "Remote",
                    operation: "call set_mutator()",
                })
            }
            PropertyImpl::Delegated(_) => {
                Err(PropertyError::InvalidMutatorOperation {
                    property_type: "Delegated",
                    operation: "call set_mutator()",
                })
            }
            PropertyImpl::Local(_) => {
                Err(PropertyError::InvalidMutatorOperation {
                    property_type: "Local",
                    operation: "have a mutator",
                })
            }
        }
    }

    // Serialization / deserialization

    /// Writes contained value into outgoing byte stream
    ///
    /// # Panics
    ///
    /// Panics if called on RemoteOwned or Local properties.
    /// Consider using `try_write` for non-panicking error handling.
    pub fn write(&self, writer: &mut dyn BitWrite) {
        self.try_write(writer)
            .expect("write called on invalid property type")
    }

    /// Try to write contained value into outgoing byte stream
    ///
    /// Returns an error if called on property types that should not be written:
    /// RemoteOwned or Local properties.
    pub fn try_write(&self, writer: &mut dyn BitWrite) -> Result<(), PropertyError> {
        match &self.inner {
            PropertyImpl::HostOwned(inner) => {
                inner.write(writer);
                Ok(())
            }
            PropertyImpl::RemoteOwned(_) => {
                Err(PropertyError::InvalidWriteOperation {
                    property_type: "Remote Private",
                })
            }
            PropertyImpl::RemotePublic(inner) => {
                inner.write(writer);
                Ok(())
            }
            PropertyImpl::Local(_) => {
                Err(PropertyError::InvalidWriteOperation {
                    property_type: "Local",
                })
            }
            PropertyImpl::Delegated(inner) => {
                inner.try_write(writer)?;
                Ok(())
            }
        }
    }

    /// Reads from a stream and immediately writes to a stream
    /// Used to buffer updates for later
    pub fn read_write(reader: &mut BitReader, writer: &mut BitWriter) -> Result<(), SerdeErr> {
        T::de(reader)?.ser(writer);
        Ok(())
    }

    /// Given a cursor into incoming packet data, updates the Property with the
    /// synced value
    ///
    /// # Panics
    ///
    /// Panics if called on HostOwned or Local properties.
    /// Consider using `try_read` for non-panicking error handling.
    pub fn read(&mut self, reader: &mut BitReader) -> Result<(), SerdeErr> {
        self.try_read(reader)
            .map_err(|e| match e {
                PropertyError::InvalidReadOperation { .. } => {
                    panic!("{}", e)
                }
                _ => panic!("Unexpected error: {}", e),
            })
    }

    /// Try to read from incoming packet data and update the Property with the synced value
    ///
    /// Returns an error if called on property types that should not read:
    /// HostOwned or Local properties.
    pub fn try_read(&mut self, reader: &mut BitReader) -> Result<(), PropertyError> {
        match &mut self.inner {
            PropertyImpl::HostOwned(_) => {
                Err(PropertyError::InvalidReadOperation {
                    property_type: "Host",
                })
            }
            PropertyImpl::RemoteOwned(inner) => {
                inner.read(reader)
                    .map_err(|_| PropertyError::InvalidReadOperation {
                        property_type: "RemoteOwned (SerdeErr)",
                    })?;
                Ok(())
            }
            PropertyImpl::RemotePublic(inner) => {
                inner.read(reader)
                    .map_err(|_| PropertyError::InvalidReadOperation {
                        property_type: "RemotePublic (SerdeErr)",
                    })?;
                Ok(())
            }
            PropertyImpl::Local(_) => {
                Err(PropertyError::InvalidReadOperation {
                    property_type: "Local",
                })
            }
            PropertyImpl::Delegated(inner) => {
                inner.read(reader)
                    .map_err(|_| PropertyError::InvalidReadOperation {
                        property_type: "Delegated (SerdeErr)",
                    })?;
                Ok(())
            }
        }
    }

    fn read_inner(reader: &mut BitReader) -> Result<T, SerdeErr> {
        T::de(reader)
    }

    // Comparison

    fn inner(&self) -> &T {
        match &self.inner {
            PropertyImpl::HostOwned(inner) => &inner.inner,
            PropertyImpl::RemoteOwned(inner) => &inner.inner,
            PropertyImpl::RemotePublic(inner) => &inner.inner,
            PropertyImpl::Local(inner) => &inner.inner,
            PropertyImpl::Delegated(inner) => &inner.inner,
        }
    }

    /// Compare to another property
    pub fn equals(&self, other: &Self) -> bool {
        self.inner() == other.inner()
    }

    /// Set value to the value of another Property, queues for update if value
    /// changes
    ///
    /// # Panics
    ///
    /// Panics if called on RemoteOwned or RemotePublic properties.
    /// Consider using `try_mirror` for non-panicking error handling.
    pub fn mirror(&mut self, other: &Self) {
        self.try_mirror(other)
            .expect("mirror called on invalid property type")
    }

    /// Try to set value to the value of another Property
    ///
    /// Returns an error if called on property types that should not be set manually:
    /// RemoteOwned or RemotePublic properties.
    pub fn try_mirror(&mut self, other: &Self) -> Result<(), PropertyError> {
        let other_inner = other.inner();
        match &mut self.inner {
            PropertyImpl::HostOwned(inner) => {
                inner.mirror(other_inner);
                Ok(())
            }
            PropertyImpl::RemoteOwned(_) | PropertyImpl::RemotePublic(_) => {
                Err(PropertyError::InvalidMirrorOperation {
                    property_type: "Remote",
                })
            }
            PropertyImpl::Delegated(inner) => {
                inner.try_mirror(other_inner)?;
                Ok(())
            }
            PropertyImpl::Local(inner) => {
                inner.mirror(other_inner);
                Ok(())
            }
        }
    }

    /// Migrate Remote Property to Public version
    ///
    /// # Panics
    ///
    /// Panics if called on HostOwned, RemotePublic, Local, or Delegated properties.
    /// Consider using `try_remote_publish` for non-panicking error handling.
    pub fn remote_publish(&mut self, mutator_index: u8, mutator: &PropertyMutator) {
        self.try_remote_publish(mutator_index, mutator)
            .expect("remote_publish called on invalid property type")
    }

    /// Try to migrate Remote Property to Public version
    ///
    /// Returns an error if the property is not in RemoteOwned state.
    pub fn try_remote_publish(
        &mut self,
        mutator_index: u8,
        mutator: &PropertyMutator,
    ) -> Result<(), PropertyError> {
        match &mut self.inner {
            PropertyImpl::HostOwned(_) => {
                Err(PropertyError::InvalidStateTransition {
                    from_state: "HostOwned",
                    to_state: "RemotePublic",
                    operation: "be made public",
                })
            }
            PropertyImpl::RemoteOwned(inner) => {
                let inner_value = inner.inner.clone();
                self.inner = PropertyImpl::RemotePublic(RemotePublicProperty::new(
                    inner_value,
                    mutator_index,
                    mutator,
                ));
                Ok(())
            }
            PropertyImpl::RemotePublic(_) => {
                Err(PropertyError::InvalidStateTransition {
                    from_state: "RemotePublic",
                    to_state: "RemotePublic",
                    operation: "be made public twice",
                })
            }
            PropertyImpl::Local(_) => {
                Err(PropertyError::InvalidStateTransition {
                    from_state: "Local",
                    to_state: "RemotePublic",
                    operation: "be made public",
                })
            }
            PropertyImpl::Delegated(_) => {
                Err(PropertyError::InvalidStateTransition {
                    from_state: "Delegated",
                    to_state: "RemotePublic",
                    operation: "be made public",
                })
            }
        }
    }

    /// Migrate Remote Property to Private version
    ///
    /// # Panics
    ///
    /// Panics if called on HostOwned, RemoteOwned, Local, or Delegated properties.
    /// Consider using `try_remote_unpublish` for non-panicking error handling.
    pub fn remote_unpublish(&mut self) {
        self.try_remote_unpublish()
            .expect("remote_unpublish called on invalid property type")
    }

    /// Try to migrate Remote Property to Private version
    ///
    /// Returns an error if the property is not in RemotePublic state.
    pub fn try_remote_unpublish(&mut self) -> Result<(), PropertyError> {
        match &mut self.inner {
            PropertyImpl::HostOwned(_) => {
                Err(PropertyError::InvalidStateTransition {
                    from_state: "HostOwned",
                    to_state: "RemoteOwned",
                    operation: "be unpublished",
                })
            }
            PropertyImpl::RemoteOwned(_) => {
                Err(PropertyError::InvalidStateTransition {
                    from_state: "RemoteOwned",
                    to_state: "RemoteOwned",
                    operation: "be unpublished (already private)",
                })
            }
            PropertyImpl::RemotePublic(inner) => {
                let inner_value = inner.inner.clone();
                self.inner = PropertyImpl::RemoteOwned(RemoteOwnedProperty::new(inner_value));
                Ok(())
            }
            PropertyImpl::Local(_) => {
                Err(PropertyError::InvalidStateTransition {
                    from_state: "Local",
                    to_state: "RemoteOwned",
                    operation: "be unpublished",
                })
            }
            PropertyImpl::Delegated(_) => {
                Err(PropertyError::InvalidStateTransition {
                    from_state: "Delegated",
                    to_state: "RemoteOwned",
                    operation: "be unpublished",
                })
            }
        }
    }

    /// Migrate Property to Delegated version
    ///
    /// # Panics
    ///
    /// Panics if the property type is incompatible with the provided parameters.
    /// Consider using `try_enable_delegation` for non-panicking error handling.
    pub fn enable_delegation(
        &mut self,
        accessor: &EntityAuthAccessor,
        mutator_opt: Option<(u8, &PropertyMutator)>,
    ) {
        self.try_enable_delegation(accessor, mutator_opt)
            .expect("enable_delegation called with invalid parameters")
    }

    /// Try to migrate Property to Delegated version
    ///
    /// Returns an error if the property type is incompatible with the provided parameters.
    pub fn try_enable_delegation(
        &mut self,
        accessor: &EntityAuthAccessor,
        mutator_opt: Option<(u8, &PropertyMutator)>,
    ) -> Result<(), PropertyError> {
        let value = self.inner().clone();

        let (mutator_index, mutator) = {
            if let Some((mutator_index, mutator)) = mutator_opt {
                match &mut self.inner {
                    PropertyImpl::RemoteOwned(_) => (mutator_index, mutator),
                    PropertyImpl::Local(_) => {
                        return Err(PropertyError::InvalidStateTransition {
                            from_state: "Local",
                            to_state: "Delegated",
                            operation: "enable delegation this way (with mutator parameter)",
                        });
                    }
                    PropertyImpl::RemotePublic(_) => {
                        return Err(PropertyError::InvalidStateTransition {
                            from_state: "RemotePublic",
                            to_state: "Delegated",
                            operation: "enable delegation this way (with mutator parameter)",
                        });
                    }
                    PropertyImpl::HostOwned(_) => {
                        return Err(PropertyError::InvalidStateTransition {
                            from_state: "HostOwned",
                            to_state: "Delegated",
                            operation: "enable delegation this way (with mutator parameter)",
                        });
                    }
                    PropertyImpl::Delegated(_) => {
                        return Err(PropertyError::InvalidStateTransition {
                            from_state: "Delegated",
                            to_state: "Delegated",
                            operation: "enable delegation this way (with mutator parameter)",
                        });
                    }
                }
            } else {
                match &mut self.inner {
                    PropertyImpl::HostOwned(inner) => (
                        inner.index,
                        inner
                            .mutator
                            .as_ref()
                            .ok_or_else(|| PropertyError::InvalidStateTransition {
                                from_state: "HostOwned",
                                to_state: "Delegated",
                                operation: "enable delegation (mutator not set)",
                            })?,
                    ),
                    PropertyImpl::RemotePublic(inner) => (inner.index, &inner.mutator),
                    PropertyImpl::RemoteOwned(_) => {
                        return Err(PropertyError::InvalidStateTransition {
                            from_state: "RemoteOwned",
                            to_state: "Delegated",
                            operation: "enable delegation this way (without mutator parameter)",
                        });
                    }
                    PropertyImpl::Delegated(_) => {
                        return Err(PropertyError::InvalidStateTransition {
                            from_state: "Delegated",
                            to_state: "Delegated",
                            operation: "enable delegation this way (without mutator parameter)",
                        });
                    }
                    PropertyImpl::Local(_) => {
                        return Err(PropertyError::InvalidStateTransition {
                            from_state: "Local",
                            to_state: "Delegated",
                            operation: "enable delegation this way (without mutator parameter)",
                        });
                    }
                }
            }
        };

        self.inner = PropertyImpl::Delegated(DelegatedProperty::new(
            value,
            accessor,
            mutator,
            mutator_index,
        ));
        Ok(())
    }

    /// Migrate Delegated Property to Host-Owned (Public) version
    ///
    /// # Panics
    ///
    /// Panics if called on HostOwned, RemoteOwned, RemotePublic, or Local properties.
    /// Consider using `try_disable_delegation` for non-panicking error handling.
    pub fn disable_delegation(&mut self) {
        self.try_disable_delegation()
            .expect("disable_delegation called on invalid property type")
    }

    /// Try to migrate Delegated Property to Host-Owned (Public) version
    ///
    /// Returns an error if the property is not in Delegated state.
    pub fn try_disable_delegation(&mut self) -> Result<(), PropertyError> {
        match &mut self.inner {
            PropertyImpl::HostOwned(_) => {
                Err(PropertyError::InvalidStateTransition {
                    from_state: "HostOwned",
                    to_state: "HostOwned",
                    operation: "disable delegation (not delegated)",
                })
            }
            PropertyImpl::RemoteOwned(_) => {
                Err(PropertyError::InvalidStateTransition {
                    from_state: "RemoteOwned",
                    to_state: "HostOwned",
                    operation: "disable delegation (not delegated)",
                })
            }
            PropertyImpl::RemotePublic(_) => {
                Err(PropertyError::InvalidStateTransition {
                    from_state: "RemotePublic",
                    to_state: "HostOwned",
                    operation: "disable delegation (not delegated)",
                })
            }
            PropertyImpl::Local(_) => {
                Err(PropertyError::InvalidStateTransition {
                    from_state: "Local",
                    to_state: "HostOwned",
                    operation: "disable delegation (not delegated)",
                })
            }
            PropertyImpl::Delegated(inner) => {
                let inner_value = inner.inner.clone();
                let mut new_inner = HostOwnedProperty::new(inner_value, inner.index);
                new_inner.set_mutator(&inner.mutator);
                self.inner = PropertyImpl::HostOwned(new_inner);
                Ok(())
            }
        }
    }

    /// Migrate Host Property to Local version
    ///
    /// # Panics
    ///
    /// Panics if called on RemoteOwned, RemotePublic, Local, or Delegated properties.
    /// Consider using `try_localize` for non-panicking error handling.
    pub fn localize(&mut self) {
        self.try_localize()
            .expect("localize called on invalid property type")
    }

    /// Try to migrate Host Property to Local version
    ///
    /// Returns an error if the property is not in HostOwned state.
    pub fn try_localize(&mut self) -> Result<(), PropertyError> {
        match &mut self.inner {
            PropertyImpl::HostOwned(inner) => {
                let inner_value = inner.inner.clone();
                self.inner = PropertyImpl::Local(LocalProperty::new(inner_value));
                Ok(())
            }
            PropertyImpl::RemoteOwned(_) | PropertyImpl::RemotePublic(_) => {
                Err(PropertyError::InvalidStateTransition {
                    from_state: "Remote",
                    to_state: "Local",
                    operation: "be made local",
                })
            }
            PropertyImpl::Local(_) => {
                Err(PropertyError::InvalidStateTransition {
                    from_state: "Local",
                    to_state: "Local",
                    operation: "be made local twice",
                })
            }
            PropertyImpl::Delegated(_) => {
                Err(PropertyError::InvalidStateTransition {
                    from_state: "Delegated",
                    to_state: "Local",
                    operation: "be made local",
                })
            }
        }
    }
}

// It could be argued that Property here is a type of smart-pointer,
// but honestly this is mainly for the convenience of type coercion
impl<T: Serde> Deref for Property<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

impl<T: Serde> DerefMut for Property<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Just assume inner value will be changed, queue for update
        self.try_deref_mut()
            .expect("deref_mut called on invalid property type")
    }
}

impl<T: Serde> Property<T> {
    /// Try to get mutable access to the property value
    ///
    /// Returns an error if called on property types that should not be mutably accessed:
    /// RemoteOwned or RemotePublic properties.
    pub fn try_deref_mut(&mut self) -> Result<&mut T, PropertyError> {
        match &mut self.inner {
            PropertyImpl::HostOwned(inner) => {
                inner.mutate();
                Ok(&mut inner.inner)
            }
            PropertyImpl::RemoteOwned(_) | PropertyImpl::RemotePublic(_) => {
                Err(PropertyError::InvalidMutableAccess {
                    property_type: "Remote",
                })
            }
            PropertyImpl::Local(inner) => Ok(&mut inner.inner),
            PropertyImpl::Delegated(inner) => {
                inner.try_mutate()?;
                Ok(&mut inner.inner)
            }
        }
    }
}

#[derive(Clone)]
pub struct HostOwnedProperty<T: Serde> {
    inner: T,
    mutator: Option<PropertyMutator>,
    index: u8,
}

impl<T: Serde> HostOwnedProperty<T> {
    /// Create a new HostOwnedProperty
    pub fn new(value: T, mutator_index: u8) -> Self {
        Self {
            inner: value,
            mutator: None,
            index: mutator_index,
        }
    }

    pub fn set_mutator(&mut self, mutator: &PropertyMutator) {
        self.mutator = Some(mutator.clone_new());
    }

    pub fn write(&self, writer: &mut dyn BitWrite) {
        self.inner.ser(writer);
    }

    pub fn mirror(&mut self, other: &T) {
        self.mutate();
        self.inner = other.clone();
    }

    pub fn mutate(&mut self) {
        let Some(mutator) = &mut self.mutator else {
            warn!("Host Property should have a mutator immediately after creation.");
            return;
        };
        let _success = mutator.mutate(self.index);
    }
}

#[derive(Clone)]
pub struct LocalProperty<T: Serde> {
    inner: T,
}

impl<T: Serde> LocalProperty<T> {
    /// Create a new LocalProperty
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }

    pub fn mirror(&mut self, other: &T) {
        self.inner = other.clone();
    }
}

#[derive(Clone)]
pub struct RemoteOwnedProperty<T: Serde> {
    inner: T,
}

impl<T: Serde> RemoteOwnedProperty<T> {
    /// Create a new RemoteOwnedProperty
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }

    pub fn read(&mut self, reader: &mut BitReader) -> Result<(), SerdeErr> {
        self.inner = Property::read_inner(reader)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct RemotePublicProperty<T: Serde> {
    inner: T,
    mutator: PropertyMutator,
    index: u8,
}

impl<T: Serde> RemotePublicProperty<T> {
    /// Create a new RemotePublicProperty
    pub fn new(value: T, mutator_index: u8, mutator: &PropertyMutator) -> Self {
        Self {
            inner: value,
            mutator: mutator.clone_new(),
            index: mutator_index,
        }
    }

    pub fn read(&mut self, reader: &mut BitReader) -> Result<(), SerdeErr> {
        self.inner = Property::read_inner(reader)?;
        self.mutate();
        Ok(())
    }

    pub fn write(&self, writer: &mut dyn BitWrite) {
        self.inner.ser(writer);
    }

    fn mutate(&mut self) {
        let _success = self.mutator.mutate(self.index);
    }
}

#[derive(Clone)]
pub struct DelegatedProperty<T: Serde> {
    inner: T,
    auth_accessor: EntityAuthAccessor,
    mutator: PropertyMutator,
    index: u8,
}

impl<T: Serde> DelegatedProperty<T> {
    /// Create a new DelegatedProperty
    pub fn new(
        value: T,
        auth_accessor: &EntityAuthAccessor,
        mutator: &PropertyMutator,
        index: u8,
    ) -> Self {
        Self {
            inner: value,
            auth_accessor: auth_accessor.clone(),
            mutator: mutator.clone_new(),
            index,
        }
    }

    pub fn read(&mut self, reader: &mut BitReader) -> Result<(), SerdeErr> {
        let value = Property::read_inner(reader)?;

        if self.can_read() {
            self.inner = value;
            if self.can_mutate() {
                self.mutate();
            }
        }

        Ok(())
    }

    pub fn write(&self, writer: &mut dyn BitWrite) {
        self.try_write(writer)
            .expect("write called without proper authority")
    }

    pub fn try_write(&self, writer: &mut dyn BitWrite) -> Result<(), PropertyError> {
        if !self.can_write() {
            return Err(PropertyError::InsufficientAuthority {
                operation: "performing write operation",
                current_authority: format!("{:?}", self.auth_accessor.auth_status()),
            });
        }
        self.inner.ser(writer);
        Ok(())
    }

    pub fn mirror(&mut self, other: &T) {
        self.try_mirror(other)
            .expect("mirror called without proper authority")
    }

    pub fn try_mirror(&mut self, other: &T) -> Result<(), PropertyError> {
        self.try_mutate()?;
        self.inner = other.clone();
        Ok(())
    }

    fn mutate(&mut self) {
        self.try_mutate()
            .expect("mutate called without proper authority")
    }

    fn try_mutate(&mut self) -> Result<(), PropertyError> {
        if !self.can_mutate() {
            return Err(PropertyError::InsufficientAuthority {
                operation: "mutating a Delegated Property",
                current_authority: format!("{:?}", self.auth_accessor.auth_status()),
            });
        }
        let _success = self.mutator.mutate(self.index);
        Ok(())
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
