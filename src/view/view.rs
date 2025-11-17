use crate::storage::block::SparseBlock;

pub struct View<'a, T> {
    data: &'a [T]
}

pub struct ViewMut<'a, T> {
    mask: u128,
    block: Option<&'a mut SparseBlock<T>>,
    data: &'a mut [T],
}

impl<'a, T> ViewMut<'a, T> {
    pub fn clear_all(&mut self){
        if let Some(block) = self.block.as_mut() {
            block.clear_all(self.mask);
        }
    }
    pub fn set_all(&mut self){
        if let Some(block) = self.block.as_mut() {
            block.set_all(self.mask);
        }
    }
    pub fn skip_all(&mut self){
        if let Some(block) = self.block.as_mut() {
            block.skip_all(self.mask);
        }
    }
    pub fn none() -> Self {
        Self { mask: 0, block: None, data: &mut [] }
    }
    pub fn is_none(&self) -> bool {
        self.data.is_empty()
    }
    pub fn len(&self) -> usize { self.data.len() }
}

impl<'a, T> View<'a, T> {
    pub fn none() -> Self {
        Self { data: &[] }
    }
    pub fn is_none(&self) -> bool {
        self.data.is_empty()
    }

    pub fn new(data: &'a [T]) -> Self { View { data } }
    pub fn len(&self) -> usize { self.data.len() }
    pub fn as_slice(&self) -> &'a [T] { self.data }
}
