use std::collections::HashMap;
use std::sync::OnceLock;

static TOFU_DATA: &[u8] = include_bytes!("data/TSCharactersTofu.txt");

/// Controls which CJK Extension ranges are replaced by detofu.
///
/// The selected level acts as a minimum threshold.
///
/// - ExtB = replace ExtB, ExtC, ExtD, ExtE, ExtF, ExtG, ExtH, ExtI
/// - ExtC = replace ExtC, ExtD, ExtE, ExtF, ExtG, ExtH, ExtI
/// - ExtD = replace ExtD, ExtE, ExtF, ExtG, ExtH, ExtI
/// - ExtE = replace ExtE, ExtF, ExtG, ExtH, ExtI
///
/// `ExtB` is therefore equivalent to "all".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DetofuLevel {
    ExtB,
    ExtC,
    ExtD,
    ExtE,
    ExtF,
    ExtG,
    ExtH,
    ExtI,
}

impl DetofuLevel {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "all" | "ext-b" | "b" => Ok(Self::ExtB),
            "ext-c" | "c" => Ok(Self::ExtC),
            "ext-d" | "d" => Ok(Self::ExtD),
            "ext-e" | "e" => Ok(Self::ExtE),
            "ext-f" | "f" => Ok(Self::ExtF),
            "ext-g" | "g" => Ok(Self::ExtG),
            "ext-h" | "h" => Ok(Self::ExtH),
            "ext-i" | "i" => Ok(Self::ExtI),
            _ => Err("supported detofu levels: all, ext-b, ext-c, ext-d, ext-e, ext-f, ext-g, ext-h, ext-i".to_string()),
        }
    }

    fn from_ext(ext: &str) -> Option<Self> {
        match ext {
            "ExtB" => Some(Self::ExtB),
            "ExtC" => Some(Self::ExtC),
            "ExtD" => Some(Self::ExtD),
            "ExtE" => Some(Self::ExtE),
            "ExtF" => Some(Self::ExtF),
            "ExtG" => Some(Self::ExtG),
            "ExtH" => Some(Self::ExtH),
            "ExtI" => Some(Self::ExtI),
            _ => None,
        }
    }
}

static TOFU_ENTRIES: OnceLock<Vec<(char, char, DetofuLevel)>> = OnceLock::new();

fn tofu_entries() -> &'static [(char, char, DetofuLevel)] {
    TOFU_ENTRIES.get_or_init(|| {
        let text =
            std::str::from_utf8(TOFU_DATA).expect("TSCharactersTofu.txt must be valid UTF-8");

        text.lines()
            .filter(|line| {
                let line = line.trim();
                !line.is_empty() && !line.starts_with('#')
            })
            .filter_map(|line| {
                let mut parts = line.split('\t');
                let tofu = parts.next()?.chars().next()?;
                let fallback = parts.next()?.chars().next()?;
                let ext = DetofuLevel::from_ext(parts.next()?)?;
                Some((tofu, fallback, ext))
            })
            .collect()
    })
}

#[derive(Debug, Clone)]
pub struct DetofuMap {
    map: HashMap<char, char>,
}

impl DetofuMap {
    pub fn builtin(level: DetofuLevel) -> Self {
        let map = tofu_entries()
            .iter()
            .filter(|(_, _, ext)| *ext >= level)
            .map(|(tofu, fallback, _)| (*tofu, *fallback))
            .collect();

        Self { map }
    }

    pub fn with_custom_pairs(mut self, pairs: &[(char, char)]) -> Self {
        for &(tofu, fallback) in pairs {
            self.map.insert(tofu, fallback);
        }
        self
    }

    pub fn detofu(&self, input: &str) -> String {
        let mut output = String::with_capacity(input.len());

        for ch in input.chars() {
            if let Some(fallback) = self.map.get(&ch) {
                output.push(*fallback);
            } else {
                output.push(ch);
            }
        }

        output
    }
}

pub fn detofu(input: &str, level: DetofuLevel) -> String {
    DetofuMap::builtin(level).detofu(input)
}
