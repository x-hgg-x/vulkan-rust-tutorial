use crate::lib::*;

use std::sync::Arc;

use vulkano::{
    buffer::{BufferUsage, ImmutableBuffer},
    command_buffer::DynamicState,
    device::{Device, DeviceExtensions, Features, Queue},
    format::Format,
    framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract, Subpass},
    image::{
        view::ImageView, AttachmentImage, ImageDimensions, ImageUsage, ImmutableImage,
        MipmapsCount, SwapchainImage,
    },
    instance::{
        debug::{DebugCallback, MessageSeverity, MessageType},
        ApplicationInfo, Instance, PhysicalDevice, QueueFamily, Version,
    },
    pipeline::{viewport::Viewport, GraphicsPipeline, GraphicsPipelineAbstract},
    sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode},
    swapchain::{
        ColorSpace, CompositeAlpha, FullscreenExclusive, PresentMode, Surface, SurfaceTransform,
        Swapchain,
    },
    sync::{GpuFuture, SharingMode},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use image::GenericImageView;

use color_eyre::{eyre::eyre, Result};

pub fn create_instance() -> Result<Arc<Instance>> {
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

    Ok(Instance::new(
        Some(&ApplicationInfo {
            application_name: Some("Vulkan Application".into()),
            application_version: Some(version),
            engine_name: Some("No Engine".into()),
            engine_version: Some(version),
        }),
        &required_extensions,
        layers,
    )?)
}

pub fn create_debug_callback(instance: &Arc<Instance>) -> Result<Option<DebugCallback>> {
    if cfg!(debug_assertions) {
        Ok(Some(DebugCallback::new(
            instance,
            MessageSeverity::errors_and_warnings(),
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
        )?))
    } else {
        Ok(None)
    }
}

pub fn create_surface(instance: Arc<Instance>) -> Result<(Arc<Surface<Window>>, EventLoop<()>)> {
    let events_loop = EventLoop::new();

    let surface = WindowBuilder::new()
        .with_inner_size(LogicalSize {
            width: WIDTH,
            height: HEIGHT,
        })
        .with_title("Vulkan Application")
        .build_vk_surface(&events_loop, instance)?;

    Ok((surface, events_loop))
}

pub fn pick_queues_families(surface: &Arc<Surface<Window>>) -> Result<(QueueFamily, QueueFamily)> {
    for physical_device in PhysicalDevice::enumerate(surface.instance()) {
        let queue_families: Vec<_> = physical_device.queue_families().collect();

        if let (Some(&graphics_queue_family), Some(&present_queue_family)) = (
            queue_families.iter().find(|&&q| q.supports_graphics()),
            queue_families
                .iter()
                .find(|&&q| surface.is_supported(q).unwrap_or(false)),
        ) {
            return Ok((graphics_queue_family, present_queue_family));
        }
    }
    Err(eyre!("couldn't find a suitable physical device"))
}

pub fn create_device(
    graphics_queue_family: QueueFamily,
    present_queue_family: QueueFamily,
) -> Result<(Arc<Device>, Arc<Queue>, Arc<Queue>)> {
    //
    let mut queue_families = vec![(graphics_queue_family, 1.0)];
    if graphics_queue_family.id() != present_queue_family.id() {
        queue_families.push((present_queue_family, 1.0));
    }

    let (device, queues) = {
        Device::new(
            graphics_queue_family.physical_device(),
            &Features {
                sampler_anisotropy: true,
                ..Features::none()
            },
            &DeviceExtensions {
                khr_swapchain: true,
                ..DeviceExtensions::none()
            },
            queue_families,
        )?
    };
    let queues: Vec<_> = queues.collect();

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

    Ok((device, graphics_queue, present_queue))
}

#[allow(clippy::type_complexity)]
pub fn create_swapchain(
    surface: Arc<Surface<Window>>,
    device: Arc<Device>,
    graphics_queue: Arc<Queue>,
    present_queue: Arc<Queue>,
) -> Result<(Arc<Swapchain<Window>>, Vec<Arc<SwapchainImage<Window>>>)> {
    let capabilities = surface.capabilities(device.physical_device())?;

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
        .find(|&&x| x == (Format::B8G8R8A8Srgb, ColorSpace::SrgbNonLinear))
        .cloned()
        .unwrap_or(capabilities.supported_formats[0]);

    let present_mode = if capabilities.present_modes.mailbox {
        PresentMode::Mailbox
    } else if capabilities.present_modes.immediate {
        PresentMode::Immediate
    } else {
        PresentMode::Fifo
    };

    Ok(Swapchain::new(
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
    )?)
}

