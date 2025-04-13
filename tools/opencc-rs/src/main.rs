use std::fs::File;
use std::io::{self, BufReader, BufWriter, IsTerminal, Read, Write};
use clap::{Arg, Command};
use encoding_rs::Encoding;
use encoding_rs_io::DecodeReaderBytesBuilder;

use opencc_fmmseg;
use opencc_fmmseg::OpenCC;

const CONFIG_LIST: [&str; 16] = [
    "s2t", "t2s", "s2tw", "tw2s", "s2twp", "tw2sp", "s2hk", "hk2s", "t2tw", "t2twp", "t2hk",
    "tw2t", "tw2tp", "hk2t", "t2jp", "jp2t",
];

fn read_input(input: &mut dyn Read, is_console: bool) -> Result<Vec<u8>, io::Error> {
    let mut buffer = Vec::new();

    if is_console {
        // Read chunks of data when input is from the console
        let mut chunk = [0; 1024]; // 1 KB chunks
        while let Ok(bytes_read) = input.read(&mut chunk) {
            if bytes_read == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..bytes_read]);
        }
    } else {
        // Read the entire input at once when it's from a file
        input.read_to_end(&mut buffer)?;
    }

    Ok(buffer)
}

fn decode_input(buffer: &[u8], in_enc: &str) -> Result<String, io::Error> {
    match in_enc {
        "UTF-8" => Ok(String::from_utf8_lossy(buffer).into_owned()),
        _ => {
            let encoding = Encoding::for_label(in_enc.as_bytes()).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unsupported input encoding: {}", in_enc),
                )
            })?;
            let mut decoder = DecodeReaderBytesBuilder::new()
                .encoding(Some(encoding))
                .build(buffer);
            let mut decoded = String::new();
            decoder.read_to_string(&mut decoded)?;
            Ok(decoded)
        }
    }
}

fn encode_and_write_output(
    output_str: &str,
    out_enc: &str,
    output: &mut dyn Write,
) -> Result<(), io::Error> {
    match out_enc {
        "UTF-8" => write!(output, "{}", output_str),
        _ => {
            let encoding = Encoding::for_label(out_enc.as_bytes()).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unsupported output encoding: {}", out_enc),
                )
            })?;
            let (encoded_bytes, _, _) = encoding.encode(output_str);
            output.write_all(&encoded_bytes)
        }
    }
}

fn remove_utf8_bom(input: &mut Vec<u8>) {
    // UTF-8 BOM: EF BB BF
    if input.len() >= 3 && &input[0..3] == &[0xEF, 0xBB, 0xBF] {
        input.drain(0..3); // Remove BOM from the beginning
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
                .default_value("false")
                .help("Punctuation conversion: [true|false]"),
        )
        .arg(
            Arg::new("in_enc")
                .long("in-enc")
                .value_name("encoding")
                .default_value("UTF-8")
                .help("Encoding for input: UTF-8|GB2312|GBK|gb18030|BIG5"),
        )
        .arg(
            Arg::new("out_enc")
                .long("out-enc")
                .value_name("encoding")
                .default_value("UTF-8")
                .help("Encoding for output: UTF-8|GB2312|GBK|gb18030|BIG5"),
        )
        .about(format!(
            "{BLUE}OpenCC Rust: Command Line Open Chinese Converter{RESET}"
        ))
        .get_matches();

    let input_file = matches.get_one::<String>("input");
    let output_file = matches.get_one::<String>("output");
    let config = matches.get_one::<String>("config").unwrap();
    if !CONFIG_LIST.contains(&config.as_str()) {
        eprintln!("Invalid config: {}", config);
        eprintln!("Valid Configs: {:?}", CONFIG_LIST);
        return Ok(());
    }
    let punctuation = matches
        .get_one::<String>("punct")
        .map_or(false, |value| value == "true");
    let in_enc = matches.get_one::<String>("in_enc").unwrap();
    let out_enc = matches.get_one::<String>("out_enc").unwrap();

    // Determine input source
    let mut input: Box<dyn Read> = match input_file {
        Some(file_name) => Box::new(BufReader::new(File::open(file_name)?)),
        None => {
            if io::stdin().is_terminal() {
                // If input is from the terminal
                println!("{BLUE}Input text to convert, <ctrl-z> or <ctrl-d> to submit:{RESET}");
            }
            Box::new(io::stdin())
        }
    };

    // Read input with chunked reading for console, or all at once for files
    let is_console = input_file.is_none();
    let mut buffer = read_input(&mut *input, is_console)?;

    // Remove BOM if present in UTF-8 input
    if in_enc == "UTF-8" && out_enc != "UTF-8" {
        remove_utf8_bom(&mut buffer);
    }

    // Decode input based on encoding
    let input_str = decode_input(&buffer, in_enc)?;

    // Initialize OpenCC and convert text
    let opencc = OpenCC::new();
    let output_str = opencc.convert(&input_str, config, punctuation);

    // Initialize output writer
    let mut output = BufWriter::new(match output_file {
        Some(file_name) => Box::new(File::create(file_name)?) as Box<dyn Write>,
        None => Box::new(io::stdout()) as Box<dyn Write>,
    });

    // Encode and write output
    encode_and_write_output(&output_str, out_enc, &mut output)?;

    // Print conversion summary
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
