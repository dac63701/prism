fn main() {
    #[cfg(target_os = "macos")]
    {
        // SPM-built dependencies (apple_metal, screencapturekit, apple_cf) link
        // against Swift runtime libraries. With CLT 16.x, the Swift compat libs
        // live under CLT's usr/lib/swift/macosx, not the toolchain path that SPM
        // assumes.  Add both the runtime rpath and the static link search path
        // so the linker can find libswiftCompatibility*.a.
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
        println!(
            "cargo:rustc-link-search=/Library/Developer/CommandLineTools/usr/lib/swift/macosx"
        );
    }

    tauri_build::build();
}
