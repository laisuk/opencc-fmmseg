fn main() {
    #[cfg(target_os = "windows")]
    {
        use std::env;
        use winres::WindowsResource;

        let mut res = WindowsResource::new();
        // res.set_icon("assets/icon.ico");

        // Cargo metadata
        let ver = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0.0".into());
        let name = env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "opencc_fmmseg_capi".into());
        let authors = env::var("CARGO_PKG_AUTHORS").unwrap_or_else(|_| "Laisuk".into());
        let desc = env::var("CARGO_PKG_DESCRIPTION").unwrap_or_else(|_| {
            "Opencc-Fmmseg C API (Simplified/Traditional Chinese Converter)".into()
        });

        // Version fields (Windows expects comma-separated numerics)
        let ver_commas = ver.replace('.', ",");

        // Set rich metadata fields
        res.set("FileDescription", &desc);
        res.set("ProductName", "Opencc-Fmmseg C API");
        res.set("CompanyName", &authors);
        res.set("LegalCopyright", "© Laisuk. MIT License");
        res.set("OriginalFilename", "opencc_fmmseg_capi.dll");
        res.set("InternalName", &name);
        res.set("Comments", "Built with Rust and Opencc-Fmmseg libraries.");
        res.set("FileVersion", &ver_commas);
        res.set("ProductVersion", &ver_commas);

        // Compile the .res and link it
        res.compile().expect("Failed to embed Windows resources");
    }
}
