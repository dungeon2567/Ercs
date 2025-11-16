use std::mem::MaybeUninit;

pub trait Component: Sized + 'static {
    #[inline(always)]
    fn init(index: u32) -> MaybeUninit<Self>{
        MaybeUninit::uninit()
    }
}

pub trait Tag: Component { }

#[cfg(test)]
mod tests {
    use super::*;

    #[ercs_macros::derive_component]
    struct E(u32);

    #[test]
    fn derive_component_impls_trait() {
        let _x = <E as Component>::init(0);
    }
}
