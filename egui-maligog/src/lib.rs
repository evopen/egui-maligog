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

        Self {}
    }
}
