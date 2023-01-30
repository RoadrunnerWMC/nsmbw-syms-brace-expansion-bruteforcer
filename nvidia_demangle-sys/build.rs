fn main() {
    cc::Build::new()
        .cpp(true)
        .file("src/nvidia_demangle.cpp")
        .compile("nvidia_demangle");
}