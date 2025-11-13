use naia_shared::EntityPropertyError;

/// Tests for EntityProperty error handling
///
/// Note: Many EntityProperty operations require complex setup with converters,
/// entity registries, and protocol configuration. These tests focus on the
/// error paths that can be triggered without full system setup.

#[test]
fn test_invalid_write_operation_error() {
    // RemoteOwned properties should not be written
    // We can't easily create a RemoteOwned property without full setup,
    // but we can test that the error type exists and is correctly defined

    // This is more of a type check - the error type should be accessible
    let error = EntityPropertyError::InvalidWriteOperation {
        property_type: "RemoteOwned",
    };

    assert_eq!(
        format!("{}", error),
        "EntityProperty of type 'RemoteOwned' should never be written"
    );
}

#[test]
fn test_invalid_mutator_operation_error() {
    let error = EntityPropertyError::InvalidMutatorOperation {
        property_type: "RemotePublic",
    };

    assert_eq!(
        format!("{}", error),
        "EntityProperty of type 'RemotePublic' cannot call set_mutator()"
    );
}

#[test]
fn test_bit_length_not_supported_error() {
    let error = EntityPropertyError::BitLengthNotSupported {
        property_type: "Local",
    };

    assert_eq!(
        format!("{}", error),
        "EntityProperty of type 'Local' should never be written, so no need for their bit length"
    );
}

#[test]
fn test_remote_property_manual_set_error() {
    let error = EntityPropertyError::RemotePropertyManualSet;

    assert_eq!(
        format!("{}", error),
        "Remote EntityProperty should never be set manually"
    );
}

#[test]
fn test_invalid_property_manual_set_error() {
    let error = EntityPropertyError::InvalidPropertyManualSet;

    assert_eq!(
        format!("{}", error),
        "Invalid EntityProperty should never be set manually"
    );
}

#[test]
fn test_invalid_property_mirror_error() {
    let error = EntityPropertyError::InvalidPropertyMirror;

    assert_eq!(
        format!("{}", error),
        "Invalid EntityProperty should never be mirrored"
    );
}

#[test]
fn test_waiting_conversion_failed_error() {
    let error = EntityPropertyError::WaitingConversionFailed;

    assert_eq!(
        format!("{}", error),
        "Error completing waiting EntityProperty! Could not convert RemoteEntity to GlobalEntity"
    );
}

#[test]
fn test_invalid_waiting_complete_error() {
    let error = EntityPropertyError::InvalidWaitingComplete {
        property_type: "HostOwned",
    };

    assert_eq!(
        format!("{}", error),
        "Cannot complete EntityProperty of type 'HostOwned'"
    );
}

#[test]
fn test_invalid_state_transition_error() {
    let error = EntityPropertyError::InvalidStateTransition {
        property_type: "RemotePublic",
        operation: "be made public twice",
    };

    assert_eq!(
        format!("{}", error),
        "EntityProperty of type 'RemotePublic' should never be made public twice"
    );
}

#[test]
fn test_invalid_delegation_enable_error() {
    let error = EntityPropertyError::InvalidDelegationEnable {
        property_type: "Local",
    };

    assert_eq!(
        format!("{}", error),
        "EntityProperty of type 'Local' should never enable delegation"
    );
}

#[test]
fn test_invalid_delegation_disable_error() {
    let error = EntityPropertyError::InvalidDelegationDisable {
        property_type: "RemotePublic",
    };

    assert_eq!(
        format!("{}", error),
        "EntityProperty of type 'RemotePublic' should never disable delegation"
    );
}

#[test]
fn test_invalid_localization_error() {
    let error = EntityPropertyError::InvalidLocalization {
        property_type: "RemoteOwned",
    };

    assert_eq!(
        format!("{}", error),
        "EntityProperty of type 'RemoteOwned' should never be made local"
    );
}

#[test]
fn test_insufficient_authority_error() {
    let error = EntityPropertyError::InsufficientAuthority;

    assert_eq!(
        format!("{}", error),
        "Must have Authority over Entity before performing this operation"
    );
}

#[test]
fn test_mutation_not_authorized_error() {
    let error = EntityPropertyError::MutationNotAuthorized;

    assert_eq!(
        format!("{}", error),
        "Must request authority to mutate a Delegated EntityProperty"
    );
}

#[test]
fn test_write_local_entity_not_supported_error() {
    let error = EntityPropertyError::WriteLocalEntityNotSupported {
        property_type: "HostOwned",
    };

    assert_eq!(
        format!("{}", error),
        "EntityProperty type 'HostOwned' cannot use write_local_entity method"
    );
}

#[test]
fn test_mutator_not_initialized_error() {
    let error = EntityPropertyError::MutatorNotInitialized;

    assert_eq!(
        format!("{}", error),
        "EntityProperty mutator should be initialized by now"
    );
}

// Note: We can't test the backward-compatible panic behavior without
// creating RemoteOwned or Invalid property types, which requires full
// entity converter setup. The panic paths still exist in the code for
// backward compatibility, but we can't easily unit test them.

// Note: More comprehensive integration tests would require:
// - Setting up full EntityConverter infrastructure
// - Creating proper component protocols
// - Registering entities in the global entity map
// - Setting up authority handling
//
// These tests focus on verifying error types are properly defined
// and can be constructed, which is sufficient for validating the
// error handling infrastructure is in place.
