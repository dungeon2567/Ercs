use std::mem::MaybeUninit;

use std::alloc::{Allocator, Global};
use std::ops::{Deref, DerefMut};
use bumpalo::Bump;
use component::Component;

use crate::component::{Tag};
use crate::tick::{Tick, TickDelta};

#[derive(Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct SparseHeader { pub absence_mask: u128 }

#[derive(Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct DenseHeader { }

pub trait MutBlock<T> {
    fn set_all(&mut self, mask: u128);
}

#[repr(C)]
pub struct Block<T, H: Default, A> {
    pub presence_mask: u128,
    pub absence_mask: u128,
    pub changed_at: Tick,
    pub header: H,
    pub alloc: A,
    pub data: T,
}

impl<T, A> Drop for SparseBlock<T, A> {
    fn drop(&mut self) {
        let mut m = self.presence_mask;
        unsafe {
            let ptr = self.data.as_mut_ptr();
            while m != 0 {
                let start = m.trailing_zeros() as usize;
                let run = (m >> (start as u32)).trailing_ones() as usize;
                for i in 0..run {
                    ptr.add(start + i).read().assume_init_drop();
                }
                let range_mask = if run == 128 { u128::MAX } else { ((1u128 << run) - 1) << start };
                m &= !range_mask;
            }
        }
    }
}

pub struct DenseBlock<T, A = Global>
where
    A: std::alloc::Allocator,
{
    pub inner: Block<Vec<T, A>, DenseHeader, A>,
}

impl<T, A: std::alloc::Allocator> Deref for DenseBlock<T, A> {
    type Target = Block<Vec<T, A>, DenseHeader, A>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T, A: std::alloc::Allocator> DerefMut for DenseBlock<T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
pub struct SparseBlock<T, A = Global> {
    pub inner: Block<[MaybeUninit<T>; 128], SparseHeader, A>,
}

impl<T, A> Deref for SparseBlock<T, A> {
    type Target = Block<[MaybeUninit<T>; 128], SparseHeader, A>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T, A> DerefMut for SparseBlock<T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: Sized, A: Allocator + Copy> DenseBlock<T, A> {
    pub fn new_in(capacity: usize, alloc: A) -> Box<Self, A> {
        Box::new_in(
            Self {
                inner: Block {
                    presence_mask: 0,
                    absence_mask: 0,
                    changed_at: Tick::new(0),
                    header: DenseHeader {},
                    data: Vec::with_capacity_in(capacity, alloc),
                    alloc,
                },
            },
            alloc,
        )
    }
}

impl<T, A: Allocator + Copy> DenseBlock<T, A> {
    pub fn set_all(&mut self, mask: u128) {
        self.inner.presence_mask |= mask;
    }
}

impl<T: Sized, A: Allocator + Copy> SparseBlock<T, A> {
    pub fn new(alloc: A) -> Self {
        Self {
            inner: Block {
                presence_mask: 0,
                absence_mask: 0,
                header: SparseHeader { absence_mask: 0 },
                data: std::array::from_fn(|_| MaybeUninit::uninit()),
                changed_at: Tick::new(0),
                alloc
            }
        }
    }

    /// Allocate a flexible block inline using the given allocator
    /// For SparseBlock the capacity is fixed to 128
    pub fn new_in(alloc: A) -> Box<Self, A> {
        Box::new_in(
            Self {
                inner: Block {
                    presence_mask: 0,
                    absence_mask: 0,
                    header: SparseHeader { absence_mask: 0  },
                    data: std::array::from_fn(|_| MaybeUninit::uninit()),
                    changed_at: Tick::new(0),
                    alloc
                }
            },
            alloc,
        )
    }
}

impl<T: Sized> Default for SparseBlock<T, Global> {
    fn default() -> Self { SparseBlock::new(Global) }
}

impl<U: Sized, A: Allocator + Copy> SparseBlock<Box<SparseBlock<U, A>, A>, A> {
    pub fn recompute_all(&mut self, mask: u128) {
        let old_select = self.presence_mask;
        let len = self.data.len();
        let cap = if len > 128 { 128 } else { len };
        let bounds_mask = if cap == 128 { u128::MAX } else { (1u128 << cap) - 1 };
        let mut m = mask & bounds_mask;
        let mut sel: u128 = 0;
        let mut skip: u128 = 0;
        
        while m != 0 {
            let idx = m.trailing_zeros() as usize;
            let child = unsafe { self.data.get_unchecked(idx).assume_init_ref() };
            sel |= child.presence_mask;
            skip |= child.header.absence_mask;
            m &= m - 1;
        }

        let eff = sel & !skip;
        self.header.absence_mask = skip;
        self.presence_mask = eff;
    }
}

impl<U: Sized, A: Allocator> DenseBlock<Box<DenseBlock<U, A>, A>, A> {
    pub fn recompute_all(&mut self, mask: u128) {
        let len = self.inner.data.len();
        let cap = if len > 128 { 128 } else { len };
        let bounds_mask = if cap == 128 { u128::MAX } else { (1u128 << cap) - 1 };
        let mut m = mask & bounds_mask;
        let mut sel: u128 = 0;
        while m != 0 {
            let idx = m.trailing_zeros() as usize;
            let child = unsafe { self.inner.data.get_unchecked(idx) };
            sel |= child.presence_mask;
            m &= m - 1;
        }
        self.inner.presence_mask = sel;
    }
}

impl<T, H: Default, A> Block<T, H, A> {
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.presence_mask.count_ones() as usize
    }

