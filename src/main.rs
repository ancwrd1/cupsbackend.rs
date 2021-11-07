pub mod cupsbackend;

fn main() {
    cupsbackend::CupsBackend::new().run();
}
