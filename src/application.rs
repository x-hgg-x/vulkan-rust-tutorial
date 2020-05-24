use crate::init::{create_debug_callback, create_instance};
use crate::utils::BacktraceExt;

use std::error::Error;

pub fn run() -> Result<(), Box<dyn Error>> {
    let instance = create_instance().debug()?;

    let _debug_callback = create_debug_callback(instance).debug()?;

    Ok(())
}
