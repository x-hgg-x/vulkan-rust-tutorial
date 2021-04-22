mod event_loop;
mod init;
mod lib;

use crate::event_loop::main_loop;
use crate::init::*;
use crate::lib::*;

use std::time::Instant;

use vulkano::{
    buffer::CpuBufferPool, command_buffer::DynamicState,
    descriptor::descriptor_set::FixedSizeDescriptorSetsPool, sync::GpuFuture,
};

use color_eyre::Result;

pub fn main() -> Result<()> {
    color_eyre::install()?;

    let instance = create_instance()?;

    let _debug_callback = create_debug_callback(&instance)?;

    let (surface, event_loop) = create_surface(instance)?;

    let (graphics_queue_family, present_queue_family) = pick_queues_families(&surface)?;

    let (device, graphics_queue, present_queue) =
        create_device(graphics_queue_family, present_queue_family)?;

    let (mut swapchain, swapchain_images) = create_swapchain(
        surface.clone(),
        device.clone(),
        graphics_queue.clone(),
        present_queue.clone(),
    )?;

    let (vertex_buffer, index_buffer) = create_buffers(graphics_queue.clone())?;

    let texture = load_texture(graphics_queue.clone())?;

    let sampler = create_sampler(device.clone())?;

    let render_pass = create_render_pass(device.clone(), swapchain.clone())?;

    let pipeline = create_pipeline(render_pass.clone())?;

    let mut dynamic_state = DynamicState::none();
    update_dynamic_viewport(swapchain.clone(), &mut dynamic_state);

    let mut framebuffers = create_framebuffers(swapchain_images, render_pass.clone())?;

    let uniform_buffer = CpuBufferPool::<vs::ty::UniformBufferObject>::uniform_buffer(device);

    let mut descriptor_pool =
        FixedSizeDescriptorSetsPool::new(pipeline.descriptor_set_layout(0).unwrap().clone());

    let mut swapchain_out_of_date = false;
    let mut previous_frame_future: Option<Box<dyn GpuFuture>> = None;
    let start_instant = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        main_loop(
            event,
            control_flow,
            start_instant,
            graphics_queue.clone(),
            present_queue.clone(),
            vertex_buffer.clone(),
            index_buffer.clone(),
            render_pass.clone(),
            pipeline.clone(),
            texture.clone(),
            sampler.clone(),
            &uniform_buffer,
            &mut descriptor_pool,
            &mut swapchain,
            &mut dynamic_state,
            &mut framebuffers,
            &mut swapchain_out_of_date,
            &mut previous_frame_future,
        )
        .unwrap_or_else(|e| {
            println!("\nError when running main loop: {:?}\n", e);
            std::process::exit(1);
        });
    });
}
