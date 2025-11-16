use std::any::{Any, TypeId};
use std::alloc::Allocator;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use bumpalo::Bump;
use crate::component::Component;
use crate::storage::block::{DenseBlock, SparseBlock};

pub trait Storage {

}

impl<T: Component, A: Allocator + Copy + Default> Storage for SparseStorage<T, A> {

}

pub struct DenseStorage<T: Component, A: Allocator + Copy> {
    pub root: DenseBlock<Box<DenseBlock<Box<DenseBlock<T, A>, A>, A>, A>, A>,
    pub alloc: A
}

pub struct SparseStorage<T: Component, A: Allocator + Copy + Default> {
    pub root: SparseBlock<T, A>,
    pub alloc: A
}

impl<T: Component, A: Allocator + Copy + Default>  SparseStorage<T, A> {
    pub fn new(alloc: A) -> Self {
        Self { root: SparseBlock::new(alloc), alloc }
    }

    pub fn leaf_mut(&mut self) -> &mut SparseBlock<T, A> {
        &mut self.root
    }

    pub fn leaf(&self) -> &SparseBlock<T, A> {
        &self.root
    }
}

impl<T: Component, A: Allocator + Copy + Default> Default for SparseStorage<T, A> {
    fn default() -> Self { Self::new(A::default()) }
}


