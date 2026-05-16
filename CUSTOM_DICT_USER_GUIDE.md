# CUSTOM_DICT_USER_GUIDE.md

This guide explains how to extend the `opencc-fmmseg` conversion dictionaries with custom terms while keeping normal
OpenCC behavior and terminology intact.

## Overview

Custom dictionaries let you add or adjust conversion entries used by `opencc-fmmseg` during Chinese text conversion.
They are useful when the built-in OpenCC lexicons are correct in general, but your application needs a few
domain-specific terms, regional preferences, or project-specific overrides.

Common use cases include:

- enterprise terminology
- regional terms
- subtitle corrections
- OCR post-processing
- game localization
- AI terminology
- NLP preprocessing
- personal phrase preferences

The custom dictionary system extends OpenCC lexicons. It does not replace the OpenCC conversion model or introduce a
separate conversion engine. Custom entries are merged into selected OpenCC dictionary slots, then used by the normal
optimized conversion pipeline.

After custom dictionaries are loaded, `DictionaryMaxlength` automatically rebuilds the maximum phrase lengths, starter
indexes, and derived lookup structures needed by the FMM engine. Conversion remains the same high-level operation:
create an `OpenCC` instance, choose a config such as `s2t` or `t2s`, and convert text.

## OpenCC Compatibility

`opencc-fmmseg` uses standard OpenCC-style plaintext dictionary files. No proprietary custom dictionary format is
required, and existing OpenCC dictionary text files remain compatible.

The expected file format is:

```text
source<TAB>target
```

Example:

```text
帕兰蒂尔	柏蘭蒂爾
人工智能	人工智慧
```

Parsing behavior:

- Lines starting with `#` are ignored.
- Empty lines are ignored.
- A UTF-8 BOM is stripped from the first data line when present.
- Trailing line whitespace, including `\r`, is stripped.
- The first whitespace-separated token after the TAB is used as the target.
- A non-comment data line without a TAB separator is treated as malformed.

This matches OpenCC dictionary philosophy: dictionaries are simple source-to-target mappings, grouped by conversion
role.

## Understanding Dictionary Slots

Choosing the correct slot is the most important part of custom dictionary usage.

OpenCC dictionaries are not one global map. They are grouped into slots that serve different conversion directions and
conversion stages. A custom entry only affects conversions that use the slot you selected.

For example, `STPhrases` and `TSPhrases` are different slots:

- `STPhrases` means Simplified to Traditional phrase mappings.
- `TSPhrases` means Traditional to Simplified phrase mappings.

Putting a Simplified-to-Traditional phrase into `TSPhrases` may produce no effect for `s2t`, because `s2t` does not use
that slot for phrase conversion.

