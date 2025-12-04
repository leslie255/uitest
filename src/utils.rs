use std::{error::Error, mem::transmute, rc::Rc, sync::Arc};

pub(crate) type DynError = Box<dyn Error>;
pub(crate) type DynResult<T> = Result<T, DynError>;

pub(crate) fn the_default<T: Default>() -> T {
    Default::default()
}

pub(crate) trait Retain {
    /// Like `clone` but for `Arc` and `Rc`, a sanity check thing to make sure no deep copy is made
    /// where not intended.
    fn retain(&self) -> Self
    where
        Self: Clone,
    {
        self.clone()
    }
}

impl<T> Retain for Rc<T> {}
impl<T> Retain for Arc<T> {}

pub(crate) unsafe fn transmute_lifetime<'a, T: ?Sized>(x: &T) -> &'a T {
    unsafe { transmute(x) }
}

pub(crate) unsafe fn transmute_lifetime_mut<'a, T: ?Sized>(x: &mut T) -> &'a mut T {
    unsafe { transmute(x) }
}

