use std::marker::PhantomData;

use bevy_app::{App, Plugin as PluginType, Update};
use bevy_ecs::schedule::{IntoSystemConfigs, IntoSystemSetConfigs};

use log::info;

use crate::{
    change_detection::{on_despawn, on_host_owned_added, HostSyncEvent},
    system_set::{BeforeHostSyncChangeTracking, HostSyncChangeTracking},
    ReceivePackets, HostOwnedMap, HandleWorldEvents,
};

pub struct SharedPlugin<T: Send + Sync + 'static> {
    phantom_t: PhantomData<T>,
}

impl<T: Send + Sync + 'static> SharedPlugin<T> {
    pub fn new() -> Self {
        Self {
            phantom_t: PhantomData,
        }
    }
}

impl<T: Send + Sync + 'static> PluginType for SharedPlugin<T> {
    fn build(&self, app: &mut App) {
        if app.is_plugin_added::<Self>() {
            info!("attempted to add SharedPlugin twice to App");
            return;
        }
        app
            // RESOURCES //
            .init_resource::<HostOwnedMap>()
            // EVENTS //
            .add_event::<HostSyncEvent>()
            // SYSTEM SETS //
            .configure_sets(
                Update,
                BeforeHostSyncChangeTracking.before(HostSyncChangeTracking),
            )
            .configure_sets(Update, HostSyncChangeTracking.before(ReceivePackets))
            .configure_sets(Update, ReceivePackets.before(HandleWorldEvents))
            // SYSTEMS //
            .add_systems(
                Update,
                on_host_owned_added.in_set(BeforeHostSyncChangeTracking),
            )
            .add_systems(Update, on_despawn.in_set(HostSyncChangeTracking));
    }
}
