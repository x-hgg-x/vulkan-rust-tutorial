mod application;
mod event_loop;
mod init;
mod lib;
mod utils;

fn main() {
    application::run().unwrap_or_else(|e| {
        println!("\nError when running application: {}\n", e);
        std::process::exit(1);
    });
}
