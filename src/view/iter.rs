use crate::view::View;
use crate::storage::block::{DenseBlock, SparseBlock};
use std::alloc::Allocator;

pub struct RunsIter<'a, T> {
    data: &'a [T],
    mask: u128,
}

impl<'a, T> Iterator for RunsIter<'a, T> {
    type Item = View<'a, T>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.mask == 0 { return None; }
        let start = self.mask.trailing_zeros() as usize;
        let run = (self.mask >> (start as u32)).trailing_ones() as usize;
        let end = start + run;
        let range_mask = if run == 128 { u128::MAX } else { ((1u128 << run) - 1) << start };
        self.mask &= !range_mask;
        Some(View::new(&self.data[start..end]))
    }
}

pub struct DenseRunsIter<'a, T> {
    data: &'a [T],
    mask: u128,
    offset: usize,
}

impl<'a, T> Iterator for DenseRunsIter<'a, T> {
    type Item = View<'a, T>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.mask == 0 { return None; }
        let start = self.mask.trailing_zeros() as usize;
        let run = (self.mask >> (start as u32)).trailing_ones() as usize;
        let range_mask = if run == 128 { u128::MAX } else { ((1u128 << run) - 1) << start };
        self.mask &= !range_mask;
        let begin = self.offset;
        let end = begin + run;
        self.offset = end;
        Some(View::new(&self.data[begin..end]))
    }
}

pub struct IntersectRuns<'a, 'b, T, U> {
    data_a: &'a [T],
    data_b: &'b [U],
    mask: u128,
}

impl<'a, 'b, T, U> Iterator for IntersectRuns<'a, 'b, T, U> {
    type Item = (View<'a, T>, View<'b, U>);
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.mask == 0 { return None; }
        let start = self.mask.trailing_zeros() as usize;
        let run = (self.mask >> (start as u32)).trailing_ones() as usize;
        let end = start + run;
        let range_mask = if run == 128 { u128::MAX } else { ((1u128 << run) - 1) << start };
        self.mask &= !range_mask;
        Some((View::new(&self.data_a[start..end]), View::new(&self.data_b[start..end])))
    }
}

pub fn intersect<'a, 'b, T, U>(a: RunsIter<'a, T>, b: RunsIter<'b, U>) -> IntersectRuns<'a, 'b, T, U> {
    IntersectRuns { data_a: a.data, data_b: b.data, mask: a.mask & b.mask }
}


impl<'a, 'b, T, U> IntersectRuns<'a, 'b, T, U> {
    pub fn and_mask(mut self, mask: u128) -> Self {
        self.mask &= mask;
        self
    }
}

// materialize removed

pub struct DenseIntersectRuns<'a, 'b, T, U> {
    data_a: &'a [T],
    data_b: &'b [U],
    mask: u128,
    off_a: usize,
    off_b: usize,
}

impl<'a, 'b, T, U> Iterator for DenseIntersectRuns<'a, 'b, T, U> {
    type Item = (View<'a, T>, View<'b, U>);
    fn next(&mut self) -> Option<Self::Item> {
        if self.mask == 0 { return None; }
        let start = self.mask.trailing_zeros() as usize;
        let run = (self.mask >> (start as u32)).trailing_ones() as usize;
        let range_mask = if run == 128 { u128::MAX } else { ((1u128 << run) - 1) << start };
        self.mask &= !range_mask;
        let a_begin = self.off_a;
        let a_end = a_begin + run;
        let b_begin = self.off_b;
        let b_end = b_begin + run;
        self.off_a = a_end;
        self.off_b = b_end;
        Some((View::new(&self.data_a[a_begin..a_end]), View::new(&self.data_b[b_begin..b_end])))
    }
}

pub fn intersect_dense<'a, 'b, T, U>(a: DenseRunsIter<'a, T>, b: DenseRunsIter<'b, U>) -> DenseIntersectRuns<'a, 'b, T, U> {
    DenseIntersectRuns { data_a: a.data, data_b: b.data, mask: a.mask & b.mask, off_a: 0, off_b: 0 }
}


impl<'a, 'b, T, U> DenseIntersectRuns<'a, 'b, T, U> {
    pub fn and_mask(mut self, mask: u128) -> Self {
        self.mask &= mask;
        self
    }
}

pub trait IterViews<'a, T> {
    fn views(&'a self) -> RunsIter<'a, T>;
}

