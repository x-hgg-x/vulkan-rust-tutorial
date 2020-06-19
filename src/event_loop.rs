use crate::init::{create_framebuffers, update_dynamic_viewport};
use crate::lib::*;

use std::{convert::TryInto, sync::Arc, time::Instant};

use vulkano::{
    buffer::CpuBufferPool,
    command_buffer::{AutoCommandBufferBuilder, DynamicState},
    descriptor::{descriptor_set::FixedSizeDescriptorSetsPool, DescriptorSet},
    device::Queue,
    format::Format,
    framebuffer::{FramebufferAbstract, RenderPassAbstract},
    image::ImmutableImage,
    pipeline::GraphicsPipelineAbstract,
    sampler::Sampler,
    swapchain::{self, AcquireError, Swapchain, SwapchainCreationError},
    sync::{self, FlushError, GpuFuture},
};
use winit::{
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    window::Window,
};

use nalgebra_glm as glm;

use color_eyre::Result;
use eyre::eyre;

#[allow(clippy::too_many_arguments)]
pub fn main_loop(
    event: Event<()>,
    control_flow: &mut ControlFlow,
    start_instant: Instant,
    graphics_queue: Arc<Queue>,
    present_queue: Arc<Queue>,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    texture: Arc<ImmutableImage<Format>>,
    sampler: Arc<Sampler>,
    uniform_buffer: &CpuBufferPool<vs::ty::UniformBufferObject>,
    descriptor_pool: &mut FixedSizeDescriptorSetsPool,
    swapchain: &mut Arc<Swapchain<Window>>,
    dynamic_state: &mut DynamicState,
    framebuffers: &mut Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    swapchain_out_of_date: &mut bool,
    previous_frame_future: &mut Option<Box<dyn GpuFuture>>,
) -> Result<()> {
    //
    match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => {
                *control_flow = ControlFlow::Exit;
            }
            WindowEvent::KeyboardInput { input, .. }
                if input.virtual_keycode == Some(VirtualKeyCode::Escape) =>
            {
                *control_flow = ControlFlow::Exit;
            }
            WindowEvent::Resized(_) => {
                *swapchain_out_of_date = true;
            }
            _ => (),
        },

        Event::RedrawEventsCleared => {
            if let Some(future) = previous_frame_future {
                future.cleanup_finished();
            }

            let (image_num, suboptimal, acquire_future) =
                match swapchain::acquire_next_image(swapchain.clone(), None) {
                    Ok(r) => r,
                    Err(AcquireError::OutOfDate) => {
                        return Ok(recreate_swapchain(
                            swapchain,
                            render_pass.clone(),
                            dynamic_state,
                            framebuffers,
                            swapchain_out_of_date,
                        )?);
                    }
                    Err(e) => return Err(eyre!("Failed to acquire next image: {:?}", e)),
                };

            if suboptimal {
                *swapchain_out_of_date = true;
            }

            // Workaround for driver bug when resizing window (but triggers validation layer errors)
            if image_num >= swapchain.num_images().try_into()? {
                return Ok(recreate_swapchain(
                    swapchain,
                    render_pass.clone(),
                    dynamic_state,
                    framebuffers,
                    swapchain_out_of_date,
                )?);
            }

            let set = update_descriptor_set(
                start_instant,
                uniform_buffer,
                descriptor_pool,
                texture,
                sampler,
            )?;

            let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(
                pipeline.device().clone(),
                graphics_queue.family(),
            )?
            .begin_render_pass(
                framebuffers[image_num].clone(),
                false,
                vec![[0.0, 0.0, 0.0, 1.0].into(), 1.0.into()],
            )?
            .draw_indexed(
                pipeline.clone(),
                &dynamic_state,
                vec![vertex_buffer],
                index_buffer,
                set,
                (),
            )?
            .end_render_pass()?
            .build()?;

            match previous_frame_future
                .take()
                .unwrap_or_else(|| Box::new(sync::now(pipeline.device().clone())))
                .join(acquire_future)
                .then_execute(graphics_queue, command_buffer)?
                .then_swapchain_present(present_queue, swapchain.clone(), image_num)
                .then_signal_fence_and_flush()
            {
                Ok(future) => {
                    *previous_frame_future = Some(Box::new(future));
                }
                Err(FlushError::OutOfDate) => {
                    *swapchain_out_of_date = true;
                    *previous_frame_future = None;
                }
                Err(e) => {
                    println!("Failed to flush future: {:?}", e);
                    *previous_frame_future = None;
                }
            }

            if *swapchain_out_of_date {
                recreate_swapchain(
                    swapchain,
                    render_pass.clone(),
                    dynamic_state,
                    framebuffers,
                    swapchain_out_of_date,
                )?;
            }
        }
        _ => (),
    }
    Ok(())
}

fn update_descriptor_set(
    start_instant: Instant,
    uniform_buffer: &CpuBufferPool<vs::ty::UniformBufferObject>,
    descriptor_pool: &mut FixedSizeDescriptorSetsPool,
    texture: Arc<ImmutableImage<Format>>,
    sampler: Arc<Sampler>,
) -> Result<Arc<dyn DescriptorSet + Send + Sync>> {
    //
    let elapsed = start_instant.elapsed().as_nanos() as f32 / 1_000_000_000.0;

    let mut ubo = vs::ty::UniformBufferObject {
        model: glm::rotate(
            &glm::identity(),
            elapsed * f32::to_radians(90.0),
            &glm::vec3(0.0, 0.0, 1.0),
        )
        .into(),

        view: glm::look_at(
            &glm::vec3(2.0, 2.0, 2.0),
            &glm::vec3(0.0, 0.0, 0.0),
            &glm::vec3(0.0, 0.0, 1.0),
        )
        .into(),

        proj: glm::perspective(
            WIDTH as f32 / HEIGHT as f32,
            f32::to_radians(45.0),
            0.1,
            10.0,
        )
        .into(),
    };
    ubo.proj[1][1] *= -1.0;

    Ok(Arc::new(
        descriptor_pool
            .next()
            .add_buffer(uniform_buffer.next(ubo)?)?
            .add_sampled_image(texture, sampler)?
            .build()?,
    ))
}

fn recreate_swapchain(
    swapchain: &mut Arc<Swapchain<Window>>,
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState,
    framebuffers: &mut Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    swapchain_out_of_date: &mut bool,
) -> Result<()> {
    //
    let (new_swapchain, new_swapchain_images) = match swapchain
        .recreate_with_dimensions(swapchain.surface().window().inner_size().into())
    {
        Ok(r) => r,
        Err(SwapchainCreationError::UnsupportedDimensions) => return Ok(()),
        Err(e) => return Err(eyre!("Failed to recreate swapchain: {:?}", e)),
    };
    *swapchain = new_swapchain;

    update_dynamic_viewport(swapchain.clone(), dynamic_state);

    *framebuffers = create_framebuffers(new_swapchain_images, render_pass)?;

    *swapchain_out_of_date = false;
    Ok(())
}
