use crate::event_loop::main_loop;
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

    let (mut swapchain, swapchain_images) = create_swapchain(
        surface.clone(),
        device.clone(),
        graphics_queue.clone(),
        present_queue.clone(),
    )
    .debug()?;

    let vertex_buffer = create_vertex_buffer(device.clone()).debug()?;

    let render_pass = create_render_pass(device, swapchain.clone()).debug()?;

    let pipeline = create_pipeline(render_pass.clone()).debug()?;

    let mut dynamic_state = DynamicState::none();
    update_dynamic_viewport(swapchain.clone(), &mut dynamic_state);

    let mut framebuffers = create_framebuffers(swapchain_images, render_pass.clone()).debug()?;

    let mut swapchain_out_of_date = false;
    let mut previous_frame_future = None;

    event_loop.run(move |event, _, control_flow| {
        main_loop(
            event,
            control_flow,
            graphics_queue.clone(),
            present_queue.clone(),
            vertex_buffer.clone(),
            render_pass.clone(),
            pipeline.clone(),
            &mut swapchain,
            &mut dynamic_state,
            &mut framebuffers,
            &mut swapchain_out_of_date,
            &mut previous_frame_future,
        )
        .unwrap_or_else(|e| {
            println!("\nError when running main loop: {}\n", e);
            std::process::exit(1);
        });
    });
}