impl<'a, T, A: Allocator> DenseBlock<T, A> {
    pub fn views_dense(&'a self) -> DenseRunsIter<'a, T> {
        let len = self.inner.data.len();
        let cap_mask = if len >= 128 { u128::MAX } else { (1u128 << len) - 1 };
        DenseRunsIter { data: &self.inner.data, mask: self.inner.presence_mask & cap_mask, offset: 0 }
    }
}

impl<'a, T, A> IterViews<'a, T> for SparseBlock<T, A> {
    fn views(&'a self) -> RunsIter<'a, T> {
        let data_t: &[T] = unsafe { std::slice::from_raw_parts(self.data.as_ptr() as *const T, 128) };
        let mask = self.presence_mask;
        RunsIter { data: data_t, mask }
    }
}

impl<'a, T, A> SparseBlock<T, A> {
    pub fn views_complement(&'a self) -> RunsIter<'a, T> {
        let data_t: &[T] = unsafe { std::slice::from_raw_parts(self.data.as_ptr() as *const T, 128) };
        let effective = (self.presence_mask & !self.header.absence_mask);
        RunsIter { data: data_t, mask: effective }
    }
}

 

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::Global;

    #[test]
    fn dense_views_single_run() {
        let mut d = DenseBlock::<u32, Global>::new_in(8, Global);
        d.inner.data.push(0);
        d.inner.data.push(0);
        d.inner.data.push(10);
        d.inner.data.push(11);
        d.inner.data.push(12);
        d.inner.data.push(13);
        d.set_all(0b00111100);
        let runs: Vec<usize> = d.views_dense().map(|v| v.len()).collect();
        assert_eq!(runs, vec![4]);
    }

    #[test]
    fn dense_views_multiple_runs() {
        let mut d = DenseBlock::<u32, Global>::new_in(8, Global);
        d.inner.data.push(0);
        d.inner.data.push(10);
        d.inner.data.push(11);
        d.inner.data.push(0);
        d.inner.data.push(14);
        d.set_all(0b10110);
        let runs: Vec<usize> = d.views_dense().map(|v| v.len()).collect();
        assert_eq!(runs, vec![2, 1]);
    }

    #[test]
    fn sparse_views_single_run() {
        let mut s = SparseBlock::<u32, Global>::new_in(Global);
        for i in 2..6 { unsafe { s.data[i].write(i as u32); } }
        let bits = 0b00111100u128;
        s.set_all(bits);
        let runs: Vec<usize> = s.views().map(|v| v.len()).collect();
        assert_eq!(runs, vec![4]);
    }

    #[test]
    fn sparse_views_multiple_runs() {
        let mut s = SparseBlock::<u32, Global>::new_in(Global);
        for &i in &[1usize, 2usize, 4usize] { unsafe { s.data[i].write(i as u32); } }
        let bits = 0b10110u128;
        s.set_all(bits);
        let runs: Vec<usize> = s.views().map(|v| v.len()).collect();
        assert_eq!(runs, vec![2, 1]);
    }

    #[test]
    fn sparse_views_complement_uses_absence_mask() {
        let mut s = SparseBlock::<u32, Global>::new_in(Global);
        for &i in &[1usize, 2usize, 3usize, 4usize, 8usize, 9usize] { unsafe { s.data[i].write(i as u32); } }
        let sel = (1u128<<1)|(1u128<<2)|(1u128<<3)|(1u128<<4)|(1u128<<8)|(1u128<<9);
        s.set_all(sel);
        s.header.absence_mask |= (1u128<<2)|(1u128<<9);
        let runs: Vec<usize> = s.views_complement().map(|v| v.len()).collect();
        assert_eq!(runs, vec![1, 2, 1]);
    }

