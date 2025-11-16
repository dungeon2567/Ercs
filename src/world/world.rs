use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::component::Component;
use crate::storage::storage::SparseStorage;
use std::alloc::Global;

pub struct World {
    storages: HashMap<TypeId, Box<dyn Any>>,
}
impl World {
    pub fn new() -> Self {
        Self { storages: HashMap::new() }
    }

    pub fn get<T: Component + Default + 'static>(&mut self) -> Rc<RefCell<SparseStorage<T, Global>>> {
        let type_id = TypeId::of::<T>();
        let entry = self.storages
            .entry(type_id)
            .or_insert_with(|| Box::new(Rc::new(RefCell::new(SparseStorage::<T, Global>::default()))));
        let typed: &Rc<RefCell<SparseStorage<T, Global>>> = entry
            .downcast_ref::<Rc<RefCell<SparseStorage<T, Global>>>>()
            .expect("World storage has wrong type");
        typed.clone()
    }
}
