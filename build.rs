extern crate cc;

fn main() {
  cc::Build::new()
    .cpp(true)
    .shared_flag(true)
    .file("worker.cpp")
    .compile("libworker.so");
}
