fn main() {
    tauri_build::build();

    // macOS Swift runtime libraries use @rpath for libswift_Concurrency.dylib.
    // Add /usr/lib/swift as an rpath so the dynamic linker can find it.
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
}
