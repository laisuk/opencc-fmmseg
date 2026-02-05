mod json_io;

use crate::json_io::DictionaryMaxlengthSerde;
use clap::{Arg, Command};
use opencc_fmmseg::dictionary_lib::DictionaryMaxlength;
use std::fs::File;
use std::io;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const BLUE: &str = "\x1B[1;34m"; // Bold Blue
    const RESET: &str = "\x1B[0m"; // Reset color

    let matches = Command::new("Dictionary Generator")
        .about(format!(
            "{BLUE}Dict Generator: Dictionary Artifacts Generator from dictionaries in ./dicts/{RESET}"
        ))
        .after_help(
            "Examples:\n\
         \n\
         dict-generate --format cbor --output dictionary_maxlength.cbor\n\
         dict-generate --format zstd --output dictionary_maxlength.zstd\n\
         \n\
         The generated CBOR can be loaded with DictionaryMaxlength::deserialize_from_cbor().\n"
        )
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .value_name("format")
                .default_value("zstd")
                .value_parser(["zstd", "cbor", "json"])
                .help("Dictionary format: [zstd|cbor|json]"),
        )
        .arg(
            Arg::new("pretty")
                .long("pretty")
                .action(clap::ArgAction::SetTrue)
                .help("Pretty-print JSON when --format json")
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("filename")
                .help("Write generated dictionary to <filename>. If not specified, a default filename is used."),
        )
        .get_matches();

    let dict_dir = Path::new("dicts");
    if !dict_dir.exists() {
        eprintln!(
            "{BLUE}Local 'dicts/' directory not found.{RESET}\n\
         Please place Opencc-Fmmseg dictionary files (*.txt) under this folder."
        );
        return Ok(()); // Exit silently
    }

    let dict_format = matches.get_one::<String>("format").map(String::as_str);
    let pretty_json = matches.get_flag("pretty"); // default compact if false

    let default_output = match dict_format {
        Some("zstd") => "dictionary_maxlength.zstd",
        Some("cbor") => "dictionary_maxlength.cbor",
        Some("json") => "dictionary_maxlength.json",
        _ => "dictionary_maxlength.unknown",
    };

    let output_file = matches
        .get_one::<String>("output")
        .map(|s| s.as_str())
        .unwrap_or(default_output);

    match dict_format {
        Some("zstd") => {
            let dictionary = DictionaryMaxlength::from_dicts()?;
            DictionaryMaxlength::save_cbor_compressed(&dictionary, output_file)?;
            eprintln!("{BLUE}Dictionary saved in ZSTD format at: {output_file}{RESET}");
        }
        Some("cbor") => {
            let dictionary = DictionaryMaxlength::from_dicts()?;
            dictionary.serialize_to_cbor(output_file)?;
            eprintln!("{BLUE}Dictionary saved in CBOR format at: {output_file}{RESET}");
        }
        Some("json") => {
            let dictionary = DictionaryMaxlength::from_dicts()?;
            // IMPORTANT: use DTO for JSON so keys are Strings
            write_reference_json(&dictionary, output_file, /* pretty = */ pretty_json)?;
            let style = if pretty_json { "pretty" } else { "compact" };
            eprintln!("{BLUE}Dictionary saved in JSON ({style}) at: {output_file}{RESET}");
        }
        other => {
            let format_str = other.unwrap_or("unknown");
            eprintln!("{BLUE}Unsupported format: {format_str}{RESET}");
        }
    }

    Ok(())
}
pub fn write_reference_json(
    dicts: &DictionaryMaxlength,
    path: impl AsRef<Path>,
    pretty: bool,
) -> io::Result<()> {
    let dto: DictionaryMaxlengthSerde = dicts.into();
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    if pretty {
        serde_json::to_writer_pretty(&mut w, &dto).map_err(to_io)?;
    } else {
        serde_json::to_writer(&mut w, &dto).map_err(to_io)?;
        // newline for POSIX-y tools
        w.write_all(b"\n")?;
    }
    w.flush()
}

// Small adapter so we can stay in io::Result
fn to_io<E: std::error::Error + Send + Sync + 'static>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::Other, e)
}
