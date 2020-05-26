use crate::utils::{ResultValue, Value};

use std::{error::Error, sync::Arc};

use vulkano::{
    device::{Device, DeviceCreationError, DeviceExtensions, Features, Queue},
    format::Format,
    framebuffer::{RenderPassAbstract, RenderPassCreationError},
    image::{ImageUsage, SwapchainImage},
    instance::{
        debug::{DebugCallback, DebugCallbackCreationError, MessageSeverity, MessageType},
        ApplicationInfo, Instance, InstanceCreationError, PhysicalDevice, QueueFamily, Version,
    },
    swapchain::{
        ColorSpace, CompositeAlpha, FullscreenExclusive, PresentMode, Surface, SurfaceTransform,
        Swapchain, SwapchainCreationError,
    },
    sync::SharingMode,
};
use vulkano_win::{CreationError, VkSurfaceBuild};
use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

const WIDTH: i32 = 800;
const HEIGHT: i32 = 600;

pub fn create_instance() -> ResultValue<Arc<Instance>, InstanceCreationError> {
    let version = Version {
        major: 1,
        minor: 0,
        patch: 0,
    };

    let mut required_extensions = vulkano_win::required_extensions();
    let mut layers = Vec::new();
    if cfg!(debug_assertions) {
        required_extensions.ext_debug_utils = true;
        layers.push("VK_LAYER_LUNARG_standard_validation");
    }

    Ok(Value(Instance::new(
        Some(&ApplicationInfo {
            application_name: Some("Vulkan Application".into()),
            application_version: Some(version),
            engine_name: Some("No Engine".into()),
            engine_version: Some(version),
        }),
        &required_extensions,
        layers,
    )?))
}

pub fn create_debug_callback(
    instance: &Arc<Instance>,
) -> ResultValue<Option<DebugCallback>, DebugCallbackCreationError> {
    //
    if cfg!(debug_assertions) {
        Ok(Value(Some(DebugCallback::new(
            instance,
            MessageSeverity {
                error: true,
                warning: true,
                information: true,
                verbose: true,
            },
            MessageType::all(),
            |msg| {
                let message_severity = if msg.severity.error {
                    "error"
                } else if msg.severity.warning {
                    "warning"
                } else if msg.severity.information {
                    "information"
                } else if msg.severity.verbose {
                    "verbose"
                } else {
                    unimplemented!()
                };

                println!(
                    "validation layer: (severity: {}) {}",
                    message_severity, msg.description
                );
            },
        )?)))
    } else {
        Ok(Value(None))
    }
}

pub fn create_surface(
    instance: Arc<Instance>,
) -> ResultValue<(Arc<Surface<Window>>, EventLoop<()>), CreationError> {
    //
    let events_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .with_inner_size(LogicalSize {
            width: WIDTH,
            height: HEIGHT,
        })
        .with_title("Vulkan Application")
        .build_vk_surface(&events_loop, instance)?;
    Ok(Value((surface, events_loop)))
}

pub fn pick_queues_families<'a>(
    surface: &'a Arc<Surface<Window>>,
) -> ResultValue<(QueueFamily, QueueFamily), Box<dyn Error>> {
    //
    for physical_device in PhysicalDevice::enumerate(surface.instance()) {
        let queue_families: Vec<_> = physical_device.queue_families().collect::<_>();

        if let (Some(&graphics_queue_family), Some(&present_queue_family)) = (
            queue_families.iter().find(|&&q| q.supports_graphics()),
            queue_families
                .iter()
                .find(|&&q| surface.is_supported(q).unwrap_or(false)),
        ) {
            return Ok(Value((graphics_queue_family, present_queue_family)));
        }
    }
    Err("couldn't find a suitable physical device".into())
}

#[allow(clippy::type_complexity)]
pub fn create_device(
    graphics_queue_family: QueueFamily,
    present_queue_family: QueueFamily,
) -> ResultValue<(Arc<Device>, Arc<Queue>, Arc<Queue>), DeviceCreationError> {
    //
    let mut queue_families = vec![(graphics_queue_family, 1.0)];
    if graphics_queue_family.id() != present_queue_family.id() {
        queue_families.push((present_queue_family, 1.0));
    }

    let (device, queues) = {
        Device::new(
            graphics_queue_family.physical_device(),
            &Features::none(),
            &DeviceExtensions {
                khr_swapchain: true,
                ..DeviceExtensions::none()
            },
            queue_families,
        )?
    };
    let queues: Vec<_> = queues.collect::<_>();

    let graphics_queue = queues
        .iter()
        .find(|q| q.family() == graphics_queue_family)
        .unwrap()
        .to_owned();

    let present_queue = queues
        .iter()
        .find(|q| q.family() == present_queue_family)
        .unwrap()
        .to_owned();

    Ok(Value((device, graphics_queue, present_queue)))
}

#[allow(clippy::type_complexity)]
pub fn create_swapchain(
    surface: Arc<Surface<Window>>,
    device: Arc<Device>,
    graphics_queue: Arc<Queue>,
    present_queue: Arc<Queue>,
) -> ResultValue<(Arc<Swapchain<Window>>, Vec<Arc<SwapchainImage<Window>>>), SwapchainCreationError>
{
    let capabilities = surface.capabilities(device.physical_device()).unwrap();

    let usage = ImageUsage {
        color_attachment: true,
        ..ImageUsage::none()
    };

    let sharing_mode = if graphics_queue.family() != present_queue.family() {
        SharingMode::Concurrent(vec![
            graphics_queue.family().id(),
            present_queue.family().id(),
        ])
    } else {
        SharingMode::Exclusive
    };

    let num_images =
        (capabilities.min_image_count + 1).min(capabilities.max_image_count.unwrap_or(u32::MAX));

    let (format, color_space) = capabilities
        .supported_formats
        .iter()
        .find(|&&x| x == (Format::B8G8R8A8Unorm, ColorSpace::SrgbNonLinear))
        .cloned()
        .unwrap_or(capabilities.supported_formats[0]);

    let present_mode = if capabilities.present_modes.mailbox {
        PresentMode::Mailbox
    } else if capabilities.present_modes.immediate {
        PresentMode::Immediate
    } else {
        PresentMode::Fifo
    };

    Ok(Value(Swapchain::new(
        device,
        surface.clone(),
        num_images,
        format,
        surface.window().inner_size().into(),
        1,
        usage,
        sharing_mode,
        SurfaceTransform::Identity,
        CompositeAlpha::Opaque,
        present_mode,
        FullscreenExclusive::Default,
        true,
        color_space,
    )?))
}

pub fn create_render_pass(
    device: Arc<Device>,
    swapchain: Arc<Swapchain<Window>>,
) -> ResultValue<Arc<dyn RenderPassAbstract>, RenderPassCreationError> {
    Ok(Value(Arc::new(vulkano::single_pass_renderpass!(device,
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: swapchain.format(),
                samples: 1,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {}
        }
    )?)))
}
