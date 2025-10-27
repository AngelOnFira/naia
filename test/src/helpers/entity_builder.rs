use super::TestEntity;

/// Fluent builder for creating test entities
pub struct TestEntityBuilder {
    id: u64,
    delegated: bool,
}

impl TestEntityBuilder {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            delegated: false,
        }
    }
    
    /// Mark entity as delegated
    pub fn delegated(mut self) -> Self {
        self.delegated = true;
        self
    }
    
    /// Build the test entity
    pub fn build(self) -> TestEntity {
        TestEntity::new(self.id)
    }
}

impl Default for TestEntityBuilder {
    fn default() -> Self {
        Self::new(1)
    }
}

