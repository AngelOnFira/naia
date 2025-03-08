use std::{collections::{HashMap, HashSet}, marker::PhantomData};

use bevy_ecs::{world::World, entity::Entity};

use naia_bevy_shared::{ComponentKind, WorldProxy, WorldRefType};

use crate::{events::InsertBundleEvent, bundle::ReplicateBundle};

pub(crate) struct BundleEventRegistry<T: Send + Sync + 'static> {
    bundles: HashMap<BundleId, BundleInfo>,
    components_to_bundle_ids: HashMap<ComponentKind, HashSet<BundleId>>,
    current_bundle_id: BundleId,
    phantom_t: PhantomData<T>,
}

unsafe impl<T: Send + Sync + 'static> Send for BundleEventRegistry<T> {}
unsafe impl<T: Send + Sync + 'static> Sync for BundleEventRegistry<T> {}

impl<T: Send + Sync + 'static> Default for BundleEventRegistry<T> {
    fn default() -> Self {
        Self {
            bundles: HashMap::new(),
            components_to_bundle_ids: HashMap::new(),
            current_bundle_id: 0,
            phantom_t: PhantomData::<T>,
        }
    }
}

impl<T: Send + Sync + 'static> BundleEventRegistry<T> {
    pub(crate) fn register_bundle_handler<B: ReplicateBundle>(
        &mut self,
    ) {
        let set = B::kind_set();
        let handler = BundleEventHandlerImpl::<T, B>::new_boxed();
        self.register_bundle_handler_impl(set, handler);
    }

    // split this out to avoid large monomorphic calls
    fn register_bundle_handler_impl(&mut self, components: HashSet<ComponentKind>, handler: Box<dyn BundleEventHandler>) {
        let bundle_id = self.next_bundle_id();

        // add components to map
        for kind in components.iter() {
            if !self.components_to_bundle_ids.contains_key(&kind) {
                self.components_to_bundle_ids.insert(*kind, HashSet::new());
            }
            let bundle_ids = self.components_to_bundle_ids.get_mut(&kind).unwrap();
            bundle_ids.insert(bundle_id);
        }

        // add bundle to map
        self.bundles.insert(bundle_id, BundleInfo::new(components, handler));
    }

    fn next_bundle_id(&mut self) -> BundleId {
        let id = self.current_bundle_id;
        self.current_bundle_id += 1;
        id
    }

    pub(crate) fn receive_inserts(&self, world: &mut World, component_kind: &ComponentKind, entities: &Vec<Entity>) {
        let Some(bundle_ids) = self.components_to_bundle_ids.get(&component_kind) else {
            // component is not part of any bundle
            return;
        };

        for bundle_id in bundle_ids {
            let bundle_info = self.bundles.get(bundle_id).unwrap();

            for entity in entities {
                // check if all components are present
                let mut all_components_present = true;
                for kind in bundle_info.kinds.iter() {
                    if !world.proxy().has_component_of_kind(entity, kind) {
                        all_components_present = false;
                        break;
                    }
                }

                if all_components_present {
                    bundle_info.handler.send_event(world, *entity);
                }
            }
        }
    }
}

type BundleId = u32;

struct BundleInfo {
    kinds: HashSet<ComponentKind>,
    handler: Box<dyn BundleEventHandler>,
}

impl BundleInfo {
    fn new(kinds: HashSet<ComponentKind>, handler: Box<dyn BundleEventHandler>) -> Self {
        Self {
            kinds,
            handler,
        }
    }
}

trait BundleEventHandler: Send + Sync {
    fn send_event(&self, world: &mut World, entity: Entity);
}

struct BundleEventHandlerImpl<T: Send + Sync + 'static, B: ReplicateBundle> {
    phantom_t: PhantomData<T>,
    phantom_r: PhantomData<B>,
}

impl<T: Send + Sync + 'static, B: ReplicateBundle> BundleEventHandlerImpl<T, B> {
    fn new() -> Self {
        Self {
            phantom_t: PhantomData::<T>,
            phantom_r: PhantomData::<B>,
        }
    }

    fn new_boxed() -> Box<dyn BundleEventHandler> {
        Box::new(Self::new())
    }
}

impl<T: Send + Sync + 'static, B: ReplicateBundle> BundleEventHandler for BundleEventHandlerImpl<T, B> {
    fn send_event(&self, world: &mut World, entity: Entity) {
        world.send_event(InsertBundleEvent::<T, B>::new(entity));
    }
}