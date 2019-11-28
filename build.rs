
extern crate cc;

fn main() {
    cc::Build::new()
        .file("worker.c")
        .compile("libworker");
}
