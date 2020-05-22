use vulkano::instance::{ApplicationInfo, Instance, InstanceExtensions, Version};

fn main() {
    let version = Version {
        major: 1,
        minor: 0,
        patch: 0,
    };
    let instance = Instance::new(
        Some(&ApplicationInfo {
            application_name: Some("Vulkan Application".into()),
            application_version: Some(version),
            engine_name: Some("No Engine".into()),
            engine_version: Some(version),
        }),
        &InstanceExtensions::none(),
        None,
    )
    .expect("failed to create instance");

    println!("{:?}", instance);
}
