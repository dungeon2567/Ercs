use crate::scheduler::PipelineStage;
pub trait System: PipelineStage {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::Global;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use ercs_macros::system;
    use ercs_macros::Component;
    use crate::storage::storage::SparseStorage;
    use crate::view::View;
    use crate::run_system;
    use crate::view::iter::IterViews;

    #[derive(Default, Component)]
    struct A(u32);
    #[derive(Default, Component)]
    struct B(u32);

    static COUNT: AtomicUsize = AtomicUsize::new(0);

    fn my_iter(a: &View<A>, b: &View<B>) {
        assert_eq!(a.len(), b.len());
        COUNT.fetch_add(a.len(), Ordering::SeqCst);
    }

    #[derive(Default, Component)]
    struct C(u32);
    #[derive(Default, Component)]
    struct D(u32);

    static COUNT2: AtomicUsize = AtomicUsize::new(0);
    
    #[system]
    fn my_iter2(a: &View<C>, b: &View<D>) {
        assert_eq!(a.len(), b.len());
        COUNT2.fetch_add(a.len(), Ordering::SeqCst);
    }
}
