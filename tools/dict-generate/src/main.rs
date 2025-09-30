mod json_io;

use crate::json_io::DictionaryMaxlengthSerde;
use clap::{Arg, Command};
use opencc_fmmseg::dictionary_lib::DictionaryMaxlength;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::time::Duration;
use std::{fs, io};
use ureq::Agent;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const BLUE: &str = "\x1B[1;34m"; // Bold Blue
    const RESET: &str = "\x1B[0m"; // Reset color

    let matches = Command::new("Dictionary Generator")
        .about(format!(
            "{BLUE}Dict Generator: Command Line Dictionary Generator{RESET}"
        ))
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .value_name("format")
                .default_value("zstd")
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
        eprint!("{BLUE}Local 'dicts/' not found. Proceed with downloading dictionaries from GitHub? (Y/n): {RESET}");
        io::stdout().flush()?; // Ensure prompt is printed before read_line

        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        let answer = answer.trim().to_lowercase();

        if answer.is_empty() || answer == "y" || answer == "yes" {
            eprintln!("{BLUE}Downloading from GitHub...{RESET}");
            fetch_dicts_from_github(dict_dir)?;
        } else {
            eprintln!("{BLUE}Aborted by user. Exiting.{RESET}");
            return Ok(()); // or `std::process::exit(0);` if you want a hard exit
        }
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
            DictionaryMaxlength::save_compressed(&dictionary, output_file)?;
            eprintln!("{BLUE}Dictionary saved in ZSTD format at: {output_file}{RESET}");
        }
        Some("cbor") => {
            let dictionary = DictionaryMaxlength::from_dicts()?;
            let file = File::create(output_file)?;
            serde_cbor::to_writer(file, &dictionary)?;
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
/// Download missing dict files from GitHub repo
fn fetch_dicts_from_github(dict_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let dict_files = [
        "STCharacters.txt",
        "STPhrases.txt",
        "TSCharacters.txt",
        "TSPhrases.txt",
        "TWPhrases.txt",
        "TWPhrasesRev.txt",
        "TWVariants.txt",
        "TWVariantsRev.txt",
        "TWVariantsRevPhrases.txt",
        "HKVariants.txt",
        "HKVariantsRev.txt",
        "HKVariantsRevPhrases.txt",
        "JPShinjitaiCharacters.txt",
        "JPShinjitaiPhrases.txt",
        "JPVariants.txt",
        "JPVariantsRev.txt",
        "STPunctuations.txt",
        "TSPunctuations.txt",
    ];

    fs::create_dir_all(dict_dir)?;

    let config = Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build();
    let agent: Agent = config.into();

    for filename in &dict_files {
        let url = format!(
            "https://raw.githubusercontent.com/laisuk/opencc-fmmseg/master/dicts/{}",
            filename
        );

        let response = agent.get(&url).call()?;
        let mut content = String::new();
        response
            .into_body()
            .into_reader()
            .read_to_string(&mut content)?;

        let dest_path = dict_dir.join(filename);
        let mut file = File::create(dest_path)?;
        file.write_all(content.as_bytes())?;

        eprintln!("Downloaded: {}", filename);
    }

    Ok(())
}
