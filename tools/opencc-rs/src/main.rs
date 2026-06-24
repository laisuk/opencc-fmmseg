mod office_converter;

use office_converter::OfficeConverter;

use clap::{
    builder::{StringValueParser, TypedValueParser, ValueParser},
    Arg, ArgMatches, Command,
};
use encoding_rs::Encoding;
use encoding_rs_io::DecodeReaderBytesBuilder;
use opencc_fmmseg::{
    CustomDictFileSpec, CustomDictMode, DetofuLevel, DictSlot, DictionaryMaxlength, OpenCC,
    OpenccConfig,
};
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::sync::OnceLock;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = build_cli().get_matches();

    match matches.subcommand() {
        Some(("convert", sub)) => handle_convert(sub),
        Some(("office", sub)) => handle_office(sub),
        _ => unreachable!(),
    }
}

fn build_cli() -> Command {
    Command::new("opencc-rs")
        .about("OpenCC Rust: Command Line Open Chinese Converter")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("convert")
                .about("Convert plain text using OpenCC")
                .args(common_args())
                .arg(
                    Arg::new("keep-ids")
                        .long("keep-ids")
                        .help("Preserve Unicode IDS expressions during conversion")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("custom-dict")
                        .long("custom-dict")
                        .value_name("SLOT:MODE:FILE")
                        .action(clap::ArgAction::Append)
                        .help("Custom dictionary file, e.g. hkphrasesrev:append:my_hk_dict.txt"),
                )
                .arg(
                    Arg::new("in_enc")
                        .long("in-enc")
                        .default_value("UTF-8")
                        .help("Encoding for input"),
                )
                .arg(
                    Arg::new("out_enc")
                        .long("out-enc")
                        .default_value("UTF-8")
                        .help("Encoding for output"),
                ),
        )
        .subcommand(
            Command::new("office")
                .about("Convert Office or EPUB documents using OpenCC")
                .args(common_args())
                .arg(
                    Arg::new("format")
                        .short('f')
                        .long("format")
                        .value_name("ext")
                        .help("Force document format: docx, odt, epub..."),
                )
                .arg(
                    Arg::new("keep_font")
                        .short('k')
                        .long("keep-font")
                        .action(clap::ArgAction::SetTrue)
                        .help("Preserve original font styles"),
                )
                .arg(
                    Arg::new("convert_filename")
                        .long("convert-filename")
                        .action(clap::ArgAction::SetTrue)
                        .help(
                            "Convert the output filename using the selected OpenCC configuration",
                        ),
                ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[ignore]
    #[test]
    fn convert_file_preserves_original_line_endings() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join(format!("opencc-rs-newline-{nonce}.in.txt"));
        let output_path = temp_dir.join(format!("opencc-rs-newline-{nonce}.out.txt"));

        fs::write(&input_path, "汉字\r\n转换\n测试\r完成").unwrap();

        let matches = build_cli()
            .try_get_matches_from([
                "opencc-rs",
                "convert",
                "-i",
                input_path.to_str().unwrap(),
                "-o",
                output_path.to_str().unwrap(),
                "-c",
                "s2t",
            ])
            .unwrap();
        let (_, convert_matches) = matches.subcommand().unwrap();

        handle_convert(convert_matches).unwrap();

        let output = fs::read_to_string(&output_path).unwrap();
        assert_eq!(output, "漢字\r\n轉換\n測試\r完成");

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }
}

fn get_supported_configs() -> &'static str {
    static SUPPORTED: OnceLock<String> = OnceLock::new();
    SUPPORTED.get_or_init(|| {
        let mut s = String::with_capacity(128);
        for (i, cfg) in OpenccConfig::ALL.iter().enumerate() {
            if i > 0 {
                s.push_str(" | ");
            }
            s.push_str(cfg.as_str());
        }
        s
    })
}

fn config_value_parser() -> ValueParser {
    ValueParser::new(StringValueParser::new().try_map(|s| {
        OpenccConfig::try_from(s.as_str())
            .map(OpenccConfig::as_str)
            .map(str::to_owned)
            .map_err(|_| format!("\nSupported configs: {}", get_supported_configs()))
    }))
}

fn common_args() -> Vec<Arg> {
    vec![
        Arg::new("input")
            .short('i')
            .long("input")
            .value_name("file")
            .help("Input file (use stdin if omitted for non-office documents)"),
        Arg::new("output")
            .short('o')
            .long("output")
            .value_name("file")
            .help("Output file (use stdout if omitted for non-office documents)"),
        Arg::new("config")
            .short('c')
            .long("config")
            .required(true)
            .value_parser(config_value_parser())
            .help(format!(
                "Conversion configuration ({})",
                get_supported_configs()
            )),
        Arg::new("punct")
            .short('p')
            .long("punct")
            .action(clap::ArgAction::SetTrue)
            .help("Enable punctuation conversion"),
        Arg::new("detofu")
            .long("detofu")
            .value_name("LEVEL")
            .num_args(0..=1)
            .default_missing_value("all")
            .help("Apply tofu-safe fallback after conversion: all, ext-c, ext-d, ext-e, ext-f, ext-g, ext-h, ext-i"),
        Arg::new("detofu-file")
            .long("detofu-file")
            .value_name("FILE")
            .help(
                "Load additional detofu fallback mappings from a UTF-8 text file. \
         Custom mappings override built-in mappings (requires --detofu)",
            ),
    ]
}