| Slot                    | Direction / Role                                 | Intended Usage                                                                                   | Example Pair                                       |
|-------------------------|--------------------------------------------------|--------------------------------------------------------------------------------------------------|----------------------------------------------------|
| `STCharacters`          | Simplified -> Traditional characters             | Single-character Simplified-to-Traditional mappings                                              | `汉 -> 漢`                                           |
| `STPhrases`             | Simplified -> Traditional phrases                | Multi-character phrase and terminology conversion for `s2t`, `s2tw`, `s2hk`, and related configs | `帕兰蒂尔 -> 柏蘭蒂爾`                                     |
| `TSCharacters`          | Traditional -> Simplified characters             | Single-character Traditional-to-Simplified mappings                                              | `漢 -> 汉`                                           |
| `TSPhrases`             | Traditional -> Simplified phrases                | Multi-character phrase and terminology conversion for `t2s`, `tw2s`, `hk2s`, and related configs | `人工智慧 -> 人工智能`                                     |
| `TWPhrases`             | Traditional -> Taiwan phrases                    | Taiwan phrase preferences layered onto Traditional output                                        | `滑鼠 -> 滑鼠`                                         |
| `TWPhrasesRev`          | Taiwan -> Traditional reverse phrases            | Reverse phrase normalization from Taiwan-specific wording                                        | `-{Taiwan term}- -> -{Traditional term}-`          |
| `TWVariants`            | Traditional -> Taiwan variants                   | Taiwan regional character or term variants                                                       | `-{Traditional variant}- -> -{Taiwan variant}-`    |
| `TWVariantsRev`         | Taiwan -> Traditional reverse variants           | Reverse conversion from Taiwan variants                                                          | `-{Taiwan variant}- -> -{Traditional variant}-`    |
| `TWVariantsRevPhrases`  | Taiwan -> Traditional reverse phrase variants    | Reverse phrase-level Taiwan variants                                                             | `-{Taiwan phrase}- -> -{Traditional phrase}-`      |
| `HKVariants`            | Traditional -> Hong Kong variants                | Hong Kong regional character or term variants                                                    | `-{Traditional variant}- -> -{Hong Kong variant}-` |
| `HKVariantsRev`         | Hong Kong -> Traditional reverse variants        | Reverse conversion from Hong Kong variants                                                       | `-{Hong Kong variant}- -> -{Traditional variant}-` |
| `HKVariantsRevPhrases`  | Hong Kong -> Traditional reverse phrase variants | Reverse phrase-level Hong Kong variants                                                          | `-{Hong Kong phrase}- -> -{Traditional phrase}-`   |
| `JPShinjitaiCharacters` | Japanese Shinjitai characters                    | Japanese Shinjitai character mappings                                                            | `-{old form}- -> -{new form}-`                     |
| `JPShinjitaiPhrases`    | Japanese Shinjitai phrases                       | Japanese Shinjitai phrase mappings                                                               | `-{old phrase}- -> -{new phrase}-`                 |
| `JPVariants`            | Traditional -> Japanese variants                 | Traditional-to-Japanese variant mappings                                                         | `-{Traditional variant}- -> -{Japanese variant}-`  |
| `JPVariantsRev`         | Japanese -> Traditional reverse variants         | Japanese-to-Traditional reverse mappings                                                         | `-{Japanese variant}- -> -{Traditional variant}-`  |
| `STPunctuations`        | Simplified -> Traditional punctuation            | Punctuation conversion for Simplified-to-Traditional workflows                                   | `“ -> 「`                                           |
| `TSPunctuations`        | Traditional -> Simplified punctuation            | Punctuation conversion for Traditional-to-Simplified workflows                                   | `「 -> “`                                           |

For most terminology customization:

- Use `STPhrases` for Simplified-to-Traditional terms.
- Use `TSPhrases` for Traditional-to-Simplified terms.
- Use `TWVariants` or `TWPhrases` only when you specifically need Taiwan regional behavior.
- Use `HKVariants` only when you specifically need Hong Kong regional behavior.
- Use punctuation slots only for punctuation conversion behavior.

If a custom entry appears to do nothing, first check that it was placed in the slot used by your conversion config.

## Custom Dictionary Merge Modes

Custom entries are merged into a slot with one of two modes.

### Append

`CustomDictMode::Append` inserts only missing keys. If the built-in OpenCC dictionary already has the source key, the
built-in value remains unchanged.

Use append when you want to extend OpenCC safely without changing existing entries.

```rust
use opencc_fmmseg::{
    CustomDictMode, CustomDictSpec, DictSlot, DictionaryMaxlength, OpenCC,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dictionary = DictionaryMaxlength::from_dicts_custom(&[
        CustomDictSpec {
            slot: DictSlot::STPhrases,
            pairs: vec![("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string())],
            mode: CustomDictMode::Append,
        },
    ])?;

    let opencc = OpenCC::from_dictionary(dictionary);
    println!("{}", opencc.convert("帕兰蒂尔", "s2t", false));

    Ok(())
}
```

### Override

`CustomDictMode::Override` inserts missing keys and replaces existing values when keys conflict.

Use override when your project intentionally prefers a different conversion for a term that OpenCC already knows.

