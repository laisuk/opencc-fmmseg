import contextlib
import io
from pathlib import Path
import tempfile
import unittest
import zipfile

from opencc_fmmseg import OpenCC
from opencc_fmmseg.__main__ import build_parser, main
from opencc_fmmseg.office_helper import convert_office_doc


class PortTests(unittest.TestCase):
    def test_native_conversion_and_lifetime(self):
        with OpenCC("T2S") as cc:
            self.assertEqual(cc.convert("漢語"), "汉语")
            self.assertEqual(cc.normalize_compat("神"), "神")
            with self.assertRaises(ValueError):
                cc.convert("汉\0语")
            with self.assertRaises(ValueError):
                cc.config = "invalid"
        cc.close()
        with self.assertRaises(RuntimeError):
            cc.convert("汉语")

    def test_custom_dict_rebuild_and_file(self):
        with OpenCC() as cc, tempfile.TemporaryDirectory() as tmp:
            cc.parallel = False
            cc.load_custom_dicts([{"slot": "STPhrases", "pairs": [("测试词", "自訂")]}])
            self.assertEqual(cc.convert("测试词"), "自訂")
            mapping = Path(tmp) / "mapping.txt"
            mapping.write_text("另一个词\t第二詞\n", encoding="utf-8-sig")
            cc.load_custom_dict_files([{"slot": "STPhrases", "files": [str(mapping)]}])
            self.assertEqual(cc.convert("测试词另一个词"), "自訂第二詞")
            self.assertFalse(cc.parallel)
            with self.assertRaises(ValueError):
                cc.load_custom_dicts([{"slot": "bad", "pairs": []}])
            self.assertEqual(cc.convert("测试词"), "自訂")

    def test_cli_excludes_jieba(self):
        help_text = build_parser().format_help()
        self.assertNotIn("segment", help_text)
        self.assertNotIn("Jieba", help_text)
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            build_parser().parse_args(["convert", "-U", "user.txt"])

    def test_native_error_is_raised_and_next_call_recovers(self):
        with OpenCC() as cc:
            cc._config = "invalid"
            with self.assertRaises(ValueError):
                cc.convert("汉语")
            cc.set_config("s2t")
            self.assertEqual(cc.convert("汉语"), "漢語")

    def test_custom_detofu_precedence(self):
        with OpenCC() as cc, tempfile.TemporaryDirectory() as tmp:
            mapping = Path(tmp) / "fallback.tsv"
            mapping.write_text("𠀀\t一\tExtB\n", encoding="utf-8")
            self.assertEqual(cc.detofu_with_custom_file("𠀀", "ExtB", str(mapping)), "一")
            self.assertEqual(cc.detofu_with_custom_file("𠀀", "ExtC", str(mapping)), cc.detofu("𠀀", "ExtC"))

    def test_office_cli_honors_config_and_fonts(self):
        with tempfile.TemporaryDirectory() as tmp:
            source, target = Path(tmp) / "input.docx", Path(tmp) / "output.docx"
            with zipfile.ZipFile(source, "w") as archive:
                archive.writestr("word/document.xml", '<w:rFonts w:eastAsia="漢體"/><w:t>漢語</w:t>')
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(main(["office", "-c", "t2s", "-i", str(source), "-o", str(target), "-k"]), 0)
            with zipfile.ZipFile(target) as archive:
                text = archive.read("word/document.xml").decode()
                self.assertIn("汉语", text)
                self.assertIn("漢體", text)

    def test_xlsx_inline_text_preserves_formula(self):
        with tempfile.TemporaryDirectory() as tmp:
            source, target = Path(tmp) / "in.xlsx", Path(tmp) / "out.xlsx"
            xml = '<worksheet><c t="inlineStr"><is><t>汉语</t></is></c><c><f>汉语</f><v>1</v></c></worksheet>'
            with zipfile.ZipFile(source, "w") as archive:
                archive.writestr("xl/worksheets/sheet1.xml", xml)
            ok, message = convert_office_doc(str(source), str(target), "xlsx", lambda s: s.replace("汉语", "漢語"))
            self.assertTrue(ok, message)
            with zipfile.ZipFile(target) as archive:
                text = archive.read("xl/worksheets/sheet1.xml").decode()
                self.assertIn("<t>漢語</t>", text)
                self.assertIn("<f>汉语</f>", text)

    def test_failed_conversion_preserves_output_and_blocks_traversal(self):
        with tempfile.TemporaryDirectory() as tmp:
            source, target = Path(tmp) / "in.docx", Path(tmp) / "out.docx"
            target.write_bytes(b"existing")
            with zipfile.ZipFile(source, "w") as archive:
                archive.writestr("word/document.xml", "text")

            def fail(text):
                raise ValueError("conversion failed")

            self.assertFalse(convert_office_doc(str(source), str(target), "docx", fail)[0])
            self.assertEqual(target.read_bytes(), b"existing")
            with zipfile.ZipFile(source, "w") as archive:
                archive.writestr("../escape.xml", "text")
            self.assertFalse(convert_office_doc(str(source), str(target), "docx", str)[0])
            self.assertFalse((Path(tmp) / "escape.xml").exists())

    def test_epub_mimetype_first_and_uncompressed(self):
        with tempfile.TemporaryDirectory() as tmp:
            source, target = Path(tmp) / "in.epub", Path(tmp) / "out.epub"
            with zipfile.ZipFile(source, "w") as archive:
                archive.writestr("mimetype", "application/epub+zip")
                archive.writestr("content.xhtml", "汉语")
            ok, message = convert_office_doc(str(source), str(target), "epub", lambda s: s.replace("汉语", "漢語"))
            self.assertTrue(ok, message)
            with zipfile.ZipFile(target) as archive:
                self.assertEqual(archive.infolist()[0].filename, "mimetype")
                self.assertEqual(archive.infolist()[0].compress_type, zipfile.ZIP_STORED)
                self.assertEqual(archive.read("content.xhtml").decode(), "漢語")


if __name__ == "__main__":
    unittest.main()
