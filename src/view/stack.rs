use std::{iter, mem};

use cgmath::*;

use crate::{
    element::{Bounds, RectSize},
    view::{View, ViewContext},
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutDirection {
    #[default]
    Center,
    /// Left for horizontal stacks; up for vertical stacks.
    Leading,
    /// Right for horizontal stacks; down for vertical stacks.
    Trailing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StackLayoutMethod {
    // ```
    // |......[VIEW] [VIEW] [VIEW]......|
    // ```
    Packed {
        direction: LayoutDirection,
        padding: f32,
    },
    // ```
    // | [..VIEW..] [..VIEW..] [..VIEW..] |
    // ```
    DistributeByStretching {
        padding: f32,
    },
    // ```
    // |...[VIEW]...[VIEW]...[VIEW]...|
    // ```
    DistributeByPadding,
}

use subviews_buffer::*;
mod subviews_buffer {
    // Invariance inside.
    use std::mem::{self, transmute};

    use super::*;

    pub(super) struct SubviewsBuffer<'cx, UiState> {
        inner: Vec<*mut (dyn View<UiState> + 'cx)>,
    }

    impl<'cx, UiState> Default for SubviewsBuffer<'cx, UiState> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<'cx, UiState> Clone for SubviewsBuffer<'cx, UiState> {
        fn clone(&self) -> Self {
            Self::new()
        }
    }

    impl<'cx, UiState> SubviewsBuffer<'cx, UiState> {
        pub(super) const fn new() -> Self {
            Self { inner: Vec::new() }
        }

        pub(super) fn take<'views>(&mut self) -> Vec<&'views mut (dyn View<UiState> + 'cx)> {
            let mut inner = mem::take(&mut self.inner);
            inner.clear();
            unsafe {
                transmute::<
                    Vec<*mut (dyn View<UiState> + 'cx)>,        //
                    Vec<&'views mut (dyn View<UiState> + 'cx)>, //
                >(inner)
            }
        }

        pub(super) fn set(&mut self, inner: Vec<&mut (dyn View<UiState> + 'cx)>) {
            self.inner.clear();
            self.inner = unsafe {
                transmute::<
                    Vec<&mut (dyn View<UiState> + 'cx)>, //
                    Vec<*mut (dyn View<UiState> + 'cx)>, //
                >(inner)
            };
        }
    }
}

pub struct HStack<'cx, UiState> {
    size: RectSize,
    subview_sizes: Vec<RectSize>,
    subviews_buffer: SubviewsBuffer<'cx, UiState>,
}

impl<'cx, UiState> Default for HStack<'cx, UiState> {
    fn default() -> Self {
        Self {
            size: Default::default(),
            subview_sizes: Default::default(),
            subviews_buffer: Default::default(),
        }
    }
}

impl<'cx, UiState> Clone for HStack<'cx, UiState> {
    fn clone(&self) -> Self {
        Self {
            size: self.size,
            subview_sizes: self.subview_sizes.clone(),
            subviews_buffer: self.subviews_buffer.clone(),
        }
    }
}

impl<'cx, UiState> HStack<'cx, UiState> {
    pub const fn new() -> Self {
        Self {
            size: RectSize::new(0., 0.),
            subview_sizes: Vec::new(),
            subviews_buffer: SubviewsBuffer::new(),
        }
    }

    pub fn build<'views>(&mut self) -> HStackBuilder<'_, 'views, 'cx, UiState> {
        let subviews = self.subviews_buffer.take();
        HStackBuilder {
            hstack: self,
            subviews,
        }
    }
}

pub struct HStackBuilder<'a, 'views, 'cx, UiState> {
    hstack: &'a mut HStack<'cx, UiState>,
    subviews: Vec<&'views mut (dyn View<UiState> + 'cx)>,
}

impl<'a, 'views, 'cx, UiState> HStackBuilder<'a, 'views, 'cx, UiState> {
    pub fn subview(&mut self, subview: &'views mut (dyn View<UiState> + 'cx)) {
        self.subviews.push(subview);
    }

    pub fn finish(self) -> HStackView<'a, 'views, 'cx, UiState> {
        let mut hstack_view = HStackView {
            hstack: self.hstack,
            subviews: self.subviews,
        };
        hstack_view.relayout();
        hstack_view
    }
}

pub struct HStackView<'a, 'views, 'cx, UiState> {
    hstack: &'a mut HStack<'cx, UiState>,
    subviews: Vec<&'views mut (dyn View<UiState> + 'cx)>,
}

impl<'a, 'views, 'cx, UiState> Drop for HStackView<'a, 'views, 'cx, UiState> {
    fn drop(&mut self) {
        self.hstack
            .subviews_buffer
            .set(mem::take(&mut self.subviews));
    }
}

impl<'a, 'views, 'cx, UiState> HStackView<'a, 'views, 'cx, UiState> {
    pub fn relayout(&mut self) {
        self.hstack.size = RectSize::new(0., 0.);
        self.hstack.subview_sizes.clear();
        for subview in self.subviews.iter() {
            let subview_size = subview.preferred_size();
            self.hstack.size.width += subview_size.width;
            self.hstack.size.height = self.hstack.size.height.max(subview_size.height);
            self.hstack.subview_sizes.push(subview_size);
        }
    }
}

impl<'a, 'views, 'cx, UiState> View<UiState> for HStackView<'a, 'views, 'cx, UiState> {
    fn preferred_size(&self) -> RectSize {
        self.hstack.size
    }

    fn apply_bounds(&mut self, size: Bounds) {
        _ = size;
        let mut x_offset = size.origin.x;
        let y_offset = size.origin.y;
        assert!(self.subviews.len() == self.hstack.subview_sizes.len());
        for (subview, &subview_size) in
            iter::zip(self.subviews.iter_mut(), self.hstack.subview_sizes.iter())
        {
            let top_padding = 0.5 * (self.hstack.size.height - subview_size.height);
            subview.apply_bounds(Bounds {
                origin: point2(x_offset, y_offset + top_padding),
                size: subview_size,
            });
            x_offset += subview_size.width;
        }
    }

    fn prepare_for_drawing(
        &mut self,
        view_context: &ViewContext<UiState>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas: &crate::wgpu_utils::CanvasView,
    ) {
        for subview in self.subviews.iter_mut() {
            subview.prepare_for_drawing(view_context, device, queue, canvas);
        }
    }

    fn draw(&self, view_context: &ViewContext<UiState>, render_pass: &mut wgpu::RenderPass) {
        for subview in self.subviews.iter() {
            subview.draw(view_context, render_pass);
        }
    }
}
