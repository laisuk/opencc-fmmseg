use std::fs::File;
use std::io::{self, BufWriter, Read, Write};

use clap::{Arg, Command};

use opencc_fmmseg;
use opencc_fmmseg::OpenCC;

const CONFIG_LIST: [&str; 16] = [
    "s2t", "t2s", "s2tw", "tw2s", "s2twp", "tw2sp", "s2hk", "hk2s", "t2tw", "t2twp", "t2hk",
    "tw2t", "tw2tp", "hk2t", "t2jp", "jp2t",
];

fn main() -> Result<(), io::Error> {
    const BLUE: &str = "\x1B[1;34m";
    const RESET: &str = "\x1B[0m";
    let matches = Command::new("OpenCC Rust")
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .value_name("file")
                .help("Read original text from <file>."),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("file")
                .help("Write converted text to <file>."),
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("conversion")
                .help(
                    "Conversion configuration: [s2t|s2tw|s2twp|s2hk|t2s|tw2s|tw2sp|hk2s|jp2t|t2jp]",
                )
                .required(true),
        )
        .arg(
            Arg::new("punct")
                .short('p')
                .long("punct")
                .value_name("boolean")
                .help("Punctuation conversion: [true|false]"),
        )
        .about(format!(
            "{}OpenCC Rust: Command Line Open Chinese Converter{}",
            BLUE, RESET
        ))
        .get_matches();

    let input_file = matches.get_one::<String>("input");
    let output_file = matches.get_one::<String>("output");
    let config = matches.get_one::<String>("config").unwrap().as_str();
    if !CONFIG_LIST.contains(&config) {
        println!("Invalid config: {}", config);
        println!("Valid Config are: [s2t|s2tw|s2twp|s2hk|t2s|tw2s|tw2sp|hk2s|jp2t|t2jp]");
        return Ok(());
    }
    let punctuation = matches
        .get_one::<String>("punct")
        .map_or(false, |value| value == "true");

    let mut input: Box<dyn Read> = match input_file {
        Some(file_name) => Box::new(File::open(file_name)?),
        None => Box::new(io::stdin()),
    };

    let output: Box<dyn Write> = match output_file {
        Some(file_name) => Box::new(File::create(file_name)?),
        None => Box::new(io::stdout()),
    };

    let mut output_buf = BufWriter::new(output);

    let mut input_str = String::new();
    input.read_to_string(&mut input_str)?;

    let opencc = OpenCC::new();

    let output_str = opencc.convert(&input_str, config, punctuation);

    write!(output_buf, "{}", output_str)?;

    output_buf.flush()?; // Flush buffer to ensure all data is written

    if let Some(input_file) = input_file {
        println!(
            "{BLUE}Conversion completed ({config}): {} -> {}{RESET}",
            input_file,
            output_file.unwrap_or(&"stdout".to_string())
        );
    } else {
        println!(
            "{BLUE}Conversion completed ({config}): <stdin> -> {}{RESET}",
            output_file.unwrap_or(&"stdout".to_string())
        );
    }

    Ok(())
}
