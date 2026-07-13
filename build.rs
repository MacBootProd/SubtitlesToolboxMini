// build.rs
fn main() {
    compile_windows_resource();
    println!("cargo:rerun-if-changed=src/main.slint");
    println!("cargo:rerun-if-changed=translations");

    // Compile Slint UI with bundled GNU Gettext translations
    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations("translations");
        
    slint_build::compile_with_config("src/main.slint", config).unwrap();
}

#[cfg(target_os = "windows")]
fn compile_windows_resource() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/app-logo.ico");
    res.compile().unwrap();
}

#[cfg(not(target_os = "windows"))]
fn compile_windows_resource() {}
