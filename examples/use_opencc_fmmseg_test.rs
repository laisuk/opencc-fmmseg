use opencc_fmmseg::{
    CustomDictMode, CustomDictSpec, DetofuLevel, DictSlot, DictionaryMaxlength, OpenCC,
    OpenccConfig,
};

fn main() {
    // ---------------------------------------------------------------------
    // Test 1: Basic conversion with the typed config API
    // ---------------------------------------------------------------------
    let opencc = OpenCC::new();

    let input = "意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";

    println!("Text: {}", input);
    println!("Text Code: {}", opencc.zho_check(input));

    println!();
    println!("== Test 1: typed conversion ==");

    let traditional = opencc.convert_with_config(input, OpenccConfig::S2twp, true);

    println!("Converted: {}", traditional);
    println!("Converted Code: {}", opencc.zho_check(&traditional));

    // ---------------------------------------------------------------------
    // Test 2: String-config compatibility API
    // ---------------------------------------------------------------------
    println!();
    println!("== Test 2: string config compatibility API ==");

    let by_name = opencc.convert(input, "s2twp", true);

    println!("Converted: {}", by_name);
    println!(
        "Same result: {}",
        if by_name == traditional {
            "PASS"
        } else {
            "FAIL"
        }
    );

    // Invalid string configs remain self-protected and report the error text.
    let invalid = opencc.convert(input, "what_is_this", false);
    println!("Invalid config returned: {}", invalid);

    OpenCC::clear_last_error();

    // ---------------------------------------------------------------------
    // Test 3: Immutable custom dictionary roundtrip
    // ---------------------------------------------------------------------
    println!();
    println!("== Test 3: immutable custom dictionary roundtrip ==");

    let custom_specs = [
        CustomDictSpec {
            slot: DictSlot::STPhrases,
            pairs: vec![
                ("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string()),
                ("软件".to_string(), "軟體".to_string()),
            ],
            mode: CustomDictMode::Append,
        },
        CustomDictSpec {
            slot: DictSlot::TSPhrases,
            pairs: vec![
                ("柏蘭蒂爾".to_string(), "帕兰蒂尔".to_string()),
                ("軟體".to_string(), "软件".to_string()),
            ],
            mode: CustomDictMode::Append,
        },
    ];

    let custom_dictionary = DictionaryMaxlength::from_zstd()
        .expect("failed to load embedded dictionaries")
        .with_custom_dicts(&custom_specs)
        .expect("failed to apply custom dictionaries");

    let custom_opencc = OpenCC::from_dictionary(custom_dictionary);

    let source = "帕兰蒂尔是一家软件公司。";
    let custom_traditional = custom_opencc.convert_with_config(source, OpenccConfig::S2t, false);
    let custom_simplified =
        custom_opencc.convert_with_config(&custom_traditional, OpenccConfig::T2s, false);

    println!("Source:      {}", source);
    println!("S2T custom:  {}", custom_traditional);
    println!("T2S custom:  {}", custom_simplified);
    println!(
        "Roundtrip:   {}",
        if custom_simplified == source {
            "PASS"
        } else {
            "FAIL"
        }
    );

    // ---------------------------------------------------------------------
    // Test 4: Compatibility normalization
    // ---------------------------------------------------------------------
    println!();
    println!("== Test 4: compatibility normalization ==");

    let compat_source = "天龍八部書";
    let compat_normalized = opencc.normalize_compat(compat_source);

    println!("Source:       {}", compat_source);
    println!("Norm compat:  {}", compat_normalized);
    println!(
        "Result:       {}",
        if compat_normalized == "天龍八部書" {
            "PASS"
        } else {
            "FAIL"
        }
    );

    // ---------------------------------------------------------------------
    // Test 5: Extended normalization -> conversion
    // ---------------------------------------------------------------------
    println!();
    println!("== Test 5: extended normalization -> T2S ==");

    let extended_source = "天龍八部書裡的聼眾‧聼聼竒羙⽟䂖甁噐⾳";

    let normalized = opencc.normalize_compat_extended(extended_source);
    let simplified = opencc.convert_with_config(&normalized, OpenccConfig::T2s, false);

    let norm_ok = normalized == "天龍八部書裡的聽眾·聽聽奇美玉石瓶器音";
    let t2s_ok = simplified == "天龙八部书里的听众·听听奇美玉石瓶器音";

    println!("Source:         {}", extended_source);
    println!("Norm extended:  {}", normalized);
    println!("T2S:            {}", simplified);
    println!(
        "Pipeline:       {}",
        if norm_ok && t2s_ok { "PASS" } else { "FAIL" }
    );

    // ---------------------------------------------------------------------
    // Test 6: DeTofu post-processing
    // ---------------------------------------------------------------------
    println!();
    println!("== Test 6: DeTofu ExtB ==");

    let detofu_source = "骖𬴂";
    let detofued = opencc.detofu(detofu_source, DetofuLevel::ExtB);

    println!("Source:      {}", detofu_source);
    println!("DeTofu:      {}", detofued);
    println!(
        "Result:      {}",
        if detofued == "骖騑" { "PASS" } else { "FAIL" }
    );

    // ---------------------------------------------------------------------
    // Test 7: Recommended full compatibility pipeline
    // ---------------------------------------------------------------------
    println!();
    println!("== Test 7: normalize -> convert -> DeTofu ==");

    let pipeline_source = "天龍八部書裡的聼眾，儼驂騑於上路。";

    let pipeline_normalized = opencc.normalize_compat_extended(pipeline_source);

    let pipeline_converted =
        opencc.convert_with_config(&pipeline_normalized, OpenccConfig::T2s, false);

    let pipeline_display = opencc.detofu(&pipeline_converted, DetofuLevel::ExtB);

    println!("Source:      {}", pipeline_source);
    println!("Normalized:  {}", pipeline_normalized);
    println!("Converted:   {}", pipeline_converted);
    println!("Display:     {}", pipeline_display);

    // ---------------------------------------------------------------------
    // Test 8: Small Seal Script conversion roundtrip
    // ---------------------------------------------------------------------
    println!();
    println!("== Test 8: Small Seal Script conversion roundtrip ==");

    let seal_traditional = "你好，小篆國際編碼18";
    let seal_simplified = "你好，小篆国际编码18";
    let seal_expected = "你𿒛，𽌠𽴖𾇓𿭖𿛛碼18";

    let t2seal = opencc.convert_with_config(seal_traditional, OpenccConfig::T2seal, false);
    let s2seal = opencc.convert_with_config(seal_simplified, OpenccConfig::S2seal, false);
    let seal2t = opencc.convert_with_config(seal_expected, OpenccConfig::Seal2t, false);
    let seal2s = opencc.convert_with_config(seal_expected, OpenccConfig::Seal2s, false);

    let seal_punctuation =
        opencc.convert_with_config("你好，小篆“國際編碼18”", OpenccConfig::T2seal, true);

    let t2seal_ok = t2seal == seal_expected;
    let s2seal_ok = s2seal == seal_expected;
    let seal2t_ok = seal2t == seal_traditional;
    let seal2s_ok = seal2s == seal_simplified;
    let punctuation_ok = seal_punctuation == "你𿒛，𽌠𽴖「𾇓𿭖𿛛碼18」";

    println!(
        "T2Seal:            {} [{}]",
        t2seal,
        if t2seal_ok { "PASS" } else { "FAIL" }
    );
    println!(
        "S2Seal:            {} [{}]",
        s2seal,
        if s2seal_ok { "PASS" } else { "FAIL" }
    );
    println!(
        "Seal2T:            {} [{}]",
        seal2t,
        if seal2t_ok { "PASS" } else { "FAIL" }
    );
    println!(
        "Seal2S:            {} [{}]",
        seal2s,
        if seal2s_ok { "PASS" } else { "FAIL" }
    );
    println!(
        "T2Seal punctuation: {} [{}]",
        seal_punctuation,
        if punctuation_ok { "PASS" } else { "FAIL" }
    );
    println!(
        "Roundtrip:          {}",
        if t2seal_ok && s2seal_ok && seal2t_ok && seal2s_ok && punctuation_ok {
            "PASS"
        } else {
            "FAIL"
        }
    );

    println!();
    println!("All tests completed.");
}
