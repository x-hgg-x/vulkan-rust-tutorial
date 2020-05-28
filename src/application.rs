use crate::init::*;
use crate::utils::BacktraceExt;

use std::error::Error;

use vulkano::command_buffer::DynamicState;

pub fn run() -> Result<(), Box<dyn Error>> {
    let instance = create_instance().debug()?;

    let _debug_callback = create_debug_callback(&instance).debug()?;

    let (surface, event_loop) = create_surface(instance).debug()?;

    let (graphics_queue_family, present_queue_family) = pick_queues_families(&surface).debug()?;

    let (device, graphics_queue, present_queue) =
        create_device(graphics_queue_family, present_queue_family).debug()?;

    let (swapchain, swapchain_images) = create_swapchain(
        surface.clone(),
        device.clone(),
        graphics_queue,
        present_queue,
    )
    .debug()?;

    let vertex_buffer = create_vertex_buffer(device.clone()).debug()?;

    let render_pass = create_render_pass(device.clone(), swapchain.clone()).debug()?;

    let pipeline = create_pipeline(device, render_pass.clone()).debug()?;

    let mut dynamic_state = DynamicState::none();
    update_dynamic_viewport(swapchain, &mut dynamic_state);

    let framebuffers = create_framebuffers(swapchain_images, render_pass.clone()).debug()?;

    Ok(())
}
