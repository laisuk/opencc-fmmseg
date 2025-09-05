use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use regex::Regex;
use tempfile::tempdir;
use walkdir::WalkDir;
use zip::{
    write::{ExtendedFileOptions, FileOptions},
    CompressionMethod, ZipArchive, ZipWriter,
};

use opencc_fmmseg::OpenCC;

pub struct ConversionResult {
    pub success: bool,
    pub message: String,
}

pub struct OfficeConverter;

impl OfficeConverter {
    pub fn convert(
        input_path: &str,
        output_path: &str,
        format: &str,
        helper: &OpenCC,
        config: &str,
        punctuation: bool,
        keep_font: bool,
    ) -> io::Result<ConversionResult> {
        let temp_dir = tempdir()?;
        let temp_path = temp_dir.path();

        // 1) Unzip in its own scope so all input handles are closed before output work
        {
            let file = File::open(input_path)?;
            let mut archive = ZipArchive::new(file)?;

            for i in 0..archive.len() {
                let mut entry = archive.by_index(i)?;
                let raw_name = entry.name().replace('\\', "/");
                let rel_path = Path::new(&raw_name);

                // Reject zip-slip & roots
                if rel_path.components().any(|c| {
                    matches!(
                        c,
                        std::path::Component::ParentDir | std::path::Component::RootDir
                    )
                }) {
                    continue;
                }

                let out_path = temp_path.join(rel_path);

                // Directory entries
                if entry.is_dir() || raw_name.ends_with('/') {
                    fs::create_dir_all(&out_path)?;
                    continue;
                }

                // File entries
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut out_file = File::create(&out_path)?;
                io::copy(&mut entry, &mut out_file)?;
            }
        }

        // 2) Convert targeted XML/text files in-place under temp_path
        for xml_file in get_target_xml_paths(format, temp_path) {
            if !xml_file.exists() || !xml_file.is_file() {
                continue;
            }
            let mut content = String::new();
            File::open(&xml_file)?.read_to_string(&mut content)?;

            let mut font_map = HashMap::new();
            if keep_font {
                mask_font(&mut content, format, &mut font_map);
            }

            let mut converted = helper.convert(&content, config, punctuation);
            if keep_font {
                for (marker, original) in font_map {
                    converted = converted.replace(&marker, &original);
                }
            }

            File::create(&xml_file)?.write_all(converted.as_bytes())?;
        }

        // 3) Output: write to temp then rename to final path
        let out_path = Path::new(output_path);
        let in_path_abs = Path::new(input_path)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(input_path));
        let out_path_abs = out_path
            .canonicalize()
            .unwrap_or_else(|_| out_path.to_path_buf());

        if out_path_abs == in_path_abs {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "output_path must differ from input_path",
            ));
        }

        replace_with_temp(out_path, |zip_writer| {
            // EPUB: ensure 'mimetype' is first and Stored; for others, order doesn't matter
            if format.eq_ignore_ascii_case("epub") {
                let mimetype_path = temp_path.join("mimetype");
                if mimetype_path.exists() && mimetype_path.is_file() {
                    let mut buf = Vec::new();
                    File::open(&mimetype_path)?.read_to_end(&mut buf)?;
                    let opts: FileOptions<'_, ExtendedFileOptions> =
                        FileOptions::default().compression_method(CompressionMethod::Stored);
                    zip_writer.start_file("mimetype", opts)?;
                    zip_writer.write_all(&buf)?;
                }
            }

            for entry in WalkDir::new(temp_path).into_iter().filter_map(Result::ok) {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                // EPUB: we've already written mimetype first; skip it here
                let rel = path
                    .strip_prefix(temp_path)
                    .map_err(|e| {
                        io::Error::new(io::ErrorKind::Other, format!("strip_prefix failed: {}", e))
                    })?
                    .to_string_lossy()
                    .replace('\\', "/");
                if format.eq_ignore_ascii_case("epub") && rel == "mimetype" {
                    continue;
                }

                let mut buffer = Vec::new();
                File::open(path)?.read_to_end(&mut buffer)?;

                let is_mimetype = rel == "mimetype";
                let method = if is_mimetype {
                    CompressionMethod::Stored
                } else {
                    CompressionMethod::Deflated
                };
                let options: FileOptions<'_, ExtendedFileOptions> =
                    FileOptions::default().compression_method(method);

                zip_writer.start_file(&rel, options)?;
                zip_writer.write_all(&buffer)?;
            }
            Ok(())
        })?;

        Ok(ConversionResult {
            success: true,
            message: "✅ Conversion completed.".to_string(),
        })
    }
}

