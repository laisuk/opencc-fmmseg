use opencc_fmmseg::{CustomDictFileSpec, CustomDictMode, DictSlot};
use std::path::PathBuf;

pub fn parse_custom_dict_spec(
    arg: &str,
) -> Result<CustomDictFileSpec<PathBuf>, Box<dyn std::error::Error>> {
    let mut parts = arg.splitn(3, ':');

    let slot = parts.next().ok_or("Missing custom dict slot")?;
    let mode = parts.next().ok_or("Missing custom dict mode")?;
    let file = parts.next().ok_or("Missing custom dict file")?;

    let slot_name = normalize_dict_slot_name(slot);
    let slot = DictSlot::try_from(slot_name.as_str())
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

fn normalize_dict_slot_name(s: &str) -> String {
    match s.trim().to_ascii_lowercase().as_str() {
        "stcharacters" => "STCharacters",
        "stphrases" => "STPhrases",
        "stpunctuations" => "STPunctuations",

        "tscharacters" => "TSCharacters",
        "tsphrases" => "TSPhrases",
        "tspunctuations" => "TSPunctuations",

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

        _ => s.trim(),
    }
    .to_string()
}
