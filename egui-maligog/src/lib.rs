use bytemuck::{Pod, Zeroable};

const SHADER: &[u8] = include_bytes!(env!("shader.spv"));

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct UniformBuffer {
    screen_size: [f32; 2],
}

struct UiPass {}

impl UiPass {
    pub fn new(device: maligog::Device) -> Self {
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
                },
                maligog::DescriptorSetLayoutBinding {
                    binding: 1,
                    descriptor_type: maligog::DescriptorType::Sampler(None),
                    stage_flags: maligog::ShaderStageFlags::FRAGMENT,
                },
            ],
        );

        let texture_descriptor_set_layout = device.create_descriptor_set_layout(
            Some("texture"),
            &[maligog::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: maligog::DescriptorType::SampledImage,
                stage_flags: maligog::ShaderStageFlags::FRAGMENT,
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

        let graphics_pipeline = device.create_graphics_pipeline(
            Some("egui pipeline"),
            pipeline_layout,
            vec![
                Arc::new(safe_vk::ShaderStage::new(
                    Arc::new(vs_module),
                    vk::ShaderStageFlags::VERTEX,
                    "main",
                )),
                Arc::new(safe_vk::ShaderStage::new(
                    Arc::new(fs_module),
                    vk::ShaderStageFlags::FRAGMENT,
                    "main",
                )),
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

        let mut uniform_descriptor_set = safe_vk::DescriptorSet::new(
            Some("uniform descriptor set"),
            descriptor_pool.clone(),
            uniform_descriptor_set_layout.clone(),
        );
        uniform_descriptor_set.update(&[
            safe_vk::DescriptorSetUpdateInfo {
                binding: 0,
                detail: safe_vk::DescriptorSetUpdateDetail::Buffer {
                    buffer: uniform_buffer.clone(),
                    offset: 0,
                },
            },
            safe_vk::DescriptorSetUpdateInfo {
                binding: 1,
                detail: safe_vk::DescriptorSetUpdateDetail::Sampler(sampler.clone()),
            },
        ]);

        Self {}
    }
}
