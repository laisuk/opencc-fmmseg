fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");

        let ver = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0.0".into());
        res.set("FileDescription", "opencc-fmmseg CLI");
        res.set("ProductName", "opencc-rs");
        res.set("FileVersion", &ver);
        res.set("ProductVersion", &ver);

        res.compile().expect("Failed to embed Windows resources");
    }
}
