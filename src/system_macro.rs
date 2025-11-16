#[macro_export]
macro_rules! run_system {
    ($world:expr, $fn:path, $A:ty, $B:ty) => {{
        use std::alloc::Global;

        let a_rc = $world.get::<$A>();
        let b_rc = $world.get::<$B>();

        let a_cell = a_rc.borrow();
        let b_cell = b_rc.borrow();

        use crate::view::iter::{IterViews, intersect};
        let a_store = &a_cell.root;
        let b_store = &b_cell.root;
        for (a_view, b_view) in intersect(a_store.views(), b_store.views()) {
            $fn(&a_view, &b_view);
        }
    }};
}
