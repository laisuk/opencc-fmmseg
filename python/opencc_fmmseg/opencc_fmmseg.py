"""OpenCC-FMMSEG Python API backed by the packaged C ABI library."""
import ctypes as C
from pathlib import Path
from typing import Dict, List, Optional, Tuple, TypedDict
from ._native import load_native_library

_CustomDictPair = Tuple[str, str]
_PreparedSpec = Tuple[int, int, List[_CustomDictPair]]

class _CustomDictSpecRequired(TypedDict):
    slot: str
    pairs: List[_CustomDictPair]


class CustomDictSpec(_CustomDictSpecRequired, total=False):
    """In-memory `OpenCC` replacements for one dictionary slot.

    Required keys are ``slot`` (see ``OpenCC.available_slots()``) and ``pairs``
    (a list of source/target string tuples). Optional ``mode`` is ``"append"``
    by default, or ``"override"`` to clear that slot before inserting pairs.
    This defines conversion mappings.
    """
    mode: str


class _CustomDictFileSpecRequired(TypedDict):
    slot: str
    files: List[str]


class CustomDictFileSpec(_CustomDictFileSpecRequired, total=False):
    """UTF-8 `OpenCC` mapping files to load into one dictionary slot.

    Required keys are ``slot`` and ``files`` (a list of paths). Each file uses
    ``source<TAB>target`` lines, with an actual tab between the fields.
    Optional ``mode`` is ``"append"`` (default) or ``"override"``; override
    replaces the selected slot with the combined mappings from the files.

    """
    mode: str


class _Pair(C.Structure):
    _fields_ = [("source", C.c_char_p), ("target", C.c_char_p)]


class _Spec(C.Structure):
    _fields_ = [("slot", C.c_uint32), ("mode", C.c_uint32),
                ("pairs", C.POINTER(_Pair)), ("pair_count", C.c_size_t)]


def _utf8(value: str) -> bytes:
    if not isinstance(value, str):
        raise TypeError("Expected a string")
    if "\0" in value:
        raise ValueError("Embedded NUL characters are not supported by the C API")
    return value.encode("utf-8")


_SLOTS = 'STCharacters STPhrases TSCharacters TSPhrases TWPhrases TWPhrasesRev HKPhrases HKPhrasesRev TWVariants TWVariantsPhrases TWVariantsRev TWVariantsRevPhrases HKVariants HKVariantsPhrases HKVariantsRev HKVariantsRevPhrases JPSCharacters JPSCharactersRev JPSPhrases STPunctuations TSPunctuations SealCharacters SealCharactersRev SealVariants SealVariantsRev'.split()

