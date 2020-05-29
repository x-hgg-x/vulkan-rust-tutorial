use crate::init::{create_framebuffers, update_dynamic_viewport};
use crate::lib::*;
use crate::utils::BacktraceExt;

use std::{error::Error, sync::Arc};

use vulkano::{
    buffer::CpuAccessibleBuffer,
    command_buffer::{AutoCommandBufferBuilder, DynamicState},
    device::Queue,
    framebuffer::{FramebufferAbstract, RenderPassAbstract},
    pipeline::GraphicsPipelineAbstract,
    swapchain::{self, AcquireError, Swapchain, SwapchainCreationError},
    sync::{self, FlushError, GpuFuture},
};
use winit::{
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    window::Window,
};

#[allow(clippy::too_many_arguments)]
pub fn main_loop(
    event: Event<()>,
    control_flow: &mut ControlFlow,
    graphics_queue: Arc<Queue>,
    present_queue: Arc<Queue>,
    vertex_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    swapchain: &mut Arc<Swapchain<Window>>,
    dynamic_state: &mut DynamicState,
    framebuffers: &mut Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    swapchain_out_of_date: &mut bool,
    previous_frame_future: &mut Option<Box<dyn GpuFuture>>,
) -> Result<(), Box<dyn Error>> {
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
                    Err(e) => return Err(format!("Failed to acquire next image: {:?}", e).into()),
                };

            if suboptimal {
                *swapchain_out_of_date = true;
            }

            // Workaround for driver bug when resizing window (but triggers validation layer errors)
            if image_num >= swapchain.num_images() as usize {
                return Ok(recreate_swapchain(
                    swapchain,
                    render_pass.clone(),
                    dynamic_state,
                    framebuffers,
                    swapchain_out_of_date,
                )?);
            }

            let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(
                graphics_queue.device().clone(),
                graphics_queue.family(),
            )?
            .begin_render_pass(
                framebuffers[image_num].clone(),
                false,
                vec![[0.0, 0.0, 0.0, 1.0].into()],
            )?
            .draw(
                pipeline.clone(),
                &dynamic_state,
                vec![vertex_buffer],
                (),
                (),
            )?
            .end_render_pass()?
            .build()?;

            match previous_frame_future
                .take()
                .unwrap_or_else(|| Box::new(sync::now(graphics_queue.device().clone())))
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

fn recreate_swapchain(
    swapchain: &mut Arc<Swapchain<Window>>,
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState,
    framebuffers: &mut Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    swapchain_out_of_date: &mut bool,
) -> Result<(), Box<dyn Error>> {
    //
    let (new_swapchain, new_swapchain_images) = match swapchain
        .recreate_with_dimensions(swapchain.surface().window().inner_size().into())
    {
        Ok(r) => r,
        Err(SwapchainCreationError::UnsupportedDimensions) => return Ok(()),
        Err(e) => return Err(format!("Failed to recreate swapchain: {:?}", e).into()),
    };
    *swapchain = new_swapchain;

    update_dynamic_viewport(swapchain.clone(), dynamic_state);

    *framebuffers = create_framebuffers(new_swapchain_images, render_pass).debug()?;

    *swapchain_out_of_date = false;
    Ok(())
}
