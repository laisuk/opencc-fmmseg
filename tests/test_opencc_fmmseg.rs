use opencc_fmmseg::OpenCC;

#[cfg(test)]
mod tests {
    use opencc_fmmseg::{format_thousand, zho_check};

    use super::*;

    #[test]
    fn zho_check_test() {
        let input = "你好，世界！龙马精神！";
        let expected_output = 2;
        let actual_output = zho_check(input);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn zho_check_test_2() {
        let input = "蟹者之王，應該是大閘蟹。";
        let expected_output = 1;
        let actual_output = zho_check(input);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn s2t_test() {
        let input = "你好，世界！龙马精神！";
        let expected_output = "你好，世界！龍馬精神！";
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
    fn format_thousand_test() {
        let input = 1234567890;
        let expected_output = "1,234,567,890";
        let actual_output = format_thousand(input);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn segment_replace_test() {
        let input = "你好，世界！龙马精神！";
        let expected_output = "你好，世界！龍馬精神！".to_string();
        let opencc = OpenCC::new();
        let actual_output = OpenCC::segment_replace(input, &[&opencc.dictionary.st_characters]);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn segment_replace_test_2() {
        let input = "你好，世界！龙马精神，富贵荣华！";
        let expected_output = "你好，世界！龍馬精神，富貴榮華！".to_string();
        let opencc = OpenCC::new();
        let combined_dict = [
            &opencc.dictionary.st_phrases,
            &opencc.dictionary.st_characters,
        ];

        let actual_output = OpenCC::segment_replace(input, &combined_dict);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn segment_replace_test_3() {
        let input = "你好，世界！龙马精神，富贵荣华！";
        let expected_output = "你好，世界！龍馬精神，富貴榮華！".to_string();
        let opencc = OpenCC::new();
        let dict_refs = [
            &opencc.dictionary.st_phrases,
            &opencc.dictionary.st_characters,
        ];
        let actual_output = OpenCC::segment_replace(input, &dict_refs);
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
    fn dict_data_test() {
        let input = "広国，読売。";
        let expected_output = "廣國，讀賣。";
        let opencc = OpenCC::new();
        let actual_output = opencc.jp2t(input);
        for dict in [
            &opencc.dictionary.st_characters.0,           // 1
            &opencc.dictionary.st_phrases.0,              // 16
            &opencc.dictionary.ts_characters.0,           // 1
            &opencc.dictionary.ts_phrases.0,              // 14
            &opencc.dictionary.tw_phrases.0,              // 10
            &opencc.dictionary.tw_phrases_rev.0,          // 10
            &opencc.dictionary.tw_variants.0,             // 1
            &opencc.dictionary.tw_variants_rev.0,         // 1
            &opencc.dictionary.tw_variants_rev_phrases.0, // 4
            &opencc.dictionary.hk_variants.0,             // 1
            &opencc.dictionary.hk_variants_rev.0,         // 1
            &opencc.dictionary.hk_variants_rev_phrases.0, // 5
            &opencc.dictionary.jps_characters.0,          // 1
            &opencc.dictionary.jps_phrases.0,             // 4
            &opencc.dictionary.jp_variants.0,             // 1
            &opencc.dictionary.jp_variants_rev.0,         // 1
        ] {
            let max_word_length = dict
                .keys()
                .map(|word| word.chars().count())
                .max()
                .unwrap_or(1);
            println!("{:?}\n{}", dict.iter().next().unwrap(), max_word_length)
        }

        let max_lengths = [
            &opencc.dictionary.st_characters.1,           // 1
            &opencc.dictionary.st_phrases.1,              // 16
            &opencc.dictionary.ts_characters.1,           // 1
            &opencc.dictionary.ts_phrases.1,              // 14
            &opencc.dictionary.tw_phrases.1,              // 10
            &opencc.dictionary.tw_phrases_rev.1,          // 10
            &opencc.dictionary.tw_variants.1,             // 1
            &opencc.dictionary.tw_variants_rev.1,         // 1
            &opencc.dictionary.tw_variants_rev_phrases.1, // 4
            &opencc.dictionary.hk_variants.1,             // 1
            &opencc.dictionary.hk_variants_rev.1,         // 1
            &opencc.dictionary.hk_variants_rev_phrases.1, // 5
            &opencc.dictionary.jps_characters.1,          // 1
            &opencc.dictionary.jps_phrases.1,             // 4
            &opencc.dictionary.jp_variants.1,             // 1
            &opencc.dictionary.jp_variants_rev.1,         // 1
        ];
        println!("{:?}", max_lengths);
        assert_eq!(actual_output, expected_output);
    }
}
