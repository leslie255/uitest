use std::any::type_name;

pub trait AsBindGroup {
    fn bind_group_layout_entries() -> Vec<wgpu::BindGroupLayoutEntry>;
    fn bind_group_entries(&self) -> Vec<wgpu::BindGroupEntry<'_>>;

    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let label = Some(type_name::<Self>());
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &Self::bind_group_layout_entries(),
        })
    }

    fn create_bind_group(
        &self,
        layout: &wgpu::BindGroupLayout,
        device: &wgpu::Device,
    ) -> wgpu::BindGroup {
        let label = Some(type_name::<Self>());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label,
            layout,
            entries: &self.bind_group_entries(),
        })
    }
}
