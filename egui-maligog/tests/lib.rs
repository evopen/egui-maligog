use std::array::IntoIter;
use std::collections::BTreeMap;
use std::iter::FromIterator;
use std::time::Duration;

use egui_maligog::ScreenDescriptor;
use egui_maligog::UiPass;
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
    image: maligog::Image,
    swapchain: maligog::Swapchain,
    ui_pass: egui_maligog::UiPass,
    width: u32,
    height: u32,
    scale_factor: f64,
}

impl Engine {
    pub fn new(window: &winit::window::Window) -> Self {
        let width = window.inner_size().width;
        let height = window.inner_size().height;
        let scale_factor = window.scale_factor();

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

        let ui_pass = UiPass::new(&device);
        Self {
            instance,
            device,
            sampler,
            image,
            swapchain,
            ui_pass,
            width,
            height,
            scale_factor,
        }
    }

    pub fn render(&mut self) {
        let mut cmd_buf = self
            .device
            .create_command_buffer(self.device.graphics_queue_family_index());
        cmd_buf.encode(|recorder| {
            let index = self.swapchain.acquire_next_image().unwrap();

            // self.swapchain.get_image(index).set_layout(
            //     maligog::ImageLayout::UNDEFINED,
            //     maligog::ImageLayout::PRESENT_SRC_KHR,
            // );
            let image = self.swapchain.get_image(index);

            self.ui_pass.execute(
                recorder,
                &image,
                &[],
                &ScreenDescriptor {
                    physical_width: self.width,
                    physical_height: self.height,
                    scale_factor: self.scale_factor as f32,
                },
            );
            self.swapchain
                .present(index, &[&self.swapchain.image_available_semaphore()]);
        });
        self.device.graphics_queue().submit_blocking()
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
                engine.render();
            }
            _ => {}
        }
    });
    engine.device.wait_idle();
}
