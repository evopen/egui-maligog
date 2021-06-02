use bytemuck::{Pod, Zeroable};

use maligog::{vk, BufferView, DescriptorSet, Device};
use maplit::btreemap;

const SHADER: &[u8] = include_bytes!(env!("shader.spv"));

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct UniformBuffer {
    screen_size: [f32; 2],
}

pub struct ScreenDescriptor {
    /// Width of the window in physical pixel.
    pub physical_width: u32,
    /// Height of the window in physical pixel.
    pub physical_height: u32,
    /// HiDPI scale factor.
    pub scale_factor: f32,
}

impl ScreenDescriptor {
    fn logical_size(&self) -> (u32, u32) {
        let logical_width = self.physical_width as f32 / self.scale_factor;
        let logical_height = self.physical_height as f32 / self.scale_factor;
        (logical_width as u32, logical_height as u32)
    }
}

pub struct UiPass {
    device: Device,
    graphics_pipeline: maligog::GraphicsPipeline,
    index_buffers: Vec<maligog::Buffer>,
    vertex_buffers: Vec<maligog::Buffer>,
    uniform_buffer: maligog::Buffer,
    uniform_descriptor_set: maligog::DescriptorSet,
    texture_descriptor_set_layout: maligog::DescriptorSetLayout,
    texture_descriptor_set: Option<maligog::DescriptorSet>,
    texture_version: Option<u64>,
    next_user_texture_id: u64,
    pending_user_textures: Vec<(u64, egui::Texture)>,
    user_textures: Vec<Option<maligog::DescriptorSet>>,
    render_pass: maligog::RenderPass,
    descriptor_pool: maligog::DescriptorPool,
}

impl UiPass {
    pub fn new(device: &maligog::Device) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let uniform_buffer = device.create_buffer(
            Some("uniform buffer"),
            std::mem::size_of::<UniformBuffer>(),
            maligog::BufferUsageFlags::UNIFORM_BUFFER | maligog::BufferUsageFlags::TRANSFER_DST,
            maligog::MemoryLocation::CpuToGpu,
        );
        let sampler = device.create_sampler(Some("egui sampler"));