    #[inline(always)]
    pub fn has(&self, index: u32) -> bool {
        assert!(index < 128, "index {} out of bounds for presence_mask", index);

        (self.presence_mask & (1 << index)) != 0
    }

    #[inline(always)]
    pub fn has_any(&self, mask: u128) -> bool {
        (self.presence_mask & mask) != 0
    }

    pub fn set_all(&mut self, mask: u128) {
        self.absence_mask &= !mask;
        self.presence_mask |= mask;
    }

    pub fn skip_all(&mut self, mask: u128) {
        self.presence_mask &= !mask;
        self.absence_mask |= mask;
    }

    pub fn clear_all(&mut self, mask: u128) {
        self.presence_mask &= !mask;
        self.absence_mask &= !mask;
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::Global;
    use bumpalo::Bump;

    #[test]
    fn sparse_block_new_in_initializes_header_and_masks() {
        let bump = Bump::new();

        let b = SparseBlock::<u32, Global>::new_in(Global);

        assert_eq!(b.presence_mask, 0);
        assert_eq!(b.absence_mask, 0);

        assert_eq!(b.count(), 0);
        assert!(!b.has(0));
    }

    #[test]
    fn sparse_block_set_all_updates_masks() {
        let mut b = SparseBlock::<u32, Global>::new_in(Global);
        b.set_all(0b101);
        assert_eq!(b.presence_mask, 0b101);
        assert_eq!(b.absence_mask, 0);

        b.set_all(0b111);
        assert_eq!(b.presence_mask, 0b111);
        assert_eq!(b.header.absence_mask, 0);
    }

    #[test]
    fn sparse_block_set_all_clears_overlapping_skip_bits() {
        let mut b = SparseBlock::<u32, Global>::new_in(Global);
        b.absence_mask = 0b101;
        b.set_all(0b111);
        assert_eq!(b.presence_mask, 0b111);
        assert_eq!(b.absence_mask, 0);
    }

    #[test]

    #[test]
    fn sparse_block_clear_all_clears_select_bits() {
        let mut b = SparseBlock::<u32, Global>::new_in(Global);
        b.set_all(0b1011);
        b.clear_all(0b0001);
        assert_eq!(b.presence_mask, 0b1010);
        assert_eq!(b.absence_mask, 0);
    }

    #[test]
    fn sparse_block_clear_all_clears_skip_bits() {
        let mut b = SparseBlock::<u32, Global>::new_in(Global);
        b.skip_all(0b0101);
        b.clear_all(0b0001);
        assert_eq!(b.absence_mask, 0b0100);
        assert_eq!(b.presence_mask, 0);
    }

    #[test]
    fn sparse_block_clear_all_clears_both_masks() {
        let mut b = SparseBlock::<u32, Global>::new_in(Global);
        b.set_all(0b0011);
        b.skip_all(0b1000);
        b.clear_all(0b1011);
        assert_eq!(b.presence_mask, 0);
        assert_eq!(b.absence_mask, 0);
    }

    #[test]
    fn invariant_disjointness_after_set_and_skip_sequences() {
        let mut b = SparseBlock::<u32, Global>::new_in(Global);
        let _ = b.set_all(0b0110);
        assert_eq!(b.presence_mask & b.absence_mask, 0);
        let _ = b.skip_all(0b0011);
        assert_eq!(b.presence_mask & b.absence_mask, 0);
        let _ = b.set_all(0b1111);
        assert_eq!(b.presence_mask & b.absence_mask, 0);
        let _ = b.skip_all(0b1100);
        assert_eq!(b.presence_mask & b.absence_mask, 0);
    }

    #[test]
    fn set_all_idempotent_and_disjointness() {
        let mut b = SparseBlock::<u32, Global>::new_in(Global);
        b.set_all(0b0101);
        b.set_all(0b0101);
        assert_eq!(b.presence_mask, 0b0101);
        assert_eq!(b.absence_mask, 0);
        assert_eq!(b.presence_mask & b.absence_mask, 0);
    }

    #[test]
    fn skip_all_idempotent_and_disjointness() {
        let mut b = SparseBlock::<u32, Global>::new_in(Global);
        b.skip_all(0b1010);
        b.skip_all(0b1010);
        assert_eq!(b.absence_mask, 0b1010);
        assert_eq!(b.presence_mask, 0);
        assert_eq!(b.presence_mask & b.absence_mask, 0);
    }

    #[test]
    fn set_then_skip_same_bits_disjoint_result() {
        let mut b = SparseBlock::<u32, Global>::new_in(Global);
        let _ = b.set_all(0b1010);
        let _ = b.skip_all(0b1000);
        assert_eq!(b.presence_mask, 0b0010);
        assert_eq!(b.absence_mask, 0b1000);
        assert_eq!(b.presence_mask & b.absence_mask, 0);
    }

    #[test]
    fn skip_then_set_same_bits_disjoint_result() {
        let mut b = SparseBlock::<u32, Global>::new_in(Global);
        let _ = b.skip_all(0b1001);
        let _ = b.set_all(0b0001);
        assert_eq!(b.absence_mask, 0b1000);
        assert_eq!(b.presence_mask, 0b0001);
        assert_eq!(b.presence_mask & b.absence_mask, 0);
    }

    #[test]
fn dense_block_set_all_updates_presence_mask_and_returns_changed_count() {
        let mut d = DenseBlock::<u32, Global>::new_in(8, Global);
        d.set_all(0b0101);
        assert_eq!(d.presence_mask, 0b0101);
        assert_eq!(d.count(), 2);

        d.set_all(0b0111);
        assert_eq!(d.presence_mask, 0b0111);
        assert_eq!(d.count(), 3);
    }

    #[test]
    fn sparse_block_drop_drops_initialized_elements_using_mask() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static DROPS: AtomicUsize = AtomicUsize::new(0);

        struct Track;
        impl Drop for Track { fn drop(&mut self) { DROPS.fetch_add(1, Ordering::SeqCst); } }

        {
            let mut b = SparseBlock::<Track, Global>::new_in(Global);
            unsafe { b.data.get_unchecked_mut(3).write(Track); }
            unsafe { b.data.get_unchecked_mut(7).write(Track); }
            let bits = (1u128 << 3) | (1u128 << 7);
            b.set_all(bits);
        }

        assert_eq!(DROPS.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn dense_block_with_bumpalo_allocator() {
        let arena = bumpalo::Bump::new();
        let mut d = DenseBlock::<u32, &bumpalo::Bump>::new_in(8, &arena);
        d.inner.data.push(1);
        d.inner.data.push(2);
        d.set_all(0b11);
        assert_eq!(d.presence_mask, 0b11);
        assert_eq!(d.inner.data.len(), 2);
    }

    #[test]
    fn dense_page_with_bumpalo_allocator_recompute() {
        let arena = bumpalo::Bump::new();
        let mut page = DenseBlock::<Box<DenseBlock<u32, &bumpalo::Bump>, &bumpalo::Bump>, &bumpalo::Bump>::new_in(8, &arena);

        let mut c0 = DenseBlock::<u32, &bumpalo::Bump>::new_in(8, &arena);
        let mut c1 = DenseBlock::<u32, &bumpalo::Bump>::new_in(8, &arena);

        c0.set_all(0b0011);
        c1.set_all(0b0101);

        page.inner.data.push(c0);
        page.inner.data.push(c1);

        page.recompute_all(0b0011);

        let child0 = unsafe { page.inner.data.get_unchecked(0) };
        let child1 = unsafe { page.inner.data.get_unchecked(1) };

        let sel = child0.presence_mask | child1.presence_mask;
        assert_eq!(page.presence_mask, sel);
    }
}