```rust
use opencc_fmmseg::{
    CustomDictMode, CustomDictSpec, DictSlot, DictionaryMaxlength, OpenCC,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dictionary = DictionaryMaxlength::from_dicts_custom(&[
        CustomDictSpec {
            slot: DictSlot::STPhrases,
            pairs: vec![("人工智能".to_string(), "人工智慧".to_string())],
            mode: CustomDictMode::Override,
        },
    ])?;

    let opencc = OpenCC::from_dictionary(dictionary);
    println!("{}", opencc.convert("人工智能公司", "s2t", false));

    Ok(())
}
```

## Pair-Based Custom Dictionaries

Use `CustomDictSpec` when you already have dictionary pairs in memory.

This is suitable for:

- embedded applications
- WebAssembly
- database-loaded terms
- generated dictionaries
- tests
- `include_str!()` workflows
- environments where file I/O is unavailable or undesirable

### Single Phrase Override

```rust
use opencc_fmmseg::{
    CustomDictMode, CustomDictSpec, DictSlot, DictionaryMaxlength, OpenCC,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dictionary = DictionaryMaxlength::from_dicts_custom(&[
        CustomDictSpec {
            slot: DictSlot::STPhrases,
            pairs: vec![("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string())],
            mode: CustomDictMode::Override,
        },
    ])?;

    let opencc = OpenCC::from_dictionary(dictionary);
    let output = opencc.convert("帕兰蒂尔是一家AI公司。", "s2t", false);

    println!("{output}");
    Ok(())
}
```

### Multiple Slots

Add entries to each direction separately. `STPhrases` does not automatically imply `TSPhrases`.

```rust
use opencc_fmmseg::{
    CustomDictMode, CustomDictSpec, DictSlot, DictionaryMaxlength, OpenCC,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dictionary = DictionaryMaxlength::from_dicts_custom(&[
        CustomDictSpec {
            slot: DictSlot::STPhrases,
            pairs: vec![("云原生平台".to_string(), "雲原生平台".to_string())],
            mode: CustomDictMode::Append,
        },
        CustomDictSpec {
            slot: DictSlot::TSPhrases,
            pairs: vec![("雲原生平台".to_string(), "云原生平台".to_string())],
            mode: CustomDictMode::Append,
        },
    ])?;

    let opencc = OpenCC::from_dictionary(dictionary);

    println!("{}", opencc.convert("云原生平台", "s2t", false));
    println!("{}", opencc.convert("雲原生平台", "t2s", false));

    Ok(())
}
```

### Runtime-Generated Dictionaries

```rust
use opencc_fmmseg::{
    CustomDictMode, CustomDictSpec, DictSlot, DictionaryMaxlength, OpenCC,
};

fn enterprise_terms() -> Vec<(String, String)> {
    vec![
        ("知识图谱平台".to_string(), "知識圖譜平台".to_string()),
        ("智能体编排".to_string(), "智慧體編排".to_string()),
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dictionary = DictionaryMaxlength::from_dicts_custom(&[
        CustomDictSpec {
            slot: DictSlot::STPhrases,
            pairs: enterprise_terms(),
            mode: CustomDictMode::Append,
        },
    ])?;

    let opencc = OpenCC::from_dictionary(dictionary);
    println!("{}", opencc.convert("知识图谱平台支持智能体编排。", "s2t", false));

    Ok(())
}
```

### Dynamically Generated Pairs

```rust
use opencc_fmmseg::{
    CustomDictMode, CustomDictSpec, DictSlot, DictionaryMaxlength, OpenCC,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let product_names = ["HyperGraph", "ModelOps", "VectorHub"];

    let pairs = product_names
        .iter()
        .map(|name| (name.to_string(), name.to_string()))
        .collect::<Vec<_>>();

    let dictionary = DictionaryMaxlength::from_dicts_custom(&[
        CustomDictSpec {
            slot: DictSlot::STPhrases,
            pairs,
            mode: CustomDictMode::Append,
        },
    ])?;

    let opencc = OpenCC::from_dictionary(dictionary);
    println!("{}", opencc.convert("HyperGraph 与 VectorHub", "s2t", false));

    Ok(())
}
```

## File-Based Custom Dictionaries

