use crate::init::*;
use crate::utils::BacktraceExt;

use std::error::Error;

pub fn run() -> Result<(), Box<dyn Error>> {
    let instance = create_instance().debug()?;

    let _debug_callback = create_debug_callback(&instance).debug()?;

    let (surface, event_loop) = create_surface(instance.clone()).debug()?;

    let (physical_device, graphics_queue_family, present_queue_family) =
        pick_physical_device(&instance, surface).debug()?;

    let (device, graphics_queue, present_queue) =
        create_device(physical_device, graphics_queue_family, present_queue_family).debug()?;

    Ok(())
}
