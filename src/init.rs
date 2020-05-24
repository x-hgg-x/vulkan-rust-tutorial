use crate::utils::{ResultValue, Value};

use std::{error::Error, sync::Arc};

use vulkano::{
    device::{Device, DeviceCreationError, DeviceExtensions, Features, Queue},
    instance::PhysicalDevice,
    instance::{
        debug::{DebugCallback, DebugCallbackCreationError, MessageSeverity, MessageType},
        ApplicationInfo, Instance, InstanceCreationError, QueueFamily, Version,
    },
    swapchain::Surface,
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

pub fn pick_physical_device<'a>(
    instance: &'a Arc<Instance>,
    surface: Arc<Surface<Window>>,
) -> ResultValue<(PhysicalDevice, QueueFamily, QueueFamily), Box<dyn Error>> {
    //
    let mut graphics_queue_family = None;
    let mut present_queue_family = None;

    let physical_device = PhysicalDevice::enumerate(&instance)
        .find(|device| {
            let queue_families: Vec<_> = device.queue_families().collect::<_>();

            match queue_families.iter().find(|&q| q.supports_graphics()) {
                Some(&queue_family) => graphics_queue_family = Some(queue_family),
                _ => return false,
            };

            match queue_families
                .into_iter()
                .find(|&q| surface.is_supported(q).unwrap_or(false))
            {
                Some(queue_family) => present_queue_family = Some(queue_family),
                _ => return false,
            };

            true
        })
        .ok_or_else(|| "couldn't find a suitable physical device")?;

    Ok(Value((
        physical_device,
        graphics_queue_family.unwrap(),
        present_queue_family.unwrap(),
    )))
}

#[allow(clippy::type_complexity)]
pub fn create_device(
    physical_device: PhysicalDevice,
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
            physical_device,
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