Use `CustomDictFilesSpec` when your custom entries live in OpenCC-style plaintext files.

```rust
use opencc_fmmseg::{
    CustomDictFilesSpec, CustomDictMode, DictSlot, DictionaryMaxlength, OpenCC,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dictionary = DictionaryMaxlength::from_dicts_custom_files(&[
        CustomDictFilesSpec {
            slot: DictSlot::STPhrases,
            files: vec!["custom_dicts/company_st_phrases.txt"],
            mode: CustomDictMode::Override,
        },
    ])?;

    let opencc = OpenCC::from_dictionary(dictionary);
    println!("{}", opencc.convert("帕兰蒂尔是一家人工智能公司。", "s2t", false));

    Ok(())
}
```

### Single File

`custom_dicts/company_st_phrases.txt`:

```text
# Simplified -> Traditional enterprise terms
帕兰蒂尔	柏蘭蒂爾
人工智能公司	人工智慧公司
```

Rust:

```rust
use opencc_fmmseg::{
    CustomDictFilesSpec, CustomDictMode, DictSlot, DictionaryMaxlength, OpenCC,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dictionary = DictionaryMaxlength::from_dicts_custom_files(&[
        CustomDictFilesSpec {
            slot: DictSlot::STPhrases,
            files: vec!["custom_dicts/company_st_phrases.txt"],
            mode: CustomDictMode::Override,
        },
    ])?;

    let opencc = OpenCC::from_dictionary(dictionary);
    println!("{}", opencc.convert("人工智能公司", "s2t", false));

    Ok(())
}
```

### Multiple Files

Files are loaded sequentially in the order provided. After all files for a spec are parsed, their pairs are merged into
the target slot using the selected mode.

```rust
use opencc_fmmseg::{
    CustomDictFilesSpec, CustomDictMode, DictSlot, DictionaryMaxlength, OpenCC,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dictionary = DictionaryMaxlength::from_dicts_custom_files(&[
        CustomDictFilesSpec {
            slot: DictSlot::STPhrases,
            files: vec![
                "custom_dicts/brand_terms.txt",
                "custom_dicts/product_terms.txt",
                "custom_dicts/subtitle_fixes.txt",
            ],
            mode: CustomDictMode::Append,
        },
    ])?;

    let opencc = OpenCC::from_dictionary(dictionary);
    println!("{}", opencc.convert("帕兰蒂尔的 HyperGraph 平台", "s2t", false));

    Ok(())
}
```

### Multi-Slot Files

```rust
use opencc_fmmseg::{
    CustomDictFilesSpec, CustomDictMode, DictSlot, DictionaryMaxlength, OpenCC,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dictionary = DictionaryMaxlength::from_dicts_custom_files(&[
        CustomDictFilesSpec {
            slot: DictSlot::STPhrases,
            files: vec!["custom_dicts/s2t_terms.txt"],
            mode: CustomDictMode::Append,
        },
        CustomDictFilesSpec {
            slot: DictSlot::TSPhrases,
            files: vec!["custom_dicts/t2s_terms.txt"],
            mode: CustomDictMode::Append,
        },
        CustomDictFilesSpec {
            slot: DictSlot::TWVariants,
            files: vec!["custom_dicts/tw_variants.txt"],
            mode: CustomDictMode::Override,
        },
    ])?;

    let opencc = OpenCC::from_dictionary(dictionary);
    println!("{}", opencc.convert("云原生平台", "s2t", false));
    println!("{}", opencc.convert("雲原生平台", "t2s", false));

    Ok(())
}
```

## Building an OpenCC Instance

`OpenCC::from_dictionary()` creates an `OpenCC` converter from a prepared `DictionaryMaxlength`.

The usual pattern is:

1. Build or load a `DictionaryMaxlength`.
2. Create an `OpenCC` instance with `OpenCC::from_dictionary()`.
3. Convert text normally.

