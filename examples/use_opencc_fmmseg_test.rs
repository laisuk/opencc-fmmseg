use opencc_fmmseg::{OpenCC, OpenccConfig};

fn main() {
    // ---------------------------------------------------------------------
    // Sample UTF-8 input (same spirit as C / C++ demos)
    // ---------------------------------------------------------------------
    let input_text = "意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";

    println!("Text:");
    println!("{}", input_text);
    println!();

    // ---------------------------------------------------------------------
    // Create OpenCC instance
    // ---------------------------------------------------------------------
    let converter = OpenCC::new();

    // Detect script
    let input_code = converter.zho_check(input_text);
    println!("Text Code: {}", input_code);

    // ---------------------------------------------------------------------
    // Test 1: Legacy string-based config (convert)
    // ---------------------------------------------------------------------
    let config_str = "s2twp";
    let punct = true;

    println!();
    println!(
        "== Test 1: convert(config = \"{}\", punctuation = {}) ==",
        config_str, punct
    );

    let output1 = converter.convert(input_text, config_str, punct);
    println!("Converted:");
    println!("{}", output1);
    println!("Converted Code: {}", converter.zho_check(&output1));
    println!(
        "Last Error: {}",
        OpenCC::get_last_error().unwrap_or_else(|| "<none>".to_string())
    );

    // ---------------------------------------------------------------------
    // Test 2: Strongly typed config (convert_with_config)
    // ---------------------------------------------------------------------
    let config_enum = OpenccConfig::S2twp;

    println!();
    println!(
        "== Test 2: convert_with_config(config = {:?}, punctuation = {}) ==",
        config_enum, punct
    );

    let output2 = converter.convert_with_config(input_text, config_enum, punct);
    println!("Converted:");
    println!("{}", output2);
    println!("Converted Code: {}", converter.zho_check(&output2));
    println!(
        "Last Error: {}",
        OpenCC::get_last_error().unwrap_or_else(|| "<none>".to_string())
    );

    // ---------------------------------------------------------------------
    // Test 3: Invalid config (string) — self-protected
    // ---------------------------------------------------------------------
    let invalid_config = "what_is_this";

    println!();
    println!(
        "== Test 3: invalid string config (\"{}\") ==",
        invalid_config
    );

    let output3 = converter.convert(input_text, invalid_config, true);
    println!("Returned:");
    println!("{}", output3);
    println!(
        "Last Error: {}",
        OpenCC::get_last_error().unwrap_or_else(|| "<none>".to_string())
    );

    // ---------------------------------------------------------------------
    // Test 4: Clear last error and verify state reset
    // ---------------------------------------------------------------------
    println!();
    println!("== Test 4: clear_last_error() ==");

    OpenCC::clear_last_error();

    println!(
        "Last Error after clear: {}",
        OpenCC::get_last_error().unwrap_or_else(|| "<none>".to_string())
    );

    // ---------------------------------------------------------------------
    // Summary
    // ---------------------------------------------------------------------
    println!();
    println!("All tests completed.");
}
