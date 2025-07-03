use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use regex::Regex;
use tempfile::tempdir;
use zip::write::{ExtendedFileOptions, FileOptions};
// 💡 important!
use zip::{ZipArchive, ZipWriter};

use opencc_fmmseg;
use opencc_fmmseg::OpenCC;

pub struct OfficeDocConverter;

pub struct ConversionResult {
    pub success: bool,
    pub message: String,
}

impl OfficeDocConverter {
    pub fn convert(
        input_path: &str,
        output_path: &str,
        format: &str,
        helper: &mut OpenCC,
        config: &str,
        punctuation: bool,
        keep_font: bool,
    ) -> ConversionResult {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let temp_path = temp_dir.path().to_path_buf();

        let file = match File::open(input_path) {
            Ok(f) => f,
            Err(_) => {
                return ConversionResult {
                    success: false,
                    message: "❌ Failed to open ZIP archive.".to_string(),
                }
            }
        };

        let mut archive = ZipArchive::new(file).unwrap();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();

            let raw_name = file.name().replace('\\', "/");
            let relative_path = Path::new(&raw_name);

            // Sanitize: skip if file has '..' or is absolute
            if relative_path.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::ParentDir | std::path::Component::RootDir
                )
            }) {
                continue; // Skip unsafe paths
            }

            let out_path = temp_path.join(relative_path);
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).ok();
            }

            let mut out_file = File::create(&out_path).unwrap();
            std::io::copy(&mut file, &mut out_file).ok();
        }

        let target_xmls = get_target_xml_paths(format, &temp_path);
        for xml_file in target_xmls {
            if !xml_file.exists() {
                continue;
            }
            let mut content = String::new();
            File::open(&xml_file)
                .unwrap()
                .read_to_string(&mut content)
                .unwrap();

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

            let mut out_file = File::create(&xml_file).unwrap();
            out_file.write_all(converted.as_bytes()).unwrap();
        }

        if Path::new(output_path).exists() {
            fs::remove_file(output_path).unwrap();
        }

        let zip_file = match File::create(output_path) {
            Ok(f) => f,
            Err(_) => {
                return ConversionResult {
                    success: false,
                    message: "❌ Failed to create output ZIP.".to_string(),
                }
            }
        };

        let mut zip_writer = ZipWriter::new(zip_file);

        // Replace this section in your code:
        for entry in walkdir::WalkDir::new(&temp_path) {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                let mut buffer = Vec::new();
                if let Err(e) = File::open(path).and_then(|mut f| f.read_to_end(&mut buffer)) {
                    return ConversionResult {
                        success: false,
                        message: format!("❌ Failed to read file {:?}: {}", path, e),
                    };
                }

                let relative_path = match path.strip_prefix(&temp_path) {
                    Ok(p) => p.to_string_lossy(),
                    Err(e) => {
                        return ConversionResult {
                            success: false,
                            message: format!("❌ Failed to compute relative path: {}", e),
                        };
                    }
                };

                // FIX: Normalize path separators to forward slashes for ZIP
                let relative_path = relative_path.replace('\\', "/");

                let options: FileOptions<'_, ExtendedFileOptions> =
                    FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

                if let Err(e) = zip_writer
                    .start_file(&relative_path, options)
                    .and_then(|_| {
                        zip_writer
                            .write_all(&buffer)
                            .map_err(zip::result::ZipError::Io)
                    })
                {
                    return ConversionResult {
                        success: false,
                        message: format!("❌ Failed to write {} to ZIP: {}", relative_path, e),
                    };
                }
            }
        }

        if let Err(e) = zip_writer.finish() {
            return ConversionResult {
                success: false,
                message: format!("❌ Failed to finalize ZIP file: {}", e),
            };
        }

        ConversionResult {
            success: true,
            message: "✅ Conversion completed.".to_string(),
        }
    }
}

fn get_target_xml_paths(format: &str, base_dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    match format {
        "docx" => result.push(base_dir.join("word/document.xml")),
        "xlsx" => result.push(base_dir.join("xl/sharedStrings.xml")),
        "pptx" => {
            for entry in walkdir::WalkDir::new(base_dir.join("ppt")) {
                let path = entry.unwrap().path().to_path_buf();
                let name = path.file_name().unwrap().to_string_lossy();
                let path_str = path.to_string_lossy();
                if name.contains("slide") || path_str.contains("notesSlide") {
                    result.push(path);
                }
            }
        }
        "odt" | "ods" | "odp" => result.push(base_dir.join("content.xml")),
        "epub" => {
            for entry in walkdir::WalkDir::new(base_dir) {
                let path = entry.unwrap().path().to_path_buf();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "xhtml" | "opf" | "ncx") {
                    result.push(path);
                }
            }
        }
        _ => {}
    }
    result
}

fn mask_font(xml: &mut String, format: &str, font_map: &mut HashMap<String, String>) {
    let pattern = match format {
        "docx" => r#"(w:(?:eastAsia|ascii|hAnsi|cs)=")([^"]+)(")"#,
        "xlsx" => r#"(val=")([^"]+)(")"#,
        "pptx" => r#"(typeface=")([^"]+)(")"#,
        "odt" | "ods" | "odp" => {
            r#"((?:style:font-name(?:-asian|-complex)?|svg:font-family|style:name)=['"])([^'"]+)(['"])"#
        }
        "epub" => r#"(font-family\s*:\s*)([^;"']+)"#,
        _ => return,
    };
    let re = Regex::new(pattern).unwrap();
    let mut counter = 0;
    let mut result = String::new();
    let mut last_end = 0;
    for caps in re.captures_iter(xml) {
        let marker = format!("__F_O_N_T_{}__", counter);
        counter += 1;
        font_map.insert(marker.clone(), caps[2].to_string());
        let mat = caps.get(0).unwrap();
        result.push_str(&xml[last_end..mat.start()]);
        result.push_str(&caps[1]);
        result.push_str(&marker);
        if caps.len() > 3 {
            result.push_str(&caps[3]);
        }
        last_end = mat.end();
    }
    result.push_str(&xml[last_end..]);
    *xml = result;
}
