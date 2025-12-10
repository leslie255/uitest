use cgmath::*;

use crate::element::{Bounds, RectSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

pub(crate) trait Point2Ext<T: Copy> {
    fn new_on_axis(axis: Axis, alpha: T, beta: T) -> Self;
    fn alpha(self, axis: Axis) -> T;
    fn beta(self, axis: Axis) -> T;
    fn alpha_mut(&mut self, axis: Axis) -> &mut T;
    fn beta_mut(&mut self, axis: Axis) -> &mut T;
}

impl<T: Copy> Point2Ext<T> for Point2<T> {
    fn new_on_axis(axis: Axis, alpha: T, beta: T) -> Self {
        match axis {
            Axis::Horizontal => Self::new(alpha, beta),
            Axis::Vertical => Self::new(beta, alpha),
        }
    }

    fn alpha(self, axis: Axis) -> T {
        match axis {
            Axis::Horizontal => self.x,
            Axis::Vertical => self.y,
        }
    }

    fn beta(self, axis: Axis) -> T {
        match axis {
            Axis::Horizontal => self.y,
            Axis::Vertical => self.x,
        }
    }

    fn alpha_mut(&mut self, axis: Axis) -> &mut T {
        match axis {
            Axis::Horizontal => &mut self.x,
            Axis::Vertical => &mut self.y,
        }
    }

    fn beta_mut(&mut self, axis: Axis) -> &mut T {
        match axis {
            Axis::Horizontal => &mut self.y,
            Axis::Vertical => &mut self.x,
        }
    }
}

impl<T: Copy> RectSize<T> {
    pub(crate) fn new_on_axis(axis: Axis, length_alpha: T, length_beta: T) -> Self {
        match axis {
            Axis::Horizontal => Self::new(length_alpha, length_beta),
            Axis::Vertical => Self::new(length_beta, length_alpha),
        }
    }

    pub(crate) fn length_alpha(self, axis: Axis) -> T {
        match axis {
            Axis::Horizontal => self.width,
            Axis::Vertical => self.height,
        }
    }

    pub(crate) fn length_beta(self, axis: Axis) -> T {
        match axis {
            Axis::Horizontal => self.height,
            Axis::Vertical => self.width,
        }
    }

    pub(crate) fn length_alpha_mut(&mut self, axis: Axis) -> &mut T {
        match axis {
            Axis::Horizontal => &mut self.width,
            Axis::Vertical => &mut self.height,
        }
    }

    pub(crate) fn length_beta_mut(&mut self, axis: Axis) -> &mut T {
        match axis {
            Axis::Horizontal => &mut self.height,
            Axis::Vertical => &mut self.width,
        }
    }
}

impl<T: Copy> Bounds<T> {
    pub(crate) fn alpha_min(self, axis: Axis) -> T {
        match axis {
            Axis::Horizontal => self.x_min(),
            Axis::Vertical => self.y_min(),
        }
    }

    pub(crate) fn beta_min(self, axis: Axis) -> T {
        match axis {
            Axis::Horizontal => self.y_min(),
            Axis::Vertical => self.x_min(),
        }
    }

    pub(crate) fn alpha_min_mut(&mut self, axis: Axis) -> &mut T {
        match axis {
            Axis::Horizontal => &mut self.origin.x,
            Axis::Vertical => &mut self.origin.y,
        }
    }

    pub(crate) fn beta_min_mut(&mut self, axis: Axis) -> &mut T {
        match axis {
            Axis::Horizontal => &mut self.origin.y,
            Axis::Vertical => &mut self.origin.x,
        }
    }

    pub(crate) fn length_alpha(self, axis: Axis) -> T {
        self.size.length_alpha(axis)
    }

    pub(crate) fn length_beta(self, axis: Axis) -> T {
        self.size.length_beta(axis)
    }

    pub(crate) fn length_alpha_mut(&mut self, axis: Axis) -> &mut T {
        self.size.length_alpha_mut(axis)
    }

    pub(crate) fn length_beta_mut(&mut self, axis: Axis) -> &mut T {
        self.size.length_beta_mut(axis)
    }
}

impl Bounds<f32> {
    pub(crate) fn alpha_max(self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.x_max(),
            Axis::Vertical => self.y_max(),
        }
    }

    pub(crate) fn beta_max(self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.y_max(),
            Axis::Vertical => self.x_max(),
        }
    }
}
