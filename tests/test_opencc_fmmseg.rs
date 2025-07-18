use opencc_fmmseg::{dictionary_lib, OpenCC};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_cbor::to_vec;
    use std::collections::HashSet;
    use std::fs;

    #[test]
    fn zho_check_test() {
        let input = "你好，世界！龙马精神！";
        let expected_output = 2;
        let opencc = OpenCC::new();
        let actual_output = opencc.zho_check(input);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn s2t_test() {
        let input = "你好，世界！龙马精神！\t\n";
        let expected_output = "你好，世界！龍馬精神！\t\n";
        let opencc = OpenCC::new();
        let actual_output = opencc.s2t(input, false);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn s2t_extended_test() {
        let input = "俨骖𬴂于上路，访风景于崇阿";
        let expected_output = "儼驂騑於上路，訪風景於崇阿";
        let opencc = OpenCC::new();
        let actual_output = opencc.s2t(input, false);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn s2tw_test() {
        let input = "你好，意大利！";
        let expected_output = "你好，意大利！";
        let opencc = OpenCC::new();
        let actual_output = opencc.s2tw(input, false);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn tw2sp_test() {
        let input = "你好，義大利！";
        let expected_output = "你好，意大利！";
        let opencc = OpenCC::new();
        let actual_output = opencc.tw2sp(input, false);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn s2twp_test() {
        let input = "你好，意大利！";
        let expected_output = "你好，義大利！";
        let opencc = OpenCC::new();
        let actual_output = opencc.s2twp(input, false);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn t2s_test() {
        let input = "你好，世界！龍馬精神！";
        let expected_output = "你好，世界！龙马精神！";
        let opencc = OpenCC::new();
        let actual_output = opencc.t2s(input, false);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn tw2s_test() {
        let input = "你好，世界！龍馬精神！";
        let expected_output = "你好，世界！龙马精神！";
        let opencc = OpenCC::new();
        let actual_output = opencc.tw2s(input, false);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn s2t_punct_test() {
        let input = "你好，世界！“龙马精神”！";
        let expected_output = "你好，世界！「龍馬精神」！";
        let opencc = OpenCC::new();
        let actual_output = opencc.s2t(input, true);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn s2t_punct_not_parallel_test() {
        let input = "你好，世界！“龙马精神”！";
        let expected_output = "你好，世界！「龍馬精神」！";
        let mut opencc = OpenCC::new();
        opencc.set_parallel(false);
        let actual_output = opencc.s2t(input, true);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn t2jp_test() {
        let input = "舊字體：廣國，讀賣。";
        let expected_output = "旧字体：広国，読売。";
        let opencc = OpenCC::new();
        let actual_output = opencc.t2jp(input);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn jp2t_test() {
        let input = "広国，読売。";
        let expected_output = "廣國，讀賣。";
        let opencc = OpenCC::new();
        let actual_output = opencc.jp2t(input);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn test_dictionary_from_dicts() {
        let dictionary = dictionary_lib::DictionaryMaxlength::from_dicts().unwrap();
        // Verify that the JSON contains the expected data
        let expected = 16;
        assert_eq!(dictionary.st_phrases.1, expected);
    }

    // Use this to generate "dictionary_maxlength.json" when you edit dicts_ data
    #[test]
    #[ignore]
    fn test_dictionary_from_dicts_then_to_json() {
        let dictionary = dictionary_lib::DictionaryMaxlength::from_dicts().unwrap();
        // Verify that the Dictionary contains the expected data
        let expected = 16;
        assert_eq!(dictionary.st_phrases.1, expected);

        let filename = "dictionary_maxlength.json";
        // Serialize to JSON instead of CBOR
        let json_data = serde_json::to_string(&dictionary).expect("Failed to serialize to JSON");
        fs::write(filename, json_data).expect("Failed to write JSON file");
        let file_contents = fs::read_to_string(filename).unwrap();
        let expected_json = 1351486;
        assert_eq!(file_contents.trim().len(), expected_json);
        // Clean up: Delete the test file
        // fs::remove_file(filename).unwrap();
    }

    #[test]
    #[ignore]
    fn test_dictionary_from_dicts_then_to_cbor() {
        let dictionary = dictionary_lib::DictionaryMaxlength::from_dicts().unwrap();

        // Verify that the Dictionary contains the expected data
        let expected = 16;
        assert_eq!(dictionary.st_phrases.1, expected);

        let filename = "dictionary_maxlength.cbor";

        // Serialize dictionary to CBOR
        let cbor_data = to_vec(&dictionary).expect("Failed to serialize dictionary to CBOR");
        fs::write(filename, &cbor_data).expect("Failed to write CBOR file");

        // Check the expected file size (update this value after first run)
        let expected_cbor_size = 1113003; // Replace with actual size after first run
        let file_size = fs::metadata(filename).unwrap().len() as usize;
        assert_eq!(file_size, expected_cbor_size);

        // Clean up: Uncomment if you want to remove the test file
        // fs::remove_file(filename).unwrap();
    }
    #[test]
    #[ignore]
    fn test_serialize_to_cbor() {
        // Define the filename for testing
        let filename = "dictionary_maxlength.cbor";
        let dictionary = dictionary_lib::DictionaryMaxlength::new().unwrap();
        // Serialize to JSON and write to file
        dictionary.serialize_to_cbor(filename).unwrap();
        // Read the contents of the file
        let file_contents = fs::read_to_string(filename).unwrap();
        // Verify that the JSON contains the expected data
        let expected_json = 1350232;
        assert_eq!(file_contents.trim().len(), expected_json);
        // Clean up: Delete the test file
        fs::remove_file(filename).unwrap();
    }

    #[test]
    fn is_parallel_test() {
        let mut opencc = OpenCC::new();
        assert_eq!(opencc.get_parallel(), true);
        opencc.set_parallel(false);
        assert_eq!(opencc.get_parallel(), false);
    }

    #[test]
    fn last_error_test() {
        OpenCC::set_last_error("Some error here.");
        assert_eq!(OpenCC::get_last_error().unwrap(), "Some error here.");
    }

    #[test]
    fn delimiters_duplicate_test() {
        const DELIMITERS0: &str = "\t\n\r (){}[]<>\"'\\/|-,.?!*:;@#$%^&_+=　，。、；：？！…“”‘’『』「」﹁﹂—－（）《》〈〉～．／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼";
        // println!("DELIMITERS0: {}", DELIMITERS0.chars().count());
        const DELIMITERS2: &str = " \t\n\r!\"#$%&'()*+,-./:;<=>?@[\\]^_{}|~＝、。“”‘’『』「」﹁﹂—－（）《》〈〉？！…／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼　～．，；：";
        // println!("DELIMITERS2: {}", DELIMITERS2.chars().count());
        let mut char_set0 = HashSet::new();
        let mut has_duplicates0 = false;
        for c in DELIMITERS0.chars() {
            if !char_set0.insert(c) {
                println!("Duplicate character found: {}", c);
                has_duplicates0 = true;
            }
        }
        if !has_duplicates0 {
            println!("No duplicate characters found.");
        }
        let mut char_set2 = HashSet::new();
        let mut has_duplicates2 = false;
        for c in DELIMITERS2.chars() {
            if !char_set2.insert(c) {
                println!("Duplicate character found: {}", c);
                has_duplicates2 = true;
            }
        }
        if !has_duplicates2 {
            println!("No duplicate characters found.");
        }
    }

    #[test]
    fn delimiters_diff_test() {
        const DELIMITERS0: &str = "\t\n\r (){}[]<>\"'\\/|-,.?!*:;@#$%^&_+=　，。、；：？！…“”‘’『』「」﹁﹂—－（）《》〈〉～．／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼";
        println!("DELIMITERS0: {}", DELIMITERS0.chars().count());
        const DELIMITERS2: &str = " \t\n\r!\"#$%&'()*+,-./:;<=>?@[\\]^_{}|~＝、。“”‘’『』「」﹁﹂—－（）《》〈〉？！…／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼　～．，；：";
        println!("DELIMITERS2: {}", DELIMITERS2.chars().count());

        let set0: HashSet<_> = DELIMITERS0.chars().collect();
        let set2: HashSet<_> = DELIMITERS2.chars().collect();
        for c in set0.difference(&set2) {
            println!("Missing character in DELIMITERS2: {}", c);
        }
        for c in set2.difference(&set0) {
            println!("Extra character in DELIMITERS2: {}", c);
        }
    }

    #[test]
    fn test_split_ranges_on_newline() {
        use std::ops::Range;

        // Simulate your struct with delimiters
        struct Dummy {
            delimiters: FxHashSet<char>,
        }

        impl Dummy {
            fn get_chars_range(&self, chars: &[char], inclusive: bool) -> Vec<Range<usize>> {
                let mut ranges = Vec::new();
                let mut start = 0;

                for (i, ch) in chars.iter().enumerate() {
                    if self.delimiters.contains(ch) {
                        if inclusive {
                            if i + 1 > start {
                                ranges.push(start..i + 1);
                            }
                        } else {
                            if i > start {
                                ranges.push(start..i);
                            }
                            ranges.push(i..i + 1);
                        }
                        start = i + 1;
                    }
                }

                if start < chars.len() {
                    ranges.push(start..chars.len());
                }

                ranges
            }
        }

        use rustc_hash::FxHashSet;

        let dummy = Dummy {
            delimiters: FxHashSet::from_iter("\n".chars()),
        };

        let base = "你好世界\n";
        let text = base.repeat(3); // "你好世界\n你好世界\n你好世界\n"
        let chars: Vec<char> = text.chars().collect();

        let ranges = dummy.get_chars_range(&chars, true);

        // Print result
        for (i, range) in ranges.iter().enumerate() {
            let segment: String = chars[range.clone()].iter().collect();
            println!("Segment {}: [{}..{}] = {:?}", i, range.start, range.end, segment);
        }

        // Optional assertion: should have 3 segments ending with '\n'
        assert_eq!(ranges.len(), 3);
        for range in &ranges {
            assert_eq!(chars[range.end - 1], '\n');
        }
    }

}
