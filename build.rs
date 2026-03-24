fn main() {
    slint_build::compile("src/main.slint").unwrap();

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/app-logo.ico");
        res.compile().unwrap();
    }
}
