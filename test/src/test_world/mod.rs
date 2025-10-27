/// Simple World implementation for E2E testing
/// Based on demos/demo_utils/demo_world

use std::collections::HashMap;

use naia_shared::{
    BigMap, BigMapKey, ComponentKind, ReplicaDynMutWrapper, ReplicaDynRefWrapper,
    ReplicaMutWrapper, ReplicaRefWrapper, Replicate, WorldMutType, WorldRefType,
};

// TestEntity - Simple u64-based entity
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct TestEntity(u64);

impl BigMapKey for TestEntity {
    fn to_u64(&self) -> u64 {
        self.0
    }

    fn from_u64(value: u64) -> Self {
        TestEntity(value)
    }
}

impl TestEntity {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

// TestWorld - Simple HashMap-based world
pub struct TestWorld {
    pub entities: BigMap<TestEntity, HashMap<ComponentKind, Box<dyn Replicate>>>,
}

impl Default for TestWorld {
    fn default() -> Self {
        Self {
            entities: BigMap::new(),
        }
    }
}

impl TestWorld {
    pub fn new() -> Self {
        Self::default()
    }
}

// Internal helper functions
fn has_entity(world: &TestWorld, entity: &TestEntity) -> bool {
    world.entities.contains_key(entity)
}

fn entities(world: &TestWorld) -> Vec<TestEntity> {
    world.entities.keys()
}

fn has_component<R: Replicate>(world: &TestWorld, entity: &TestEntity) -> bool {
    if let Some(component_map) = world.entities.get(entity) {
        component_map.contains_key(&R::kind())
    } else {
        false
    }
}

fn has_component_of_type(
    world: &TestWorld,
    entity: &TestEntity,
    component_kind: &ComponentKind,
) -> bool {
    if let Some(component_map) = world.entities.get(entity) {
        component_map.contains_key(component_kind)
    } else {
        false
    }
}

fn component<R: Replicate>(world: &TestWorld, entity: &TestEntity) -> Option<ReplicaRefWrapper<R>> {
    if let Some(component_map) = world.entities.get(entity) {
        if let Some(boxed_component) = component_map.get(&R::kind()) {
            let component_ref = boxed_component.as_ref();
            let casted_ref = component_ref as *const dyn Replicate as *const R;
            let typed_ref = unsafe { &*casted_ref };
            return Some(ReplicaRefWrapper::new(typed_ref));
        }
    }
    None
}

fn component_of_kind<'a>(
    world: &'a TestWorld,
    entity: &TestEntity,
    component_kind: &ComponentKind,
) -> Option<ReplicaDynRefWrapper<'a>> {
    if let Some(component_map) = world.entities.get(entity) {
        if let Some(boxed_component) = component_map.get(component_kind) {
            return Some(ReplicaDynRefWrapper::new(boxed_component.as_ref()));
        }
    }
    None
}

// WorldRefType implementation
impl WorldRefType<TestEntity> for TestWorld {
    fn has_entity(&self, entity: &TestEntity) -> bool {
        has_entity(self, entity)
    }

    fn entities(&self) -> Vec<TestEntity> {
        entities(self)
    }

    fn has_component<R: Replicate>(&self, entity: &TestEntity) -> bool {
        has_component::<R>(self, entity)
    }

    fn has_component_of_kind(&self, entity: &TestEntity, component_kind: &ComponentKind) -> bool {
        has_component_of_type(self, entity, component_kind)
    }

    fn component<R: Replicate>(&self, entity: &TestEntity) -> Option<ReplicaRefWrapper<R>> {
        component(self, entity)
    }

    fn component_of_kind<'a>(
        &'a self,
        entity: &TestEntity,
        component_kind: &ComponentKind,
    ) -> Option<ReplicaDynRefWrapper<'a>> {
        component_of_kind(self, entity, component_kind)
    }
}

// WorldMutType implementation
impl WorldMutType<TestEntity> for TestWorld {
    fn spawn_entity(&mut self) -> TestEntity {
        let component_map = HashMap::new();
        self.entities.insert(component_map)
    }

    fn local_duplicate_entity(&mut self, entity: &TestEntity) -> TestEntity {
        let new_entity = self.spawn_entity();
        self.local_duplicate_components(&new_entity, entity);
        new_entity
    }

    fn local_duplicate_components(&mut self, new_entity: &TestEntity, old_entity: &TestEntity) {
        for component_kind in self.component_kinds(old_entity) {
            let mut boxed_option: Option<Box<dyn Replicate>> = None;
            if let Some(component) = self.component_of_kind(old_entity, &component_kind) {
                boxed_option = Some(component.copy_to_box());
            }
            if let Some(boxed_component) = boxed_option {
                self.entities
                    .get_mut(new_entity)
                    .unwrap()
                    .insert(component_kind, boxed_component);
            }
        }
    }

    fn despawn_entity(&mut self, entity: &TestEntity) {
        self.entities.remove(entity);
    }

    fn component_kinds(&mut self, entity: &TestEntity) -> Vec<ComponentKind> {
        if let Some(component_map) = self.entities.get(entity) {
            component_map.keys().copied().collect()
        } else {
            Vec::new()
        }
    }

    fn component_mut<R: Replicate>(&mut self, entity: &TestEntity) -> Option<ReplicaMutWrapper<R>> {
        if let Some(component_map) = self.entities.get_mut(entity) {
            if let Some(boxed_component) = component_map.get_mut(&R::kind()) {
                let component_ref = boxed_component.as_mut();
                let casted_ref = component_ref as *mut dyn Replicate as *mut R;
                let typed_ref = unsafe { &mut *casted_ref };
                return Some(ReplicaMutWrapper::new(typed_ref));
            }
        }
        None
    }