    #[test]
    fn intersect_dense_runs_yields_tuple_views() {
        let mut a = DenseBlock::<u32, Global>::new_in(10, Global);
        let mut b = DenseBlock::<u64, Global>::new_in(10, Global);
        a.inner.data.extend([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        b.inner.data.extend([10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
        let ma = (1u128<<2)|(1u128<<3)|(1u128<<4)|(1u128<<5)|(1u128<<8);
        let mb = (1u128<<3)|(1u128<<4)|(1u128<<8);
        a.set_all(ma);
        b.set_all(mb);
        let pairs: Vec<(usize, usize)> = intersect_dense(a.views_dense(), b.views_dense())
            .map(|(va, vb)| (va.len(), vb.len()))
            .collect();
        assert_eq!(pairs, vec![(2, 2), (1, 1)]);
    }

    #[test]
    fn all_and_either_sparse_only_iterate_ab() {
        let mut a = SparseBlock::<u32, Global>::new_in(Global);
        let mut b = SparseBlock::<u32, Global>::new_in(Global);
        let mut c = SparseBlock::<u32, Global>::new_in(Global);
        let mut d = SparseBlock::<u32, Global>::new_in(Global);
        let mut e = SparseBlock::<u32, Global>::new_in(Global);

        for i in [2usize,3,4,8,9] { unsafe { a.data[i].write(i as u32); } }
        for i in [3usize,4,9] { unsafe { b.data[i].write(i as u32); } }
        for i in [4usize,9] { unsafe { c.data[i].write(i as u32); } }
        for i in [3usize] { unsafe { d.data[i].write(i as u32); } }
        for i in [8usize] { unsafe { e.data[i].write(i as u32); } }

        let ma = (1u128<<2)|(1u128<<3)|(1u128<<4)|(1u128<<8)|(1u128<<9);
        let mb = (1u128<<3)|(1u128<<4)|(1u128<<9);
        let mc = (1u128<<4)|(1u128<<9);
        let md = (1u128<<3);
        let me = (1u128<<8);

        a.set_all(ma);
        b.set_all(mb);
        c.set_all(mc);
        d.set_all(md);
        e.set_all(me);

        let either = {
            let cm = c.views().mask;
            let dm = d.views().mask;
            let em = e.views().mask;
            cm | dm | em
        };
        let pairs: Vec<(usize, usize)> = intersect(a.views(), b.views()).and_mask(either)
            .map(|(va, vb)| (va.len(), vb.len()))
            .collect();
        assert_eq!(pairs, vec![(2, 2), (1, 1)]);
    }

    #[test]
    fn all_and_either_dense_only_iterate_ab() {
        let mut a = DenseBlock::<u32, Global>::new_in(10, Global);
        let mut b = DenseBlock::<u32, Global>::new_in(10, Global);
        let mut c = DenseBlock::<u32, Global>::new_in(10, Global);
        let mut d = DenseBlock::<u32, Global>::new_in(10, Global);
        let mut e = DenseBlock::<u32, Global>::new_in(10, Global);

        a.inner.data.extend(0..10);
        b.inner.data.extend(0..10);
        c.inner.data.extend(0..10);
        d.inner.data.extend(0..10);
        e.inner.data.extend(0..10);

        let ma = (1u128<<3)|(1u128<<4)|(1u128<<8);
        let mb = (1u128<<3)|(1u128<<4)|(1u128<<9);
        let mc = (1u128<<4)|(1u128<<9);
        let md = (1u128<<3);
        let me = (1u128<<8);

        a.set_all(ma);
        b.set_all(mb);
        c.set_all(mc);
        d.set_all(md);
        e.set_all(me);

        let either = {
            let cm = c.views_dense().mask;
            let dm = d.views_dense().mask;
            let em = e.views_dense().mask;
            cm | dm | em
        };
        let pairs: Vec<(usize, usize)> = intersect_dense(a.views_dense(), b.views_dense()).and_mask(either)
            .map(|(va, vb)| (va.len(), vb.len()))
            .collect();
        assert_eq!(pairs, vec![(2, 2)]);
    }

    #[test]
    fn intersect_sparse_runs_yields_tuple_views() {
        let mut a = SparseBlock::<u32, Global>::new_in(Global);
        let mut b = SparseBlock::<u64, Global>::new_in(Global);
        for i in [2usize, 3usize, 4usize, 9usize] { unsafe { a.data[i].write(i as u32); } }
        for i in [3usize, 4usize, 9usize] { unsafe { b.data[i].write(i as u64); } }
        let ma = (1u128 << 2) | (1u128 << 3) | (1u128 << 4) | (1u128 << 9);
        let mb = (1u128 << 3) | (1u128 << 4) | (1u128 << 9);
        a.set_all(ma);
        b.set_all(mb);
        let pairs: Vec<(usize, usize)> = intersect(a.views(), b.views())
            .map(|(va, vb)| (va.len(), vb.len()))
            .collect();
        assert_eq!(pairs, vec![(2, 2), (1, 1)]);
    }

}
