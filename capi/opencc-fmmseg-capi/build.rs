#[cfg(target_os = "windows")]
fn pack_win_ver(major: u64, minor: u64, patch: u64, revision: u64) -> u64 {
    (major << 48) | (minor << 32) | (patch << 16) | revision
}

fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=Cargo.toml");

        use cargo_metadata::MetadataCommand;
        use std::env;
        use winres::{VersionInfo, WindowsResource};

        let this_pkg_name =
            env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "opencc-fmmseg-capi".into());

        let metadata = MetadataCommand::new()
            .no_deps()
            .exec()
            .expect("Failed to read cargo metadata");

        let pkg = metadata
            .packages
            .iter()
            .find(|p| p.name == this_pkg_name)
            .unwrap_or_else(|| panic!("Package not found in cargo metadata: {}", this_pkg_name));

        let revision: u64 = pkg
            .metadata
            .get("capi")
            .and_then(|v| v.get("revision"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let major: u64 = pkg.version.major;
        let minor: u64 = pkg.version.minor;
        let patch: u64 = pkg.version.patch;

        let packed_u64 = pack_win_ver(major, minor, patch, revision);
        let ver_str_commas = format!("{},{},{},{}", major, minor, patch, revision);

        let authors = env::var("CARGO_PKG_AUTHORS").unwrap_or_else(|_| "Laisuk".into());
        let desc = env::var("CARGO_PKG_DESCRIPTION").unwrap_or_else(|_| {
            "Opencc-Fmmseg C API (Simplified/Traditional Chinese Converter)".into()
        });

        let mut res = WindowsResource::new();

        // ✅ Set the FIXEDFILEINFO raw versions (this fixes FileVersionRaw/ProductVersionRaw)
        res.set_version_info(VersionInfo::FILEVERSION, packed_u64);
        res.set_version_info(VersionInfo::PRODUCTVERSION, packed_u64);

        // ✅ Also set the string table versions (Explorer “Details” page)
        res.set("FileVersion", &ver_str_commas);
        res.set("ProductVersion", &ver_str_commas);

        // Other metadata
        res.set("FileDescription", &desc);
        res.set("ProductName", "Opencc-Fmmseg C API");
        res.set("CompanyName", &authors);
        res.set("OriginalFilename", "opencc_fmmseg_capi.dll");
        res.set("InternalName", &this_pkg_name);
        res.set("LegalCopyright", "© Laisuk. MIT License");
        res.set("Comments", "Built with Rust and Opencc-Fmmseg libraries.");

        res.compile().expect("Failed to embed Windows resources");
    }
}
