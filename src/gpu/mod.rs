use crate::sim::{Axis, Elem, Grip, Twist, blockbuilding::BlockList, group};

#[macro_use]
mod bindings;
mod buffer;

use buffer::CachedBuffer;

type Out = u32;

pub struct Gpu {
    lut_buffer: wgpu::Buffer,

    block_lists_buffer: CachedBuffer<BlockList>,
    twists_buffer: CachedBuffer<Twist>,

    output_buffer: CachedBuffer<Out>,
    download_buffer: CachedBuffer<Out>,

    pipeline: wgpu::ComputePipeline,

    queue: wgpu::Queue,
    device: wgpu::Device,
}

impl Default for Gpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Gpu {
    pub fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::from_env_or_default());

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .expect("failed to create wgpu adapter");

        // Ensure that compute shaders are supported
        let capabilities_flags = adapter.get_downlevel_capabilities().flags;
        if !capabilities_flags.contains(wgpu::DownlevelFlags::COMPUTE_SHADERS) {
            panic!("wgpu adapter does not support compute shaders");
        }

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("failed to request wgpu device");

        let mut lut_texture_data = vec![];
        let mul_elem_elem = lut_texture_data.len() / group::ELEM_COUNT;
        for e2 in Elem::iter_all() {
            for e1 in Elem::iter_all() {
                lut_texture_data.push((e1 * e2).id());
            }
        }
        let mul_elem_grip = lut_texture_data.len() / group::ELEM_COUNT;
        for g in Grip::ALL {
            for e in Elem::iter_all() {
                lut_texture_data.push((e * g).id());
            }
        }
        let mul_elem_axis = lut_texture_data.len() / group::ELEM_COUNT;
        for a in Axis::ALL {
            for e in Elem::iter_all() {
                lut_texture_data.push((e * a).id());
            }
        }
        let inverse_elem = lut_texture_data.len() / group::ELEM_COUNT;
        for e in Elem::iter_all() {
            lut_texture_data.push(e.inv().id());
        }

        let module = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("compute_pipeline_layout"),
                    bind_group_layouts: &[&device.create_bind_group_layout(
                        &bind_group_layout_desc![
                            0 => ReadOnlyBuffer, // lut_buffer
                            1 => WriteableBuffer, // output_buffer
                            2 => ReadOnlyBuffer, // block_lists_buffer
                            3 => ReadOnlyBuffer, // twists_buffer
                        ],
                    )],
                    immediate_size: 0,
                }),
            ),
            module: &module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &[
                    ("MUL_ELEM_ELEM", mul_elem_elem as f64),
                    ("MUL_ELEM_GRIP", mul_elem_grip as f64),
                    ("MUL_ELEM_AXIS", mul_elem_axis as f64),
                    ("INVERSE_ELEM", inverse_elem as f64),
                ],
                zero_initialize_workgroup_memory: true,
            },
            cache: None,
        });

        let lut_buffer = CachedBuffer::new("lut_buffer", &device, wgpu::BufferUsages::STORAGE)
            .with_data(&lut_texture_data);

        Self {
            lut_buffer,

            block_lists_buffer: CachedBuffer::new(
                "block_lists_buffer",
                &device,
                wgpu::BufferUsages::STORAGE,
            ),
            twists_buffer: CachedBuffer::new("twists_buffer", &device, wgpu::BufferUsages::STORAGE),
            output_buffer: CachedBuffer::new(
                "output_buffer",
                &device,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            ),
            download_buffer: CachedBuffer::new(
                "download_buffer",
                &device,
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            ),

            pipeline,

            queue,
            device,
        }
    }

    pub fn test_do_twist(&mut self, states: &[BlockList], twists: &[Twist]) -> Vec<Out> {
        let len = states.len() * twists.len();

        let block_lists_buffer = self.block_lists_buffer.with_data(states);
        let twists_buffer = self.twists_buffer.with_data(twists);
        let output_buffer = self.output_buffer.at_len(len);
        let download_buffer = self.download_buffer.at_len(len);

        let bind_group = self.device.create_bind_group(&bind_group_desc!(
            self.pipeline.get_bind_group_layout(0),
            [
                0 => self.lut_buffer.as_entire_binding(),
                1 => output_buffer.as_entire_binding(),
                2 => block_lists_buffer.as_entire_binding(),
                3 => twists_buffer.as_entire_binding(),
            ]
        ));

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());

        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        // Now we dispatch a series of workgroups. Each workgroup is a 3D grid of individual programs.
        //
        // We defined the workgroup size in the shader as 64x1x1. So in order to process all of our
        // inputs, we ceiling divide the number of inputs by 64. If the user passes 32 inputs, we will
        // dispatch 1 workgroups. If the user passes 65 inputs, we will dispatch 2 workgroups, etc.
        let workgroup_size = 256;
        let workgroup_count = (len as u32).div_ceil(workgroup_size);
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);

        drop(compute_pass);

        encoder.copy_buffer_to_buffer(&output_buffer, 0, &download_buffer, 0, output_buffer.size());

        self.queue.submit([encoder.finish()]);

        let download_buffer_slice = download_buffer.slice(..);
        download_buffer_slice.map_async(wgpu::MapMode::Read, |_| {});

        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();

        std::fs::write(
            "bytes.bin",
            bytemuck::cast_slice(&download_buffer_slice.get_mapped_range()),
        )
        .unwrap();
        bytemuck::cast_slice(&download_buffer_slice.get_mapped_range()).to_vec()
    }
}

struct ReadOnlyBuffer;
impl From<ReadOnlyBuffer> for wgpu::BindingType {
    fn from(_: ReadOnlyBuffer) -> Self {
        wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    }
}

struct WriteableBuffer;
impl From<WriteableBuffer> for wgpu::BindingType {
    fn from(_: WriteableBuffer) -> Self {
        wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    }
}