fn handle_convert(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let input_file = matches.get_one::<String>("input");
    let output_file = matches.get_one::<String>("output");
    let config = matches.get_one::<String>("config").unwrap();
    let in_enc = matches.get_one::<String>("in_enc").unwrap();
    let out_enc = matches.get_one::<String>("out_enc").unwrap();
    let punctuation = matches.get_flag("punct");

    if matches.contains_id("detofu-file") && matches.get_one::<String>("detofu").is_none() {
        return Err("--detofu-file requires --detofu".into());
    }

    let is_console = input_file.is_none();
    let mut input: Box<dyn Read> = match input_file {
        Some(file_name) => Box::new(BufReader::new(File::open(file_name)?)),
        None => {
            if io::stdin().is_terminal() {
                println!("Input text to convert, <ctrl-z/d> to submit:");
            }
            Box::new(BufReader::new(io::stdin().lock()))
        }
    };

    let mut buffer = read_input(&mut *input, is_console)?;
    if in_enc.eq_ignore_ascii_case("UTF-8") && !out_enc.eq_ignore_ascii_case("UTF-8") {
        remove_utf8_bom(&mut buffer);
    }

    let input_str = decode_input(&buffer, in_enc)?;
    // let mut cc = OpenCC::new();
    let mut cc = if let Some(values) = matches.get_many::<String>("custom-dict") {
        let specs = values
            .map(|v| parse_custom_dict_spec(v))
            .collect::<Result<Vec<_>, _>>()?;

        let dictionary = DictionaryMaxlength::from_zstd()?.with_custom_dict_files(&specs)?;

        OpenCC::from_dictionary(dictionary)
    } else {
        OpenCC::new()
    };

    if matches.get_flag("keep-ids") {
        cc.set_preserve_ids(true);
    }

    let output_str = cc.convert(&input_str, config, punctuation);

    let output_str = if let Some(level) = matches.get_one::<String>("detofu") {
        let level = DetofuLevel::parse(level)?;

        if let Some(path) = matches.get_one::<String>("detofu-file") {
            cc.detofu_with_custom_file(&output_str, level, path)?
        } else {
            cc.detofu(&output_str, level)
        }
    } else {
        output_str
    };

    let is_console_output = output_file.is_none();
    let mut output: Box<dyn Write> = match output_file {
        Some(file_name) => Box::new(BufWriter::new(File::create(file_name)?)),
        None => Box::new(BufWriter::new(io::stdout().lock())),
    };

    let final_output = if is_console_output && !output_str.ends_with('\n') {
        format!("{output_str}\n")
    } else {
        output_str
    };

    encode_and_write_output(&final_output, out_enc, &mut output)?;
    output.flush()?;

    Ok(())
}

fn handle_office(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let office_extensions: HashSet<&'static str> =
        ["docx", "xlsx", "pptx", "odt", "ods", "odp", "epub"].into();

    let input_file = matches
        .get_one::<String>("input")
        .ok_or("❌  Input file is required for office mode")?;

    let output_file = matches.get_one::<String>("output");
    let config = matches.get_one::<String>("config").unwrap();
    let punctuation = matches.get_flag("punct");
    let keep_font = matches.get_flag("keep_font");
    let convert_filename = matches.get_flag("convert_filename");
    let format = matches.get_one::<String>("format").map(String::as_str);

    let office_format = if let Some(f) = format {
        f.to_lowercase()
    } else {
        let ext = std::path::Path::new(input_file)
            .extension()
            .and_then(|e| e.to_str())
            .ok_or("❌  Cannot infer file extension. Please provide --format.")?
            .to_lowercase();

        if office_extensions.contains(ext.as_str()) {
            ext
        } else {
            return Err(format!(
                "❌  Unsupported Office extension: .{ext}. Please provide --format."
            )
            .into());
        }
    };

    if !office_extensions.contains(office_format.as_str()) {
        return Err(format!("❌  Unsupported Office format: {office_format}").into());
    }

    let helper = OpenCC::new();

    let final_output = match output_file {
        Some(path) => {
            let output_path = std::path::Path::new(path);

            if output_path.extension().is_none() {
                format!("{path}.{}", office_format)
            } else {
                path.clone()
            }
        }
        None => {
            let input_path = std::path::Path::new(input_file);
            let file_stem = input_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("converted");

            let parent = input_path.parent().unwrap_or_else(|| ".".as_ref());
            let final_stem = if convert_filename {
                let file_stem_converted = helper.convert(file_stem, config, punctuation);
                format!("{file_stem_converted}_converted")
            } else {
                format!("{file_stem}_converted")
            };

            parent
                .join(format!("{final_stem}.{office_format}"))
                .to_string_lossy()
                .to_string()
        }
    };

    match OfficeConverter::convert(
        input_file,
        &final_output,
        &office_format,
        &helper,
        config,
        punctuation,
        keep_font,
    ) {
        Ok(result) if result.success => {
            eprintln!("{}\n📁  Output saved to: {}", result.message, final_output);
        }
        Ok(result) => {
            eprintln!("❌  Office document conversion failed: {}", result.message);
        }
        Err(e) => {
            eprintln!("❌  Error: {}", e);
        }
    }

    Ok(())
}

