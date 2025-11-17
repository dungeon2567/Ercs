use std::rc::Rc;
use crate::world::world::World;
use ercs_macros::Component;
use std::alloc::Global;
use crate::storage::storage::SparseStorage;

#[derive(Default, Component)]
struct Foo { v: usize }

#[derive(Default, Component)]
struct Bar { name: &'static str }
