use std::{error::Error, mem::transmute};

pub(crate) type DynError = Box<dyn Error>;
pub(crate) type DynResult<T> = Result<T, DynError>;

pub(crate) fn the_default<T: Default>() -> T {
    Default::default()
}

pub(crate) unsafe fn transmute_lifetime<'a, T: ?Sized>(x: &T) -> &'a T {
    unsafe { transmute(x) }
}

pub(crate) unsafe fn transmute_lifetime_mut<'a, T: ?Sized>(x: &mut T) -> &'a mut T {
    unsafe { transmute(x) }
}

#[macro_export]
macro_rules! param_getters_setters {
    {
        vis: $vis:vis,
        param_ty: $ty:ty,
        param: $param:ident,
        param_mut: $param_mut:ident,
        set_param: $set_param:ident,
        with_param: $with_param:ident,
        param_mut_preamble: $param_mut_preamble:expr $(,)?
    } => {
        $vis fn $param(&self) -> $ty {
            self.$param
        }
        $vis fn $param_mut(&mut self) -> &mut $ty {
            $param_mut_preamble(self);
            &mut self.$param
        }
        $vis fn $set_param(&mut self, $param: impl Into<$ty>) {
            $param_mut_preamble(self);
            self.$param = $param.into();
        }
        $vis fn $with_param(mut self, $param: impl Into<$ty>) -> Self {
            $param_mut_preamble(&mut self);
            self.$param = $param.into();
            self
        }
    };

    // Convenience pattern for ignoring `vis`.
    {
        param_ty: $ty:ty,
        param: $param:ident,
        param_mut: $param_mut:ident,
        param_mut_preamble: $param_mut_preamble:expr,
        set_param: $set_param:ident $(,)?
    } => {
        param_getters_setters! {
            vis: pub(self),
            param_ty: $ty,
            param: $param,
            param_mut: $param_mut,
            param_mut_preamble: $param_mut_preamble,
            set_param: $set_param $(,)?
        }
    };
}
