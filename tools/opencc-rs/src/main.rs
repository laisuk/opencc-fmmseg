use clap::{App, Arg};
use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
use std::string::String;

use opencc_fmmseg;
use opencc_fmmseg::OpenCC;

fn main() -> Result<(), io::Error> {
    const BLUE: &str = "\x1B[1;34m";
    const RESET: &str = "\x1B[0m";
    let matches = App::new("OpenCC Rust: Command Line Open Chinese Converter")
        .arg(
            Arg::with_name("input")
                .short('i')
                .long("input")
                .value_name("file")
                .help("Read original text from <file>.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                .short('o')
                .long("output")
                .value_name("file")
                .help("Write converted text to <file>.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("config")
                .short('c')
                .long("config")
                .value_name("conversion")
                .help(
                    "Conversion configuration: [s2t|s2tw|s2twp|s2hk|t2s|tw2s|tw2sp|hk2s|jp2t|t2jp]",
                )
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("punct")
                .short('p')
                .long("punct")
                .value_name("boolean")
                .help("Punctuation conversion: [true|false]")
                .takes_value(true),
        )
        .get_matches();

    let input_file = matches.value_of("input");
    let output_file = matches.value_of("output");
    let config = matches.value_of("config").unwrap();
    let punctuation = matches
        .value_of("punct")
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
            "{BLUE}Conversion completed: {} -> {}{RESET}",
            input_file,
            output_file.unwrap_or("stdout").to_string()
        );
    } else {
        println!(
            "{BLUE}Conversion completed: <stdin> -> {}{RESET}",
            output_file.unwrap_or("stdout")
        );
    }

    Ok(())
}
