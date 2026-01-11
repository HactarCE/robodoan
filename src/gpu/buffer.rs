use std::marker::PhantomData;

use wgpu::util::DeviceExt;

pub struct CachedBuffer<T> {
    label: &'static str,
    device: wgpu::Device,
    usage: wgpu::BufferUsages,
    inner: Option<wgpu::Buffer>,
    _marker: PhantomData<T>,
}

impl<T> CachedBuffer<T> {
    pub fn new(label: &'static str, device: &wgpu::Device, usage: wgpu::BufferUsages) -> Self {
        Self {
            label,
            device: device.clone(),
            usage,
            inner: None,
            _marker: PhantomData,
        }
    }

    pub fn at_len(&mut self, len: usize) -> wgpu::Buffer {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(self.label),
            size: (len * std::mem::size_of::<T>()) as u64,
            usage: self.usage,
            mapped_at_creation: false,
        });
        self.inner = Some(buffer.clone());
        buffer
    }

    pub fn with_data(&mut self, data: &[T]) -> wgpu::Buffer
    where
        T: bytemuck::Pod,
    {
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(self.label),
                contents: bytemuck::cast_slice(data),
                usage: self.usage,
            });
        self.inner = Some(buffer.clone());
        buffer
    }
}