class OpenCC:
    """Convert Chinese text with optional `OpenCC` mapping overlays.

    Synchronize dictionary rebuilds, option changes, and close with other calls.
    """
    VALID_CONFIGS = ('hk2s', 'hk2sp', 'hk2t', 'hk2tp', 'jp2t', 's2hk', 's2hkp', 's2seal', 's2t', 's2tw', 's2twp', 'seal2s', 'seal2t', 't2hk', 't2hkp', 't2jp', 't2s', 't2seal', 't2tw', 't2twp', 'tw2s', 'tw2sp', 'tw2t', 'tw2tp')

    def __init__(self, config: Optional[str] = "s2t") -> None:
        self._opencc_instance = None
        self._config = self.canonicalise_config(config) if self.is_valid_config(config) else "s2t"
        self._specs: List[_PreparedSpec] = []
        self.lib = load_native_library()
        p, s, b, n = C.c_void_p, C.c_char_p, C.c_bool, C.c_size_t
        signatures = {
            "new_custom": (p, [C.POINTER(_Spec), n]),
            "delete": (None, [p]), "string_free": (None, [p]),
            "last_error": (p, []), "error_free": (None, [p]),
            "clear_last_error": (None, []),
            "convert": (p, [p, s, s, b]), "zho_check": (C.c_int, [p, s]),
            "get_parallel": (b, [p]), "set_parallel": (None, [p, b]),
        }
        for name in ("normalize_compat", "normalize_compat_extended", "normalize_unicode_compat"):
            if hasattr(self.lib, "opencc_" + name):
                signatures[name] = (p, [p, s])
        if hasattr(self.lib, "opencc_detofu"):
            signatures["detofu"] = (p, [p, s, C.c_uint32])
        for name, (result, args) in signatures.items():
            fn = getattr(self.lib, "opencc_" + name)
            fn.restype, fn.argtypes = result, args
        self._opencc_instance = self.lib.opencc_new_custom(None, 0)
        if not self._opencc_instance:
            raise ValueError(self._last_error() or "Failed to create OpenCC converter")

    def _require_instance(self):
        if not self._opencc_instance:
            raise RuntimeError("OpenCC converter is closed")
        return self._opencc_instance

    def _last_error(self) -> Optional[str]:
        ptr = self.lib.opencc_last_error()
        if not ptr:
            return None
        try:
            message = C.string_at(ptr).decode("utf-8")
            return None if message == "No error" else message
        finally:
            self.lib.opencc_error_free(ptr)

    def _string(self, name: str, *args: object) -> str:
        self._require_instance()
        fn = getattr(self.lib, "opencc_" + name, None)
        if fn is None:
            raise RuntimeError(f"Native library lacks opencc_{name}; update the native library")
        self.lib.opencc_clear_last_error()
        ptr = fn(*args)
        try:
            error = self._last_error()
            if error or not ptr:
                raise ValueError(error or "Native OpenCC operation failed")
            return C.string_at(ptr).decode("utf-8")
        finally:
            if ptr:
                self.lib.opencc_string_free(ptr)

    def close(self) -> None:
        instance = getattr(self, "_opencc_instance", None)
        if instance:
            self._opencc_instance = None
            self.lib.opencc_delete(instance)

    def __del__(self):
        self.close()

    def __enter__(self):
        self._require_instance()
        return self

    def __exit__(self, *args):
        self.close()

    def get_config(self) -> str:
        return self.config

    @property
    def parallel(self) -> bool:
        return self.lib.opencc_get_parallel(self._require_instance())

    @parallel.setter
    def parallel(self, value: bool) -> None:
        self.lib.opencc_set_parallel(self._require_instance(), value)

    def set_config(self, config: str) -> None:
        self.config = config

    def zho_check(self, text: str) -> int:
        result = self.lib.opencc_zho_check(self._require_instance(), _utf8(text))
        if result < 0:
            raise ValueError(self._last_error() or "Chinese detection failed")
        return result

    def _rebuild(self, specs: List[_PreparedSpec]) -> None:
        parallel = self.parallel
        arrays = [(_Pair * len(pairs))(*[_Pair(_utf8(a), _utf8(b)) for a, b in pairs])
                  for _, _, pairs in specs]
        ffi_specs = (_Spec * len(specs))(*[
            _Spec(slot, mode, pairs, len(pairs))
            for (slot, mode, _), pairs in zip(specs, arrays)])
        instance = self.lib.opencc_new_custom(ffi_specs, len(specs))
        if not instance:
            raise ValueError(self._last_error() or "Dictionary construction failed")
        self.lib.opencc_set_parallel(instance, parallel)
        old, self._opencc_instance = self._opencc_instance, instance
        self._specs = specs
        self.lib.opencc_delete(old)

    @staticmethod
    def supported_configs() -> List[str]:
        return list(OpenCC.VALID_CONFIGS)


    @staticmethod
    def available_slots() -> List[str]:
        return list(_SLOTS)


    @staticmethod
    def canonicalise_config(config: str) -> str:
        if not isinstance(config, str):
            raise ValueError(f"Unsupported config: {config}")
        name = config.lower()
        if name not in OpenCC.VALID_CONFIGS:
            raise ValueError(f"Unsupported config: {config}")
        return name


    @staticmethod
    def is_valid_config(config: object) -> bool:
        return isinstance(config, str) and config.lower() in OpenCC.VALID_CONFIGS


    @property
    def config(self) -> str:
        return self._config


    @config.setter
    def config(self, value: str) -> None:
        self._config = self.canonicalise_config(value)


    def apply_config(self, config: str) -> None:
        self.config = config


    def convert(self, text: str, punctuation: bool = False) -> str:
        return self._string("convert", self._require_instance(), _utf8(text), _utf8(self.config), punctuation)


    def normalize_compat(self, text: str) -> str:
        return self._string("normalize_compat", self._require_instance(), _utf8(text))


    def normalize_compat_extended(self, text: str) -> str:
        return self._string("normalize_compat_extended", self._require_instance(), _utf8(text))


    def normalize_unicode_compat(self, text: str) -> str:
        """Apply curated Unicode mappings, preserving CJK compatibility ideographs.

        Requires a native library exporting opencc_normalize_unicode_compat.
        """
        if not hasattr(self.lib, "opencc_normalize_unicode_compat"):
            raise RuntimeError(
                "The native library lacks opencc_normalize_unicode_compat; "
                "rebuild or update the bundled native library."
            )
        return self._string("normalize_unicode_compat", self._require_instance(), _utf8(text))


    @staticmethod
    def _level(level: str) -> int:
        key = level.strip().lower()
        if key == "all":
            return 0
        if key.startswith("ext-"):
            key = key[4:]
        elif key.startswith("ext"):
            key = key[3:]
        if key not in list("bcdefghi"):
            raise ValueError(f"Unsupported DeTofu level: {level}")
        return "bcdefghi".index(key)


    def detofu(self, text: str, level: str = "ExtB") -> str:
        return self._string("detofu", self._require_instance(), _utf8(text), self._level(level))


    def detofu_with_custom_file(self, text: str, level: str, path: str) -> str:
        _utf8(text)
        # The C ABI exposes built-ins only. Match Rust's single-pass custom
        # precedence without feeding custom replacement characters back into it.
        threshold = self._level(level)
        mappings: Dict[str, str] = {}
        for number, line in enumerate(Path(path).read_text(encoding="utf-8").splitlines(), 1):
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = [part.strip() for part in line.split("\t")]
            if len(parts) < 3 or not parts[0] or not parts[1]:
                raise ValueError(f"{path}:{number}: expected character<TAB>fallback<TAB>extension")
            if self._level(parts[2]) >= threshold:
                mappings[parts[0][0]] = parts[1][0]
        chunks: List[str] = []
        start = 0
        for i, char in enumerate(text):
            if char in mappings:
                chunks.extend((self.detofu(text[start:i], level), mappings[char]))
                start = i + 1
        chunks.append(self.detofu(text[start:], level))
        return "".join(chunks)


    def load_custom_dicts(self, specs: List[CustomDictSpec]) -> None:
        prepared: List[_PreparedSpec] = []
        aliases = {"JPShinjitaiCharacters": "JPSCharacters", "JPShinjitaiCharactersRev": "JPSCharactersRev",
                   "JPShinjitaiPhrases": "JPSPhrases"}
        for spec in specs:
            slot = spec["slot"].strip()
            if slot.lower().endswith(".txt"):
                slot = slot[:-4]
            names: Dict[str, str] = {name.lower(): name for name in _SLOTS}
            names.update({name.lower(): value for name, value in aliases.items()})
            slot = names.get(slot.lower(), slot)
            mode = spec.get("mode")
            mode = ("append" if mode is None else mode).strip().lower()
            if slot not in _SLOTS or mode not in ("append", "override"):
                raise ValueError(f"Invalid dictionary slot or mode: {slot}:{mode}")
            pairs = list(spec["pairs"])
            for source, target in pairs:
                _utf8(source), _utf8(target)
            prepared.append((_SLOTS.index(slot) + 1, 1 if mode == "append" else 2, pairs))
        if prepared:
            self._rebuild(self._specs + prepared)


    def load_custom_dict_files(self, specs: List[CustomDictFileSpec]) -> None:
        prepared: List[CustomDictSpec] = []
        for spec in specs:
            pairs = []
            for path in spec["files"]:
                for number, line in enumerate(Path(path).read_text(encoding="utf-8-sig").splitlines(), 1):
                    line = line.rstrip()
                    if not line or line.lstrip().startswith("#"):
                        continue
                    source, sep, target = line.partition("\t")
                    targets = target.split()
                    if not sep or not source or not targets:
                        raise ValueError(f"{path}:{number}: expected source<TAB>target")
                    pairs.append((source, targets[0]))
            prepared.append({"slot": spec["slot"], "mode": spec.get("mode", "append"), "pairs": pairs})
        self.load_custom_dicts(prepared)


    @classmethod
    def from_dicts(cls, config: str = "s2t", specs: Optional[List[CustomDictSpec]] = None) -> "OpenCC":
        cc = cls(config)
        cc.load_custom_dicts(specs or [])
        return cc


    @classmethod
    def from_dict_files(cls, config: str = "s2t", specs: Optional[List[CustomDictFileSpec]] = None) -> "OpenCC":
        cc = cls(config)
        cc.load_custom_dict_files(specs or [])
        return cc