```rust
use opencc_fmmseg::{
    CustomDictMode, CustomDictSpec, DictSlot, DictionaryMaxlength, OpenCC,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dictionary = DictionaryMaxlength::from_dicts_custom(&[
        CustomDictSpec {
            slot: DictSlot::STPhrases,
            pairs: vec![
                ("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string()),
                ("大语言模型".to_string(), "大型語言模型".to_string()),
            ],
            mode: CustomDictMode::Override,
        },
    ])?;

    let opencc = OpenCC::from_dictionary(dictionary);

    let input = "帕兰蒂尔正在测试大语言模型。";
    let output = opencc.convert(input, "s2t", false);

    println!("{output}");
    Ok(())
}
```

For reverse conversion, add reverse-direction entries to `TSPhrases`:

```rust
use opencc_fmmseg::{
    CustomDictMode, CustomDictSpec, DictSlot, DictionaryMaxlength, OpenCC,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dictionary = DictionaryMaxlength::from_dicts_custom(&[
        CustomDictSpec {
            slot: DictSlot::STPhrases,
            pairs: vec![("大语言模型".to_string(), "大型語言模型".to_string())],
            mode: CustomDictMode::Append,
        },
        CustomDictSpec {
            slot: DictSlot::TSPhrases,
            pairs: vec![("大型語言模型".to_string(), "大语言模型".to_string())],
            mode: CustomDictMode::Append,
        },
    ])?;

    let opencc = OpenCC::from_dictionary(dictionary);

    assert_eq!(opencc.convert("大语言模型", "s2t", false), "大型語言模型");
    assert_eq!(opencc.convert("大型語言模型", "t2s", false), "大语言模型");

    Ok(())
}
```

## Alternate Dictionary Directories

`DictionaryMaxlength::from_dicts_at()` loads the standard OpenCC plaintext dictionary set from an alternate base
directory instead of the default `dicts/`.

Use this when you maintain a custom OpenCC dictionary directory, ship dictionaries outside the crate, or need a portable
deployment layout.

Expected structure:

```text
my_opencc_dicts/
  STCharacters.txt
  STPhrases.txt
  TSCharacters.txt
  TSPhrases.txt
  TWPhrases.txt
  TWPhrasesRev.txt
  TWVariants.txt
  TWVariantsRev.txt
  TWVariantsRevPhrases.txt
  HKVariants.txt
  HKVariantsRev.txt
  HKVariantsRevPhrases.txt
  JPShinjitaiCharacters.txt
  JPShinjitaiPhrases.txt
  JPVariants.txt
  JPVariantsRev.txt
  STPunctuations.txt
  TSPunctuations.txt
```

Example:

```rust
use opencc_fmmseg::{DictionaryMaxlength, OpenCC};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dictionary = DictionaryMaxlength::from_dicts_at("my_opencc_dicts")?;
    let opencc = OpenCC::from_dictionary(dictionary);

    println!("{}", opencc.convert("汉字转换", "s2t", false));

    Ok(())
}
```

## dict-generate Workflow

For production, prefer generating a compact dictionary artifact from plaintext dictionaries.

Recommended workflow:

```text
custom_dicts/*.txt
    |
    v
dict-generate -b custom_dicts -f zstd
    |
    v
dictionary_maxlength.zstd
    |
    v
deploy artifact
```

During development, plaintext files are convenient because they are easy to review and edit. For deployment, generated
Zstd or CBOR artifacts avoid repeated plaintext parsing and provide faster startup.

### Generate Zstd

```bash
dict-generate --base-dir custom_dicts --format zstd --output dictionary_maxlength.zstd
```

Short form:

```bash
dict-generate -b custom_dicts -f zstd -o dictionary_maxlength.zstd
```

### Generate CBOR

```bash
dict-generate -b custom_dicts -f cbor -o dictionary_maxlength.cbor
```

### Generate JSON

JSON is useful for inspection, debugging, or tooling.

```bash
dict-generate -b custom_dicts -f json -o dictionary_maxlength.json --pretty
```

### Alternate Base Directory

The `-b` / `--base-dir` option points to the directory containing OpenCC dictionary `.txt` files.

```bash
dict-generate --base-dir vendor/opencc-dicts --format zstd
```