        let uniform_descriptor_set_layout = device.create_descriptor_set_layout(
            Some("uniform"),
            &[
                maligog::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: maligog::DescriptorType::UniformBuffer,
                    stage_flags: maligog::ShaderStageFlags::VERTEX,
                    descriptor_count: 1,
                },
                maligog::DescriptorSetLayoutBinding {
                    binding: 1,
                    descriptor_type: maligog::DescriptorType::Sampler(None),
                    stage_flags: maligog::ShaderStageFlags::FRAGMENT,
                    descriptor_count: 1,
                },
            ],
        );

        let texture_descriptor_set_layout = device.create_descriptor_set_layout(
            Some("texture"),
            &[maligog::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: maligog::DescriptorType::SampledImage,
                stage_flags: maligog::ShaderStageFlags::FRAGMENT,
                descriptor_count: 1,
            }],
        );

        let pipeline_layout = device.create_pipeline_layout(
            Some("egui pipeline layout"),
            &[
                &uniform_descriptor_set_layout,
                &texture_descriptor_set_layout,
            ],
            &[],
        );

        let render_pass = device.create_render_pass(
            &vk::RenderPassCreateInfo::builder()
                .attachments(&[vk::AttachmentDescription::builder()
                    .format(vk::Format::B8G8R8A8_UNORM)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .load_op(vk::AttachmentLoadOp::LOAD)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .build()])
                .subpasses(&[vk::SubpassDescription::builder()
                    .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                    .color_attachments(&[vk::AttachmentReference::builder()
                        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .attachment(0)
                        .build()])
                    .build()])
                .build(),
        );

        let render_pass = device.create_render_pass(
            &vk::RenderPassCreateInfo::builder()
                .attachments(&[vk::AttachmentDescription::builder()
                    .format(vk::Format::B8G8R8A8_UNORM)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .load_op(vk::AttachmentLoadOp::LOAD)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .build()])
                .subpasses(&[vk::SubpassDescription::builder()
                    .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                    .color_attachments(&[vk::AttachmentReference::builder()
                        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .attachment(0)
                        .build()])
                    .build()])
                .build(),
        );

        let graphics_pipeline = device.create_graphics_pipeline(
            Some("egui pipeline"),
            pipeline_layout,
            vec![
                maligog::ShaderStage::new(
                    shader_module.clone(),
                    maligog::ShaderStageFlags::VERTEX,
                    "main_vs",
                ),
                maligog::ShaderStage::new(
                    shader_module.clone(),
                    maligog::ShaderStageFlags::FRAGMENT,
                    "main_fs",
                ),
            ],
            render_pass.clone(),
            &vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_binding_descriptions(&[vk::VertexInputBindingDescription::builder()
                    .stride(5 * 4)
                    .input_rate(vk::VertexInputRate::VERTEX)
                    .binding(0)
                    .build()])
                .vertex_attribute_descriptions(&[
                    vk::VertexInputAttributeDescription::builder()
                        .binding(0)
                        .location(0)
                        .format(vk::Format::R32G32_SFLOAT)
                        .offset(0)
                        .build(),
                    vk::VertexInputAttributeDescription::builder()
                        .binding(0)
                        .location(1)
                        .format(vk::Format::R32G32_SFLOAT)
                        .offset(4 * 2)
                        .build(),
                    vk::VertexInputAttributeDescription::builder()
                        .binding(0)
                        .location(2)
                        .format(vk::Format::R32_UINT)
                        .offset(4 * 4)
                        .build(),
                ])
                .build(),
            &vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                .build(),
            &vk::PipelineRasterizationStateCreateInfo::builder()
                .cull_mode(vk::CullModeFlags::NONE)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .polygon_mode(vk::PolygonMode::FILL)
                .line_width(1.0)
                .build(),
            &vk::PipelineMultisampleStateCreateInfo::builder()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1)
                .build(),
            &vk::PipelineDepthStencilStateCreateInfo::default(),
            &vk::PipelineColorBlendStateCreateInfo::builder()
                .attachments(&[vk::PipelineColorBlendAttachmentState::builder()
                    .blend_enable(true)
                    .color_blend_op(vk::BlendOp::ADD)
                    .src_color_blend_factor(vk::BlendFactor::ONE)
                    .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                    .alpha_blend_op(vk::BlendOp::ADD)
                    .src_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_DST_ALPHA)
                    .dst_alpha_blend_factor(vk::BlendFactor::ONE)
                    .color_write_mask(vk::ColorComponentFlags::all())
                    .build()])
                .build(),
            &vk::PipelineViewportStateCreateInfo::builder()
                .viewport_count(1)
                .scissor_count(1),
            &vk::PipelineDynamicStateCreateInfo::builder()
                .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
                .build(),
        );

        let descriptor_pool = device.create_descriptor_pool(
            &[vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .build()],
            1,
        );

        let uniform_descriptor_set = device.create_descriptor_set(
            Some("uniform descriptor set"),
            &descriptor_pool,
            &uniform_descriptor_set_layout,
            btreemap! {
                0 => maligog::DescriptorUpdate::Buffer(vec![BufferView { buffer: uniform_buffer.clone(), offset: 0 }]),
                1 => maligog::DescriptorUpdate::Sampler(sampler.clone())
            },
        );

        Self {
            device: device.clone(),
            graphics_pipeline,
            index_buffers: Vec::with_capacity(64),
            vertex_buffers: Vec::with_capacity(64),
            uniform_buffer,
            uniform_descriptor_set,
            texture_descriptor_set_layout,
            texture_descriptor_set: None,
            texture_version: None,
            next_user_texture_id: 0,
            pending_user_textures: Vec::new(),
            user_textures: Vec::new(),
            render_pass,
            descriptor_pool,
        }
    }

    pub fn execute(
        &mut self,
        recorder: &mut maligog::CommandRecorder,
        color_attachment: &maligog::Image,
        paint_jobs: &[egui::paint::ClippedMesh],
        screen_descriptor: &ScreenDescriptor,
    ) {
        let image_view = color_attachment.create_view();
        let framebuffer = self.device.create_framebuffer(
            self.render_pass.clone(),
            screen_descriptor.physical_width,
            screen_descriptor.physical_height,
            vec![&image_view],
        );

        let scale_factor = screen_descriptor.scale_factor;
        let physical_width = screen_descriptor.physical_width;
        let physical_height = screen_descriptor.physical_height;

        recorder.begin_render_pass(&self.render_pass, &framebuffer, |recorder| {
            recorder.bind_graphics_pipeline(&self.graphics_pipeline, |recorder| {
                recorder.bind_descriptor_sets(vec![&self.uniform_descriptor_set], 0);
                for ((egui::ClippedMesh(clip_rect, mesh), vertex_buffer), index_buffer) in
                    paint_jobs
                        .iter()
                        .zip(self.vertex_buffers.iter())
                        .zip(self.index_buffers.iter())
                {
                    // Transform clip rect to physical pixels.
                    let clip_min_x = scale_factor * clip_rect.min.x;
                    let clip_min_y = scale_factor * clip_rect.min.y;
                    let clip_max_x = scale_factor * clip_rect.max.x;
                    let clip_max_y = scale_factor * clip_rect.max.y;

                    // Make sure clip rect can fit within an `u32`.
                    let clip_min_x = clip_min_x.clamp(0.0, physical_width as f32);
                    let clip_min_y = clip_min_y.clamp(0.0, physical_height as f32);
                    let clip_max_x = clip_max_x.clamp(clip_min_x, physical_width as f32);
                    let clip_max_y = clip_max_y.clamp(clip_min_y, physical_height as f32);

                    let clip_min_x = clip_min_x.round() as u32;
                    let clip_min_y = clip_min_y.round() as u32;
                    let clip_max_x = clip_max_x.round() as u32;
                    let clip_max_y = clip_max_y.round() as u32;

                    let width = (clip_max_x - clip_min_x).max(1);
                    let height = (clip_max_y - clip_min_y).max(1);

                    {
                        // clip scissor rectangle to target size
                        let x = clip_min_x.min(physical_width);
                        let y = clip_min_y.min(physical_height);
                        let width = width.min(physical_width - x);
                        let height = height.min(physical_height - y);

                        // skip rendering with zero-sized clip areas
                        if width == 0 || height == 0 {
                            continue;
                        }

                        recorder.set_scissor(&[vk::Rect2D {
                            offset: vk::Offset2D {
                                x: x as i32,
                                y: y as i32,
                            },
                            extent: vk::Extent2D { width, height },
                        }]);
                        recorder.set_viewport(vk::Viewport {
                            x: 0.0,
                            y: physical_height as f32,
                            width: physical_width as f32,
                            height: -(physical_height as f32),
                            min_depth: 0.1,
                            max_depth: 1.0,
                        });
                    }
                    recorder.bind_descriptor_sets(
                        vec![&self.get_texture_descriptor_set(mesh.texture_id)],
                        1,
                    );

                    recorder.bind_index_buffer(index_buffer.clone(), 0, vk::IndexType::UINT32);
                    recorder.bind_vertex_buffer(vec![vertex_buffer.clone()], &[0]);
                    recorder.draw_indexed(mesh.indices.len() as u32, 1);
                }
            });
        });
    }

    fn get_texture_descriptor_set(&self, texture_id: egui::TextureId) -> maligog::DescriptorSet {
        match texture_id {
            egui::TextureId::Egui => {
                self.texture_descriptor_set
                    .as_ref()
                    .expect("egui texture was not set before the first draw")
                    .clone()
            }
            egui::TextureId::User(id) => {
                let id = id as usize;
                assert!(id < self.user_textures.len());
                self.user_textures
                    .get(id)
                    .unwrap_or_else(|| panic!("user texture {} not found", id))
                    .as_ref()
                    .unwrap_or_else(|| panic!("user texture {} freed", id))
                    .clone()
            }
        }
    }

    pub fn update_texture(&mut self, egui_texture: &egui::Texture) {
        // Don't update the texture if it hasn't changed.
        if self.texture_version == Some(egui_texture.version) {
            return;
        }
        // we need to convert the texture into rgba format
        let egui_texture = egui::Texture {
            version: egui_texture.version,
            width: egui_texture.width,
            height: egui_texture.height,
            pixels: egui_texture
                .pixels
                .iter()
                .flat_map(|p| std::iter::repeat(*p).take(4))
                .collect(),
        };
        let descriptor_set = self.egui_texture_to_gpu(&egui_texture);

        self.texture_version = Some(egui_texture.version);
        self.texture_descriptor_set = Some(descriptor_set);
    }

    fn egui_texture_to_gpu(&mut self, egui_texture: &egui::Texture) -> DescriptorSet {
        let image = self.device.create_image_init(
            Some("egui texture"),
            vk::Format::B8G8R8A8_UNORM,
            egui_texture.width as u32,
            egui_texture.height as u32,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            maligog::MemoryLocation::GpuOnly,
            egui_texture.pixels.as_slice(),
        );

        image.set_layout(
            maligog::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );

        let mut descriptor_set = self.device.create_descriptor_set(
            Some("texture descriptor set"),
            &self.descriptor_pool,
            &self.texture_descriptor_set_layout,
            btreemap! {
                0 => maligog::DescriptorUpdate::Image(vec![image.create_view()])
            },
        );

        descriptor_set
    }

    pub fn update_buffers(
        &mut self,
        paint_jobs: &[egui::paint::ClippedMesh],
        screen_descriptor: &ScreenDescriptor,
    ) {
        let index_size = self.index_buffers.len();
        let vertex_size = self.vertex_buffers.len();

        let (logical_width, logical_height) = screen_descriptor.logical_size();

        self.uniform_buffer
            .copy_from(bytemuck::cast_slice(&[UniformBuffer {
                screen_size: [logical_width as f32, logical_height as f32],
            }]));

        for (i, egui::ClippedMesh(_, mesh)) in paint_jobs.iter().enumerate() {
            let data: &[u8] = bytemuck::cast_slice(&mesh.indices);
            if i < index_size {
                if self.index_buffers[i].size() != data.len() {
                    self.index_buffers[i] = self.device.create_buffer_init(
                        Some("index buffer"),
                        data,
                        vk::BufferUsageFlags::INDEX_BUFFER,
                        maligog::MemoryLocation::CpuToGpu,
                    );
                } else {
                    self.index_buffers[i].copy_from(data);
                }
            } else {
                let buffer = self.device.create_buffer_init(
                    Some("index buffer"),
                    data,
                    vk::BufferUsageFlags::INDEX_BUFFER,
                    maligog::MemoryLocation::CpuToGpu,
                );
                self.index_buffers.push(buffer);
            }

            let data: &[u8] = as_byte_slice(&mesh.vertices);
            if i < vertex_size {
                if self.vertex_buffers[i].size() != data.len() {
                    self.vertex_buffers[i] = self.device.create_buffer_init(
                        Some("vertex buffer"),
                        data,
                        vk::BufferUsageFlags::VERTEX_BUFFER,
                        maligog::MemoryLocation::CpuToGpu,
                    );
                } else {
                    self.vertex_buffers[i].copy_from(data);
                }
            } else {
                let buffer = self.device.create_buffer_init(
                    Some("vertex buffer"),
                    data,
                    vk::BufferUsageFlags::VERTEX_BUFFER,
                    maligog::MemoryLocation::CpuToGpu,
                );
                self.vertex_buffers.push(buffer);
            }
        }
    }
}

// Needed since we can't use bytemuck for external types.
fn as_byte_slice<T>(slice: &[T]) -> &[u8] {
    let len = slice.len() * std::mem::size_of::<T>();
    let ptr = slice.as_ptr() as *const u8;
    unsafe { std::slice::from_raw_parts(ptr, len) }
}
