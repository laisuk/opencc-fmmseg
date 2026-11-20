# opencc-fmmseg Python API and CLI

The ctypes wrapper uses FMMSEG and the existing native/<platform-arch> libraries.
It has no Jieba dependency. Install from the repository root with `pip install ./python`.

```python
from opencc_fmmseg import OpenCC

with OpenCC("s2twp") as cc:
    print(cc.convert("汉语软件", punctuation=True))
    print(cc.normalize_compat("神"))
    print(cc.detofu("𠀀", "ExtB"))
    cc.load_custom_dicts([
        {"slot": "STPhrases", "pairs": [("自定义词", "自訂詞")]}
    ])
```

Available methods include normalize_compat, normalize_compat_extended,
normalize_unicode_compat, detofu, detofu_with_custom_file, load_custom_dicts,
load_custom_dict_files, from_dicts, from_dict_files, supported_configs, and
available_slots. Mapping specs support append (default) and override modes.
File specs use slot, files, and optional mode; UTF-8 files contain
source<TAB>target lines. The first target is used for alternative targets.
The parallel property controls native parallel conversion.

Names are case-insensitive. Invalid constructor configs fall back to s2t;
config assignment, apply_config, and set_config validate names. Embedded NUL
input is rejected. Native failures raise exceptions. Use close() or a context
manager to release instances. Synchronize dictionary rebuilds, option changes,
and closing with other calls using the instance.

```shell
python -m opencc_fmmseg convert -c t2s -i input.txt -o output.txt
python -m opencc_fmmseg convert -c s2t -n --detofu ExtB -D STPhrases:append:custom.txt
python -m opencc_fmmseg office -c t2s -i input.docx -k
python -m opencc_fmmseg office -c s2twp -i input.epub -F
```

The CLI supports normalization (-n/-E), DeTofu (--detofu, --detofu-file), custom
`OpenCC` mapping files (-D), and text stdin/stdout and encodings. Office formats
are DOCX, XLSX, PPTX, ODT, ODS, ODP, and EPUB. -F converts the default output
filename stem; -k preserves recognized fonts. Input/output aliases are rejected.

The office helper now accepts a callable instead of the old OpenCC-object and
punctuation arguments:

```python
from opencc_fmmseg.office_helper import convert_office_doc

with OpenCC("t2s") as cc:
    success, message = convert_office_doc(
        "input.xlsx", "output.xlsx", "xlsx",
        lambda text: cc.convert(text, punctuation=True), keep_font=True,
    )
```

The newer helper includes XLSX inline strings without converting formulas,
PPTX text-part selection, font protection, EPUB packaging rules, ZIP traversal
checks, and validated temporary output published atomically. Conversion failure
preserves existing output.

Use native libraries built from the current C API for the newer features,
including the four Small Seal configurations. Missing normalization/DeTofu
exports raise a runtime error. This port does not rebuild native binaries.

Run tests from the repository root with PYTHONPATH=python:
`python -m unittest discover -s python/tests`.
