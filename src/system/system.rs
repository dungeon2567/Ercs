use crate::scheduler::PipelineStage;
pub trait System: PipelineStage {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::Global;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use crate::storage::storage::SparseStorage;
    use crate::view::View;
    use crate::run_system;
    use crate::component::Component;
    use crate::view::iter::IterViews;

    #[derive(Default)]
    struct A(u32);
    impl Component for A {}
    #[derive(Default)]
    struct B(u32);
    impl Component for B {}

    static COUNT: AtomicUsize = AtomicUsize::new(0);

    fn my_iter(a: &View<A>, b: &View<B>) {
        assert_eq!(a.len(), b.len());
        COUNT.fetch_add(a.len(), Ordering::SeqCst);
    }

    #[test]
    fn run_system_intersects_sparse_views() {
        let mut world = crate::world::World::new();

        let a_rc = world.get::<A>();
        let b_rc = world.get::<B>();

        {
            let mut a_cell = a_rc.borrow_mut();
            let s = a_cell.leaf_mut();
            for i in 2..=5 { unsafe { s.data[i].write(A(i as u32)); } }
            let bits = (1u128<<2)|(1u128<<3)|(1u128<<4)|(1u128<<5);
            s.set_all(bits);
        }

        {
            let mut b_cell = b_rc.borrow_mut();
            let s = b_cell.leaf_mut();
            for i in 3..=5 { unsafe { s.data[i].write(B(i as u32)); } }
            let bits = (1u128<<3)|(1u128<<4)|(1u128<<5);
            s.set_all(bits);
        }

        COUNT.store(0, Ordering::SeqCst);
        run_system!(world, my_iter, A, B);
        assert_eq!(COUNT.load(Ordering::SeqCst), 3);
    }

    #[derive(Default)]
    struct C(u32);
    impl Component for C {}
    #[derive(Default)]
    struct D(u32);
    impl Component for D {}

    static COUNT2: AtomicUsize = AtomicUsize::new(0);

    #[ercs_macros::system]
    fn my_iter2(a: &View<C>, b: &View<D>) {
        assert_eq!(a.len(), b.len());
        COUNT2.fetch_add(a.len(), Ordering::SeqCst);
    }

    #[test]
    fn attribute_system_creates_struct_and_runs() {
        let mut world = crate::world::World::new();

        let a_rc = world.get::<C>();
        let b_rc = world.get::<D>();

        {
            let mut a_cell = a_rc.borrow_mut();
            let s = a_cell.leaf_mut();
            for i in 2..=5 { unsafe { s.data[i].write(C(i as u32)); } }
            let bits = (1u128<<2)|(1u128<<3)|(1u128<<4)|(1u128<<5);
            s.set_all(bits);
        }

        {
            let mut b_cell = b_rc.borrow_mut();
            let s = b_cell.leaf_mut();
            for i in 3..=5 { unsafe { s.data[i].write(D(i as u32)); } }
            let bits = (1u128<<3)|(1u128<<4)|(1u128<<5);
            s.set_all(bits);
        }

        COUNT2.store(0, Ordering::SeqCst);
        let mut sys = MyIter2System::new(&mut world);
        sys.run();
        assert_eq!(COUNT2.load(Ordering::SeqCst), 3);
    }
}
