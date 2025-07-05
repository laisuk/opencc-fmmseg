extern crate copypasta;

use std::collections::HashSet;
use std::env;

use copypasta::{ClipboardContext, ClipboardProvider};
use once_cell::sync::Lazy;
use opencc_fmmseg::{find_max_utf8_length, OpenCC};

#[derive(Debug, PartialEq)]
enum ConversionType {
    S2T,
    T2S,
    S2TW,
    TW2S,
    S2TWP,
    TW2SP,
    S2HK,
    HK2S,
    T2TW,
    T2TWP,
    T2HK,
    TW2T,
    TW2TP,
    HK2T,
    T2JP,
    JP2T,
    Auto,
    None,
}

impl ConversionType {
    fn from_str(s: &str) -> Self {
        match s {
            "s2t" => Self::S2T,
            "t2s" => Self::T2S,
            "s2tw" => Self::S2TW,
            "tw2s" => Self::TW2S,
            "s2twp" => Self::S2TWP,
            "tw2sp" => Self::TW2SP,
            "s2hk" => Self::S2HK,
            "hk2s" => Self::HK2S,
            "t2tw" => Self::T2TW,
            "t2twp" => Self::T2TWP,
            "t2hk" => Self::T2HK,
            "tw2t" => Self::TW2T,
            "tw2tp" => Self::TW2TP,
            "hk2t" => Self::HK2T,
            "t2jp" => Self::T2JP,
            "jp2t" => Self::JP2T,
            "auto" => Self::Auto,
            _ => Self::None,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::S2T => "s2t",
            Self::T2S => "t2s",
            Self::S2TW => "s2tw",
            Self::TW2S => "tw2s",
            Self::S2TWP => "s2twp",
            Self::TW2SP => "tw2sp",
            Self::S2HK => "s2hk",
            Self::HK2S => "hk2s",
            Self::T2TW => "t2tw",
            Self::T2TWP => "t2twp",
            Self::T2HK => "t2hk",
            Self::TW2T => "tw2t",
            Self::TW2TP => "tw2tp",
            Self::HK2T => "hk2t",
            Self::T2JP => "t2jp",
            Self::JP2T => "jp2t",
            Self::Auto => "auto",
            Self::None => "none",
        }
    }
    fn is_japanese(&self) -> bool {
        self == &Self::T2JP || self == &Self::JP2T
    }
}

pub static CONFIG_LIST: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "s2t", "t2s", "s2tw", "tw2s", "s2twp", "tw2sp", "s2hk", "hk2s", "t2tw", "t2twp", "t2hk",
        "tw2t", "tw2tp", "hk2t", "t2jp", "jp2t",
    ]
    .iter()
    .cloned()
    .collect()
});

fn main() {
    const RED: &str = "\x1B[1;31m";
    const GREEN: &str = "\x1B[1;32m";
    const YELLOW: &str = "\x1B[1;33m";
    const BLUE: &str = "\x1B[1;34m";
    const RESET: &str = "\x1B[0m";

    let args: Vec<String> = env::args().collect();
    let mut conversion_type = ConversionType::Auto;
    let mut use_punctuation = false;

    if args.len() > 1 {
        let config_arg = args[1].to_lowercase();
        if config_arg == "help" {
            eprintln!(
                "Opencc-Clip-fmmseg Zho Converter version 1.0.0 Copyright (c) 2024 Bryan Lai"
            );
            eprintln!("Usage: opencc-clip [s2t|t2s|s2tw|tw2s|s2twp|tw2sp|s2hk|hk2s|t2tw|tw2t|t2twp|tw2t|tw2tp|t2hk|hk2t|jp2t|t2jp|auto|help] [punct]\n");
            return;
        }

        if CONFIG_LIST.contains(config_arg.as_str()) {
            conversion_type = ConversionType::from_str(&config_arg);
        }

        if args.last().unwrap().to_lowercase() == "punct" {
            use_punctuation = true;
        }
    }
    // Create a new clipboard context
    let mut ctx: ClipboardContext = match ClipboardContext::new() {
        Ok(context) => context,
        Err(err) => {
            eprintln!("{}Error creating clipboard context: {}{}", RED, err, RESET);
            return;
        }
    };

    // Attempt to read text from the clipboard
    match ctx.get_contents() {
        Ok(contents) => {
            let opencc = OpenCC::new();
            // opencc.set_parallel(false);
            let input_code = opencc.zho_check(&contents);

            if conversion_type == ConversionType::Auto {
                conversion_type = match input_code {
                    1 => ConversionType::T2S,
                    2 => ConversionType::S2T,
                    _ => ConversionType::None,
                };
            }

            let (display_input_code, display_output_code) =
                if input_code == 0 || conversion_type.is_japanese() {
                    ("Non-zho 其它", "Non-zho 其它")
                } else if conversion_type.as_str().starts_with('s') {
                    ("Simplified Chinese 简体", "Traditional Chinese 繁体")
                } else if conversion_type.as_str().ends_with('s')
                    || conversion_type.as_str().ends_with("sp")
                {
                    ("Traditional Chinese 繁体", "Simplified Chinese 简体")
                } else {
                    ("Traditional Chinese 繁体", "Traditional Chinese 繁体")
                };

            let output = if conversion_type != ConversionType::None {
                opencc.convert(&contents, conversion_type.as_str(), use_punctuation)
            } else {
                contents.clone()
            };

            let (display_input, display_output, ellipsis) = if contents.len() > 600 {
                let contents_max_utf8_length = find_max_utf8_length(&contents, 600);
                let output_max_utf8_length = find_max_utf8_length(&output, 600);
                (
                    &contents[..contents_max_utf8_length],
                    &output[..output_max_utf8_length],
                    "...",
                )
            } else {
                (contents.as_str(), output.as_str(), "")
            };

            eprintln!(
                "Opencc-Clip-fmmseg Zho Converter version 1.0.0 Copyright (c) 2024 Bryan Lai"
            );
            eprintln!(
                "Config: {}{}, {}",
                BLUE,
                conversion_type.as_str(),
                use_punctuation
            );
            eprintln!(
                "{}Clipboard Input ({}):\n{}{}{}\n",
                GREEN, display_input_code, YELLOW, display_input, ellipsis
            );
            eprintln!(
                "{}Converted Output ({}):\n{}{}{}{}",
                GREEN, display_output_code, YELLOW, display_output, ellipsis, RESET
            );

            if let Err(err) = ctx.set_contents(output) {
                eprintln!("{}Error setting clipboard: {}{}", RED, err, RESET);
            } else {
                let input_length = contents.chars().count();
                eprintln!(
                    "{}(Output set to clipboard: {} chars){}",
                    BLUE,
                    format_thousand(input_length),
                    RESET
                );
            }
        }
        Err(err) => {
            // If an error occurs, print the error message
            eprintln!("{}No text in clipboard: {}{}", RED, err, RESET)
        }
    }
}

pub fn format_thousand(n: usize) -> String {
    let mut result_str = n.to_string();
    let mut offset = result_str.len() % 3;
    if offset == 0 {
        offset = 3;
    }

    while offset < result_str.len() {
        result_str.insert(offset, ',');
        offset += 4; // Including the added comma
    }
    result_str
}
