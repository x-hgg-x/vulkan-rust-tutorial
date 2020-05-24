use crate::utils::Value;

use std::sync::Arc;

use vulkano::instance::{
    debug::{DebugCallback, DebugCallbackCreationError, MessageSeverity, MessageType},
    ApplicationInfo, Instance, InstanceCreationError, Version,
};

pub fn create_instance() -> Result<Value<Arc<Instance>>, InstanceCreationError> {
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
    instance: Arc<Instance>,
) -> Result<Value<Option<DebugCallback>>, DebugCallbackCreationError> {
    if cfg!(debug_assertions) {
        Ok(Value(Some(DebugCallback::new(
            &instance,
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
