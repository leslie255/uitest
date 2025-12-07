use std::{iter, mem::transmute};

use cgmath::*;

use crate::{
    element::{Bounds, RectSize},
    utils::*,
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

#[derive(Default)]
pub struct HStack {
    size: RectSize,
    subview_sizes: Vec<RectSize>,
    subviews: Vec<*mut dyn std::any::Any>,
}

impl HStack {
    pub fn add_subviews<'a, 'views, UiState>(
        &'a mut self,
        f: impl FnOnce(&'a mut HStackSubviews<'views, UiState>),
    ) -> &'a mut HStackView<'views, UiState> {
        self.subviews.clear();
        let subviews: &'a mut HStackSubviews<'views, UiState> =
            unsafe { transmute(&mut self.subviews) };
        f(subviews);
        unsafe { transmute(self) }
    }
}

#[repr(transparent)]
pub struct HStackSubviews<'views, UiState> {
    subviews: Vec<&'views mut dyn View<UiState>>,
}

impl<'views, UiState> HStackSubviews<'views, UiState> {
    pub fn add(&mut self, subview: &'views mut dyn View<UiState>) {
        self.subviews.push(subview);
    }
}

pub struct HStackView<'views, UiState> {
    size: RectSize,
    subview_sizes: Vec<RectSize>,
    subviews: Vec<&'views mut dyn View<UiState>>,
}

impl<'views, UiState> HStackView<'views, UiState> {
    pub fn relayout(&mut self) {
        self.size = RectSize::new(0., 0.);
        self.subview_sizes.clear();
        for subview in self.subviews.iter_mut() {
            let subview_size = subview.preferred_size();
            self.size.width += subview_size.width;
            self.size.height = self.size.height.max(subview_size.height);
            self.subview_sizes.push(subview_size);
        }
    }

    pub fn finish(&mut self) {
        self.relayout();
    }
}

impl<'views, UiState> View<UiState> for HStackView<'views, UiState> {
    fn preferred_size(&self) -> RectSize {
        self.size
    }

    fn set_bounds(&mut self, size: Bounds) {
        _ = size;
        let mut x_offset = size.origin.x;
        let y_offset = size.origin.y;
        assert!(self.subviews.len() == self.subview_sizes.len());
        for (subview, &subview_size) in
            iter::zip(self.subviews.iter_mut(), self.subview_sizes.iter())
        {
            subview.set_bounds(Bounds {
                origin: point2(x_offset, y_offset),
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
