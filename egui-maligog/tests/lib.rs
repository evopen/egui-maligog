use std::array::IntoIter;
use std::collections::BTreeMap;
use std::iter::FromIterator;
use std::time::Duration;

use maligog::vk;
use maligog::BufferView;

use maplit::btreemap;
use winit::platform::run_return::EventLoopExtRunReturn;
#[cfg(unix)]
use winit::platform::unix::EventLoopExtUnix;
#[cfg(windows)]
use winit::platform::windows::EventLoopExtWindows;

struct Engine {
    instance: maligog::Instance,
    device: maligog::Device,
    sampler: maligog::Sampler,
    buffer1: maligog::Buffer,
    buffer2: maligog::Buffer,
    image: maligog::Image,
    swapchain: maligog::Swapchain,
    image_view: maligog::ImageView,
    descriptor_set: maligog::DescriptorSet,
}

impl Engine {
    pub fn new(window: &winit::window::Window) -> Self {
        let entry = maligog::Entry::new().unwrap();
        let mut required_extensions = maligog::Surface::required_extensions();
        required_extensions.push(maligog::name::instance::Extension::ExtDebugUtils);
        let instance = entry.create_instance(&[], &&required_extensions);
        let pdevice = instance
            .enumerate_physical_device()
            .first()
            .unwrap()
            .to_owned();
        let device = pdevice.create_device(&[(pdevice.queue_families().first().unwrap(), &[1.0])]);
        let buffer1 = device.create_buffer(
            None,
            123,
            maligog::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR,
            maligog::MemoryLocation::GpuOnly,
        );
        dbg!(&buffer1);
        let buffer2 = device.create_buffer(
            Some("hello"),
            1,
            maligog::BufferUsageFlags::TRANSFER_DST | maligog::BufferUsageFlags::STORAGE_BUFFER,
            maligog::MemoryLocation::GpuToCpu,
        );
        dbg!(&buffer2);

        let buffer3 = device.create_buffer_init(
            Some("hastalavista"),
            &[1, 2, 3, 1, 51, 56, 4, 23, 1, 3],
            maligog::BufferUsageFlags::STORAGE_BUFFER,
            maligog::MemoryLocation::CpuToGpu,
        );
        dbg!(&buffer3);

        let _buffer4 = device.create_buffer_init(
            Some("gpu only"),
            &[1, 2, 3, 1, 51, 56, 4, 23, 1, 3, 65, 2, 2, 2, 3],
            maligog::BufferUsageFlags::UNIFORM_BUFFER,
            maligog::MemoryLocation::GpuOnly,
        );
        dbg!(&_buffer4);

        let set_layout = device.create_descriptor_set_layout(
            Some("descriptor_set_layout"),
            &[maligog::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: maligog::DescriptorType::UniformBuffer,
                stage_flags: maligog::ShaderStageFlags::VERTEX,
                descriptor_count: 1,
            }],
        );

        let pipeline_layout =
            device.create_pipeline_layout(Some("pipeline layout"), &[&set_layout], &[]);
        let descriptor_pool = device.create_descriptor_pool(
            &[maligog::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(2)
                .build()],
            1,
        );
        let sampler = device.create_sampler(Some("sampler"));
        let image = device.create_image(
            Some("this is an image"),
            vk::Format::B8G8R8A8_UNORM,
            200,
            200,
            maligog::ImageUsageFlags::STORAGE,
            maligog::MemoryLocation::GpuOnly,
        );
        let image1 = device.create_image_init(
            Some("initialized image"),
            vk::Format::R8_UINT,
            1,
            1,
            vk::ImageUsageFlags::empty(),
            maligog::MemoryLocation::GpuOnly,
            &[123],
        );
        image1.set_layout(
            maligog::ImageLayout::UNDEFINED,
            maligog::ImageLayout::GENERAL,
        );
        let image_view = image.create_view();
        let surface = instance.create_surface(window);
        let swapchain = device.create_swapchain(surface, maligog::PresentModeKHR::FIFO);
        let descriptor_set_layout = device.create_descriptor_set_layout(
            Some("temp descriptor set layout"),
            &[maligog::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: maligog::DescriptorType::StorageBuffer,
                stage_flags: maligog::ShaderStageFlags::ALL_GRAPHICS,
                descriptor_count: 2,
            }],
        );

        let descriptor_set = device.create_descriptor_set(
            Some("temp descriptor set"),
            &descriptor_pool,
            &descriptor_set_layout,
            btreemap! {
                0 => maligog::DescriptorUpdate::Buffer(vec![BufferView{buffer: buffer3.clone(), offset: 0},
                                                            BufferView{buffer: buffer2.clone(), offset: 0}]),
            },
        );
        Self {
            instance,
            device,
            sampler,
            buffer1,
            buffer2,
            image,
            swapchain,
            image_view,
            descriptor_set,
        }
    }
}

#[test]
fn test_general() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .try_init()
        .ok();

    let mut event_loop = winit::event_loop::EventLoop::<()>::new_any_thread();
    let win = winit::window::WindowBuilder::new()
        .build(&event_loop)
        .unwrap();
    let mut engine = Engine::new(&win);

    event_loop.run_return(|event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;
        match event {
            winit::event::Event::WindowEvent { window_id, event } => {
                match event {
                    winit::event::WindowEvent::CloseRequested => {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                    }
                    winit::event::WindowEvent::KeyboardInput {
                        device_id,
                        input,
                        is_synthetic,
                    } => todo!(),
                    _ => {}
                }
            }
            winit::event::Event::MainEventsCleared => {
                win.request_redraw();
            }
            winit::event::Event::RedrawRequested(_) => {
                let index = engine.swapchain.acquire_next_image().unwrap();

                engine.swapchain.get_image(index).set_layout(
                    maligog::ImageLayout::UNDEFINED,
                    maligog::ImageLayout::PRESENT_SRC_KHR,
                );
                engine
                    .swapchain
                    .present(index, &[&engine.swapchain.image_available_semaphore()]);
            }
            _ => {}
        }
    });
    engine.device.wait_idle();
}
