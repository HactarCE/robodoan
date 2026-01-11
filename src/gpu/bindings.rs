macro_rules! bind_group_layout_desc {
    [$($binding:literal => $binding_type:expr),* $(,)?] => {
        wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[$(
                wgpu::BindGroupLayoutEntry {
                    binding: $binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: $binding_type.into(),
                    count: None,
                }
            ),*],
        }
    };
}

macro_rules! bind_group_desc {
    ($layout:expr, [$($binding:literal => $resource:expr),* $(,)?]) => {
        wgpu::BindGroupDescriptor {
            label: None,
            layout: &$layout,
            entries: &[$(
                wgpu::BindGroupEntry {
                    binding: $binding,
                    resource: $resource,
                }
            ),*],
        }
    };
}
