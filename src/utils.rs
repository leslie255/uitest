#![allow(dead_code)]

use std::{ mem::transmute, sync::atomic::{self, AtomicBool}};

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
macro_rules! property {
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
}

/// Getters and setters for params that are computed from other params.
#[macro_export]
macro_rules! computed_property {
    {
        vis: $vis:vis,
        param_ty: $ty:ty,
        param: $param:ident,
        set_param: $set_param:ident,
        with_param: $with_param:ident,
        fget: $fget:expr,
        fset: $fset:expr $(,)?
    } => {
        $vis fn $param(&self) -> $ty {
            $fget(self)
        }
        $vis fn $set_param(&mut self, $param: impl Into<$ty>) {
            let param: $ty = $param.into();
            $fset(self, param);
        }
        $vis fn $with_param(mut self, $param: impl Into<$ty>) -> Self {
            self.$set_param($param);
            self
        }
    };

    {
        vis: $vis:vis,
        param_ty: $ty:ty,
        param: $param:ident,
        set_param: $set_param:ident,
        with_param: $with_param:ident,
        fset: $fset:expr $(,)?
    } => {
        $vis fn $set_param(&mut self, $param: impl Into<$ty>) {
            let param: $ty = $param.into();
            $fset(self, param);
        }
        $vis fn $with_param(mut self, $param: impl Into<$ty>) -> Self {
            self.$set_param($param);
            self
        }
    };

}

pub trait AtomicBoolExt {
    fn fetch_set(&self, value: bool, order: atomic::Ordering) -> bool;
}

impl AtomicBoolExt for AtomicBool {
    #[inline(always)]
    fn fetch_set(&self, value: bool, order: atomic::Ordering) -> bool {
        match value {
            true => self.fetch_or(true, order),
            false => self.fetch_and(false, order),
        }
    }
}