/* ---------- Helpers ---------- */

fn remove_existing_file(path: &Path) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("output_path is a directory: {:?}", path),
        ));
    }
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        #[cfg(windows)]
        if perms.readonly() {
            perms.set_readonly(false);
            fs::set_permissions(path, perms)?;
        }
    }
    fs::remove_file(path)
}

/// Write to a temp file then replace the final path.
/// On Windows the destination must not exist; we remove it first.
fn replace_with_temp(
    final_out: &Path,
    write_zip: impl FnOnce(&mut ZipWriter<File>) -> io::Result<()>,
) -> io::Result<()> {
    let ext = final_out
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("zip");
    let tmp_out = final_out.with_extension(format!("tmp.{}", ext));

    // Clean any stale temp
    let _ = remove_existing_file(&tmp_out);

    // 1) Create & write temp zip
    {
        let zip_file = File::create(&tmp_out)?;
        let mut zw = ZipWriter::new(zip_file);
        write_zip(&mut zw)?;
        zw.finish()?;
    }

    // 2) Remove existing final (handle read-only on Windows)
    remove_existing_file(final_out)?;

    // 3) Move temp -> final (same volume)
    fs::rename(&tmp_out, final_out)
}

/// Select only the files we intend to modify per format.
fn get_target_xml_paths(format: &str, base_dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    match format {
        "docx" => result.push(base_dir.join("word/document.xml")),
        "xlsx" => result.push(base_dir.join("xl/sharedStrings.xml")),
        "pptx" => {
            // Slides & notes only, skip .rels
            for dir in ["ppt/slides", "ppt/notesSlides"] {
                let root = base_dir.join(dir);
                if !root.exists() {
                    continue;
                }
                for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
                    let p = entry.path();
                    if !p.is_file() {
                        continue;
                    }
                    if p.extension().and_then(|e| e.to_str()) != Some("xml") {
                        continue;
                    }
                    if p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.ends_with(".rels"))
                        .unwrap_or(false)
                    {
                        continue;
                    }
                    result.push(p.to_path_buf());
                }
            }
        }
        "odt" | "ods" | "odp" => result.push(base_dir.join("content.xml")),
        "epub" => {
            for entry in WalkDir::new(base_dir).into_iter().filter_map(Result::ok) {
                let p = entry.path();
                if !p.is_file() {
                    continue;
                }
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "xhtml" | "opf" | "ncx" | "html") {
                    result.push(p.to_path_buf());
                }
            }
        }
        _ => {}
    }
    result
}

fn mask_font(xml: &mut String, format: &str, font_map: &mut HashMap<String, String>) {
    let pattern = match format {
        "docx" => r#"(w:(?:eastAsia|ascii|hAnsi|cs)=")(.*?)(")"#,
        "xlsx" => r#"(val=")(.*?)(")"#,
        "pptx" => r#"(typeface=")(.*?)(")"#,
        "odt" | "ods" | "odp" => {
            r#"((?:style:font-name(?:-asian|-complex)?|svg:font-family|style:name)=['"])([^'"]+)(['"])"#
        }
        "epub" => r#"(font-family\s*:\s*)([^;"']+)"#,
        _ => return,
    };
    let re = Regex::new(pattern).unwrap();
    let mut counter = 0;
    let mut result_str = String::new();
    let mut last_end = 0;
    for caps in re.captures_iter(xml) {
        let marker = format!("__F_O_N_T_{}__", counter);
        counter += 1;
        font_map.insert(marker.clone(), caps[2].to_string());
        let mat = caps.get(0).unwrap();
        result_str.push_str(&xml[last_end..mat.start()]);
        result_str.push_str(&caps[1]);
        result_str.push_str(&marker);
        if caps.len() > 3 {
            result_str.push_str(&caps[3]);
        }
        last_end = mat.end();
    }
    result_str.push_str(&xml[last_end..]);
    *xml = result_str;
}
