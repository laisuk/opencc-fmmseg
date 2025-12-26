use opencc_fmmseg::OpenCC;
use std::process;

fn main() {
    let input_file = "examples/OneDay.txt";

    let input_text = match std::fs::read_to_string(input_file) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("❌ Failed to read file '{}': {}", input_file, e);
            process::exit(1);
        }
    };

    let converter = OpenCC::new();
    let input_code = converter.zho_check(&input_text);

    let config = match input_code {
        1 => "t2s",
        _ => "s2t",
    };

    let punct = true;
    let output_text = converter.convert(&input_text, config, punct);

    println!(
        "Input code: {}, config: {}, punctuation: {}",
        input_code, config, punct
    );
    println!("Converted:\n{}", output_text);
}