fn read_input(input: &mut dyn Read, is_console: bool) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    if is_console {
        let mut chunk = [0; 1024];
        while let Ok(bytes_read) = input.read(&mut chunk) {
            if bytes_read == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..bytes_read]);
        }
    } else {
        input.read_to_end(&mut buffer)?;
    }
    Ok(buffer)
}

fn decode_input(buffer: &[u8], enc: &str) -> io::Result<String> {
    if enc.eq_ignore_ascii_case("UTF-8") {
        return Ok(String::from_utf8_lossy(buffer).into_owned());
    }
    let encoding = Encoding::for_label(enc.as_bytes()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unsupported encoding: {enc}"),
        )
    })?;
    let mut reader = DecodeReaderBytesBuilder::new()
        .encoding(Some(encoding))
        .build(buffer);
    let mut decoded = String::new();
    reader.read_to_string(&mut decoded)?;
    Ok(decoded)
}

fn encode_and_write_output(output_str: &str, enc: &str, output: &mut dyn Write) -> io::Result<()> {
    if enc.eq_ignore_ascii_case("UTF-8") {
        write!(output, "{}", output_str)
    } else {
        let encoding = Encoding::for_label(enc.as_bytes()).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unsupported encoding: {enc}"),
            )
        })?;
        let (encoded, _, _) = encoding.encode(output_str);
        output.write_all(&encoded)
    }
}

fn remove_utf8_bom(input: &mut Vec<u8>) {
    if input.starts_with(&[0xEF, 0xBB, 0xBF]) {
        input.drain(..3);
    }
}

fn parse_custom_dict_spec(
    arg: &str,
) -> Result<CustomDictFileSpec<PathBuf>, Box<dyn std::error::Error>> {
    let mut parts = arg.splitn(3, ':');

    let slot = parts.next().ok_or("Missing custom dict slot")?;

    let mode = parts.next().ok_or("Missing custom dict mode")?;

    let file = parts.next().ok_or("Missing custom dict file")?;

    let slot = DictSlot::try_from(normalize_dict_slot_name(slot))
        .map_err(|_| format!("Unknown custom dictionary slot: {slot}"))?;

    let mode = match mode.to_ascii_lowercase().as_str() {
        "append" => CustomDictMode::Append,
        "override" => CustomDictMode::Override,
        other => return Err(format!("Unknown custom dict mode: {other}").into()),
    };

    Ok(CustomDictFileSpec {
        slot,
        files: vec![PathBuf::from(file)],
        mode,
    })
}

fn normalize_dict_slot_name(s: &str) -> &str {
    match s.to_ascii_lowercase().as_str() {
        "stcharacters" => "STCharacters",
        "stphrases" => "STPhrases",

        "tscharacters" => "TSCharacters",
        "tsphrases" => "TSPhrases",

        "twphrases" => "TWPhrases",
        "twphrasesrev" => "TWPhrasesRev",

        "twvariants" => "TWVariants",
        "twvariantsphrases" => "TWVariantsPhrases",
        "twvariantsrev" => "TWVariantsRev",
        "twvariantsrevphrases" => "TWVariantsRevPhrases",

        "hkphrases" => "HKPhrases",
        "hkphrasesrev" => "HKPhrasesRev",

        "hkvariants" => "HKVariants",
        "hkvariantsphrases" => "HKVariantsPhrases",
        "hkvariantsrev" => "HKVariantsRev",
        "hkvariantsrevphrases" => "HKVariantsRevPhrases",

        "jpscharacters" => "JPSCharacters",
        "jpscharactersrev" => "JPSCharactersRev",
        "jpsphrases" => "JPSPhrases",

        "stpunctuations" => "STPunctuations",
        "tspunctuations" => "TSPunctuations",

        _ => s,
    }
}
