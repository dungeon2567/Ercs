use std::rc::Rc;
use crate::world::world::World;
use crate::component::Component;
use std::alloc::Global;
use crate::storage::storage::SparseStorage;

#[derive(Default)]
struct Foo { v: usize }
impl Component for Foo {}

#[derive(Default)]
struct Bar { name: &'static str }
impl Component for Bar {}

#[test]
fn world_get_inserts_once_and_returns_same_rc() {
    let mut w = World::new();
    let a = w.get::<Foo>();
    let b = w.get::<Foo>();
    assert!(Rc::ptr_eq(&a, &b));
}

#[test]
fn world_sparse_storage_mutation_persists() {
    let mut w = World::new();
    let rc = w.get::<Foo>();
    {
        let mut cell = rc.borrow_mut();
        let leaf = cell.leaf_mut();
        unsafe { leaf.data[1].write(Foo { v: 42 }); }
        let bit = 1u128 << 1;
        leaf.set_all(bit);
    }
    let rc2 = w.get::<Foo>();
    let cell2 = rc2.borrow();
    let leaf2 = cell2.leaf();
    assert!(leaf2.has(1));
}

#[test]
fn world_handles_multiple_types_separately() {
    let mut w = World::new();
    let f = w.get::<Foo>();
    let b = w.get::<Bar>();
    {
        let mut bf = b.borrow_mut();
        let leaf = bf.leaf_mut();
        unsafe { leaf.data[3].write(Bar { name: "bar" }); }
        let bit = 1u128 << 3;
        leaf.set_all(bit);
    }
    {
        let mut ff = f.borrow_mut();
        let leaf = ff.leaf_mut();
        unsafe { leaf.data[7].write(Foo { v: 7 }); }
        let bit = 1u128 << 7;
        leaf.set_all(bit);
    }
    let b2 = w.get::<Bar>();
    assert!(b2.borrow().leaf().has(3));
    let f2 = w.get::<Foo>();
    assert!(f2.borrow().leaf().has(7));
}