### Loading Generated Artifacts

Use the standard artifact loaders for generated dictionaries:

```rust
use opencc_fmmseg::{DictionaryMaxlength, OpenCC};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dictionary = DictionaryMaxlength::load_cbor_compressed("dictionary_maxlength.zstd")?;
    let opencc = OpenCC::from_dictionary(dictionary);

    println!("{}", opencc.convert("汉字转换", "s2t", false));

    Ok(())
}
```

```rust
use opencc_fmmseg::{DictionaryMaxlength, OpenCC};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dictionary = DictionaryMaxlength::deserialize_from_cbor("dictionary_maxlength.cbor")?;
    let opencc = OpenCC::from_dictionary(dictionary);

    println!("{}", opencc.convert("汉字转换", "s2t", false));

    Ok(())
}
```

## Performance Notes

Custom dictionary loading happens during dictionary construction only.  
Pair parsing, file parsing, and merge work are not repeated during each conversion.

After construction:

- maximum phrase lengths are rebuilt
- starter indexes are rebuilt
- union caches are prepared by the normal dictionary pipeline
- conversion uses the same optimized lookup path as built-in dictionaries

For most applications, custom pair merging cost is negligible compared with large conversion workloads.  
If startup time matters, generate and deploy a Zstd or CBOR artifact.

## Limitations and Notes

- Dictionaries are immutable after `OpenCC` construction.
- Runtime hot-reload injection is not currently provided.
- Prepare custom dictionaries before constructing `OpenCC`.
- Malformed dictionary lines produce errors.
- Choosing the wrong slot may produce no effect or unexpected conversion behavior.
- `from_zstd_custom()` and `from_cbor_custom()` are intentionally not implemented, keeping the API surface simpler and
  maintenance manageable.

If you need runtime updates, rebuild a new `DictionaryMaxlength` and create a new `OpenCC` instance.

## Best Practices

- Prefer `CustomDictMode::Append` when you only need to add missing terms.
- Use `CustomDictMode::Override` sparingly and document why the built-in behavior is being replaced.
- Keep enterprise, product, regional, subtitle, OCR, and test terms in separate files.
- Keep direction-specific files separate, such as `s2t_terms.txt` and `t2s_terms.txt`.
- Generate Zstd artifacts for production deployments.
- Test custom conversions with representative input.
- Maintain slot discipline: match the slot to the conversion direction and regional behavior you expect.
- Keep custom dictionaries UTF-8 encoded.

## API Reference Summary

| API                                              | Kind     | Purpose                                                                                 |
|--------------------------------------------------|----------|-----------------------------------------------------------------------------------------|
| `CustomDictSpec`                                 | Struct   | Pair-based custom dictionary spec for one `DictSlot`.                                   |
| `CustomDictFilesSpec`                            | Struct   | File-based custom dictionary spec for one `DictSlot`.                                   |
| `CustomDictMode`                                 | Enum     | Selects `Append` or `Override` merge behavior.                                          |
| `DictSlot`                                       | Enum     | Identifies the OpenCC dictionary slot to customize.                                     |
| `DictionaryMaxlength::from_dicts_custom()`       | Function | Loads the default plaintext dictionary directory and applies in-memory custom pairs.    |
| `DictionaryMaxlength::from_dicts_custom_files()` | Function | Loads the default plaintext dictionary directory and applies custom OpenCC-style files. |
| `DictionaryMaxlength::from_dicts_at()`           | Function | Loads a complete OpenCC-style dictionary directory from an alternate base path.         |
| `OpenCC::from_dictionary()`                      | Function | Creates an `OpenCC` converter from a prepared `DictionaryMaxlength`.                    |

## Conclusion

Custom dictionaries in `opencc-fmmseg` are designed to extend OpenCC safely. They let you preserve compatibility with
existing OpenCC dictionary ecosystems while adding project-specific terminology, regional preferences, and deployment
workflows.

Use the slot system carefully, choose append or override deliberately, and generate compact artifacts for production
when startup performance matters.