    fn component_mut_of_kind<'a>(
        &'a mut self,
        entity: &TestEntity,
        component_kind: &ComponentKind,
    ) -> Option<ReplicaDynMutWrapper<'a>> {
        if let Some(component_map) = self.entities.get_mut(entity) {
            if let Some(boxed_component) = component_map.get_mut(component_kind) {
                return Some(ReplicaDynMutWrapper::new(boxed_component.as_mut()));
            }
        }
        None
    }

    fn insert_component<R: Replicate>(&mut self, entity: &TestEntity, component: R) {
        let component_kind = component.kind();
        if let Some(component_map) = self.entities.get_mut(entity) {
            component_map.insert(component_kind, Box::new(component));
        }
    }

    fn insert_boxed_component(
        &mut self,
        entity: &TestEntity,
        boxed_component: Box<dyn Replicate>,
    ) {
        let component_kind = boxed_component.kind();
        if let Some(component_map) = self.entities.get_mut(entity) {
            component_map.insert(component_kind, boxed_component);
        }
    }
    
    fn component_apply_update(
        &mut self,
        _converter: &dyn naia_shared::LocalEntityAndGlobalEntityConverter,
        entity: &TestEntity,
        component_kind: &ComponentKind,
        update: naia_shared::ComponentUpdate,
    ) -> Result<(), naia_shared::SerdeErr> {
        if let Some(component) = self.component_mut_of_kind(entity, component_kind) {
            component.read_apply_update(&update)?;
        }
        Ok(())
    }
    
    fn component_apply_field_update(
        &mut self,
        _converter: &dyn naia_shared::LocalEntityAndGlobalEntityConverter,
        entity: &TestEntity,
        component_kind: &ComponentKind,
        update: naia_shared::ComponentFieldUpdate,
    ) -> Result<(), naia_shared::SerdeErr> {
        if let Some(component) = self.component_mut_of_kind(entity, component_kind) {
            component.read_apply_field_update(&update)?;
        }
        Ok(())
    }
    
    fn mirror_entities(&mut self, _mutable_entity: &TestEntity, _immutable_entity: &TestEntity) {
        // No-op for test world
    }
    
    fn mirror_components(
        &mut self,
        _mutable_entity: &TestEntity,
        _immutable_entity: &TestEntity,
        _component_kind: &ComponentKind,
    ) {
        // No-op for test world
    }
    
    fn entity_publish(
        &mut self,
        _component_kinds: &naia_shared::ComponentKinds,
        _converter: &dyn naia_shared::EntityAndGlobalEntityConverter<TestEntity>,
        _global_world_manager: &dyn naia_shared::GlobalWorldManagerType,
        _entity: &TestEntity,
    ) {
        // No-op for test world
    }
    
    fn component_publish(
        &mut self,
        _component_kinds: &naia_shared::ComponentKinds,
        _converter: &dyn naia_shared::EntityAndGlobalEntityConverter<TestEntity>,
        _global_world_manager: &dyn naia_shared::GlobalWorldManagerType,
        _entity: &TestEntity,
        _component_kind: &ComponentKind,
    ) {
        // No-op for test world
    }
    
    fn entity_unpublish(&mut self, _entity: &TestEntity) {
        // No-op for test world
    }
    
    fn component_unpublish(&mut self, _entity: &TestEntity, _component_kind: &ComponentKind) {
        // No-op for test world
    }
    
    fn entity_enable_delegation(
        &mut self,
        _component_kinds: &naia_shared::ComponentKinds,
        _converter: &dyn naia_shared::EntityAndGlobalEntityConverter<TestEntity>,
        _global_world_manager: &dyn naia_shared::GlobalWorldManagerType,
        _entity: &TestEntity,
    ) {
        // No-op for test world
    }
    
    fn component_enable_delegation(
        &mut self,
        _component_kinds: &naia_shared::ComponentKinds,
        _converter: &dyn naia_shared::EntityAndGlobalEntityConverter<TestEntity>,
        _global_world_manager: &dyn naia_shared::GlobalWorldManagerType,
        _entity: &TestEntity,
        _component_kind: &ComponentKind,
    ) {
        // No-op for test world
    }
    
    fn entity_disable_delegation(&mut self, _entity: &TestEntity) {
        // No-op for test world
    }
    
    fn component_disable_delegation(&mut self, _entity: &TestEntity, _component_kind: &ComponentKind) {
        // No-op for test world
    }

    fn remove_component<R: Replicate>(&mut self, entity: &TestEntity) -> Option<R> {
        if let Some(component_map) = self.entities.get_mut(entity) {
            if let Some(boxed_component) = component_map.remove(&R::kind()) {
                let raw_ptr = Box::into_raw(boxed_component);
                let casted_ptr = raw_ptr as *mut R;
                let boxed = unsafe { Box::from_raw(casted_ptr) };
                return Some(*boxed);
            }
        }
        None
    }

    fn remove_component_of_kind(
        &mut self,
        entity: &TestEntity,
        component_kind: &ComponentKind,
    ) -> Option<Box<dyn Replicate>> {
        if let Some(component_map) = self.entities.get_mut(entity) {
            return component_map.remove(component_kind);
        }
        None
    }
}

