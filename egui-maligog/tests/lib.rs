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
    scale_factor: f32,
    egui_instance: egui_winit_platform::Platform,
    start_time: std::time::Instant,
    paint_jobs: Vec<egui::ClippedMesh>,
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
        let device = pdevice.create_device();

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
        let egui_instance =
            egui_winit_platform::Platform::new(egui_winit_platform::PlatformDescriptor {
                physical_width: width,
                physical_height: height,
                scale_factor: scale_factor,
                font_definitions: egui::FontDefinitions::default(),
                style: egui::Style::default(),
            });
        let start_time = std::time::Instant::now();
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
            egui_instance,
            start_time,
            paint_jobs: Vec::new(),
        }
    }

    pub fn update(&mut self, event: &winit::event::Event<()>) {
        self.egui_instance.handle_event(event);

        self.egui_instance
            .update_time(self.start_time.elapsed().as_secs_f64());
        self.egui_instance.begin_frame();
        egui::TopPanel::top(egui::Id::new("menu bar")).show(
            &self.egui_instance.context().clone(),
            |ui| {
                egui::menu::bar(ui, |ui| {
                    egui::menu::menu(ui, "File", |ui| {
                        if ui.button("Organize Windows").clicked() {
                            ui.ctx().memory().reset_areas();
                        }
                    });
                });
            },
        );
        let (_, paint_commands) = self.egui_instance.end_frame();
        self.paint_jobs = self.egui_instance.context().tessellate(paint_commands);
        self.ui_pass.update_buffers(
            &self.paint_jobs,
            &ScreenDescriptor {
                physical_width: self.width,
                physical_height: self.height,
                scale_factor: self.scale_factor as f32,
            },
        );
        self.ui_pass
            .update_texture(&self.egui_instance.context().texture());
    }

    pub fn render(&mut self) {
        let mut cmd_buf = self.device.create_command_buffer(
            Some("frame command buffer"),
            self.device.graphics_queue_family_index(),
        );
        let index = self.swapchain.acquire_next_image().unwrap();
        let image = self.swapchain.get_image(index);
        cmd_buf.encode(|recorder| {
            // self.swapchain.get_image(index).set_layout(
            //     maligog::ImageLayout::UNDEFINED,
            //     maligog::ImageLayout::PRESENT_SRC_KHR,
            // );

            self.ui_pass.execute(
                recorder,
                &image,
                &self.paint_jobs,
                &ScreenDescriptor {
                    physical_width: self.width,
                    physical_height: self.height,
                    scale_factor: self.scale_factor as f32,
                },
                Some(vk::ClearColorValue {
                    float32: [1.0, 1.0, 1.0, 1.0],
                }),
            );
        });
        self.device.graphics_queue().submit_blocking(&[cmd_buf]);
        image.set_layout(
            vk::ImageLayout::ATTACHMENT_OPTIMAL_KHR,
            vk::ImageLayout::PRESENT_SRC_KHR,
        );
        self.swapchain.present(index, &[]);
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
        engine.update(&event);

        *control_flow = winit::event_loop::ControlFlow::Poll;
        match event {
            winit::event::Event::WindowEvent { window_id, event } => {
                match event {
                    winit::event::WindowEvent::CloseRequested => {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                    }
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
