// Standalone host for exercising the same Windows worker used by the desktop app.
fn main() {
    if !bimb_core::platform::media_worker_main() {
        eprintln!("Use --bimb-media-worker on Windows with a media request on stdin");
        std::process::exit(1);
    }
}