pub fn create_buffers(graphics_queue: Arc<Queue>) -> Result<(VertexBuffer, IndexBuffer)> {
    let (models, _) = tobj::load_obj("models/chalet.obj", true)?;
    let mesh = &models[0].mesh;

    let (vertex_buffer, vertex_future) = ImmutableBuffer::from_iter(
        mesh.positions
            .chunks_exact(3)
            .zip(mesh.texcoords.chunks_exact(2))
            .map(|(pos, tex)| Vertex {
                position: [pos[0], pos[1], pos[2]],
                texture_coords: [tex[0], 1.0 - tex[1]],
            }),
        BufferUsage::vertex_buffer(),
        graphics_queue.clone(),
    )?;

    let (index_buffer, index_future) = ImmutableBuffer::from_iter(
        mesh.indices.iter().cloned(),
        BufferUsage::index_buffer(),
        graphics_queue,
    )?;

    vertex_future
        .join(index_future)
        .then_signal_fence_and_flush()?
        .cleanup_finished();

    Ok((vertex_buffer, index_buffer))
}

pub fn load_texture(graphics_queue: Arc<Queue>) -> Result<Arc<ImmutableImage<Format>>> {
    let img = image::open("textures/chalet.jpg")?;
    let (width, height) = img.dimensions();

    let (texture, texture_future) = ImmutableImage::from_iter(
        img.to_bytes().into_iter(),
        ImageDimensions::Dim2d {
            width,
            height,
            array_layers: 1,
        },
        MipmapsCount::One,
        Format::R8G8B8Srgb,
        graphics_queue,
    )?;

    texture_future
        .then_signal_fence_and_flush()?
        .cleanup_finished();

    Ok(texture)
}

pub fn create_sampler(device: Arc<Device>) -> Result<Arc<Sampler>> {
    let sampler = Sampler::new(
        device.clone(),
        Filter::Linear,
        Filter::Linear,
        MipmapMode::Linear,
        SamplerAddressMode::Repeat,
        SamplerAddressMode::Repeat,
        SamplerAddressMode::Repeat,
        0.0,
        device.physical_device().limits().max_sampler_anisotropy(),
        0.0,
        1000.0,
    )?;
    Ok(sampler)
}

pub fn create_render_pass(
    device: Arc<Device>,
    swapchain: Arc<Swapchain<Window>>,
) -> Result<Arc<dyn RenderPassAbstract + Send + Sync>> {
    //
    Ok(Arc::new(vulkano::single_pass_renderpass!(device,
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: swapchain.format(),
                samples: 1,
            },
            depth: {
                load: Clear,
                store: DontCare,
                format: Format::D32Sfloat,
                samples: 1,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {depth}
        }
    )?))
}

pub fn create_pipeline(
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
) -> Result<Arc<dyn GraphicsPipelineAbstract + Send + Sync>> {
    //
    let device = render_pass.device();
    Ok(Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(vs::Shader::load(device.clone())?.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs::Shader::load(device.clone())?.main_entry_point(), ())
            .depth_stencil_simple_depth()
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())?,
    ))
}

pub fn update_dynamic_viewport(
    swapchain: Arc<Swapchain<Window>>,
    dynamic_state: &mut DynamicState,
) {
    //
    const RATIO: f32 = WIDTH as f32 / HEIGHT as f32;

    let dimensions = swapchain.dimensions();
    let (mut width, mut height) = (dimensions[0] as f32, dimensions[1] as f32);

    if width / height > RATIO {
        width = RATIO * height;
    } else {
        height = width / RATIO;
    }

    let dimensions = swapchain.dimensions();
    dynamic_state.viewports = Some(vec![Viewport {
        origin: [
            (dimensions[0] as f32 - width) / 2.0,
            (dimensions[1] as f32 - height) / 2.0,
        ],
        dimensions: [width, height],
        depth_range: 0.0..1.0,
    }]);
}

pub fn create_framebuffers(
    swapchain_images: Vec<Arc<SwapchainImage<Window>>>,
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
) -> Result<Vec<Arc<dyn FramebufferAbstract + Send + Sync>>> {
    //
    let depth_buffer = AttachmentImage::transient(
        render_pass.device().clone(),
        swapchain_images[0].dimensions(),
        Format::D32Sfloat,
    )?;

    let mut framebuffers = Vec::<Arc<dyn FramebufferAbstract + Send + Sync>>::new();
    for image in swapchain_images {
        framebuffers.push(Arc::new(
            Framebuffer::start(render_pass.clone())
                .add(ImageView::new(image.clone())?)?
                .add(ImageView::new(depth_buffer.clone())?)?
                .build()?,
        ));
    }
    Ok(framebuffers)
}
