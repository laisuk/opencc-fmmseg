fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=assets/icon.ico");

        use std::env;
        use winres::{VersionInfo, WindowsResource};

        // --- Version from Cargo (SemVer: major.minor.patch) ---
        let major: u16 = env::var("CARGO_PKG_VERSION_MAJOR").unwrap().parse().unwrap();
        let minor: u16 = env::var("CARGO_PKG_VERSION_MINOR").unwrap().parse().unwrap();
        let patch: u16 = env::var("CARGO_PKG_VERSION_PATCH").unwrap().parse().unwrap();
        let revision: u16 = 0; // implicit 4th digit

        // Pack into Windows FIXEDFILEINFO format
        let packed: u64 =
            ((major as u64) << 48)
                | ((minor as u64) << 32)
                | ((patch as u64) << 16)
                | (revision as u64);

        // String forms
        let ver_dots = format!("{major}.{minor}.{patch}.{revision}");
        let ver_commas = format!("{major},{minor},{patch},{revision}");

        // Other Cargo metadata
        let name = env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "opencc-rs".into());
        let authors = env::var("CARGO_PKG_AUTHORS").unwrap_or_else(|_| "Laisuk".into());
        let desc = env::var("CARGO_PKG_DESCRIPTION").unwrap_or_else(|_| {
            "Opencc-Fmmseg CLI (Simplified/Traditional Chinese Converter)".into()
        });

        let mut res = WindowsResource::new();
        res.set_icon("assets/icon.ico");

        // --- Authoritative numeric versions ---
        res.set_version_info(VersionInfo::FILEVERSION, packed);
        res.set_version_info(VersionInfo::PRODUCTVERSION, packed);

        // --- Explorer-readable strings ---
        res.set("FileVersion", &ver_dots);
        res.set("ProductVersion", &ver_dots);
        res.set("FileVersionRaw", &ver_commas);
        res.set("ProductVersionRaw", &ver_commas);

        // --- Rich metadata ---
        res.set("FileDescription", &desc);
        res.set("ProductName", &name);
        res.set("CompanyName", &authors);
        res.set("LegalCopyright", "© Laisuk. MIT License");
        res.set("OriginalFilename", "opencc-rs.exe");
        res.set("InternalName", &name);
        res.set("Comments", "Built with Rust and Opencc-Fmmseg libraries.");

        res.compile().expect("Failed to embed Windows resources");
    }
}
