import ctypes
from ._native import load_native_library


class OpenCC:
    """
    Python ctypes wrapper for opencc-fmmseg C API.
    """

    VALID_CONFIGS = {
        "s2t",
        "t2s",
        "s2tw",
        "tw2s",
        "s2twp",
        "tw2sp",
        "s2hk",
        "hk2s",
        "t2tw",
        "tw2t",
        "t2twp",
        "tw2tp",
        "t2hk",
        "hk2t",
        "t2jp",
        "jp2t",
    }

    def __init__(self, config=None):
        self.config = config if config in self.VALID_CONFIGS else "s2t"
        self.lib = load_native_library()
        self._bind_functions()

        self._opencc_instance = self.lib.opencc_new()
        if not self._opencc_instance:
            raise RuntimeError("Failed to create OpenCC converter")

    def _bind_functions(self) -> None:
        """
        Define C function signatures.
        """
        self.lib.opencc_new.restype = ctypes.c_void_p
        self.lib.opencc_new.argtypes = []

        self.lib.opencc_convert.restype = ctypes.c_void_p
        self.lib.opencc_convert.argtypes = [
            ctypes.c_void_p,  # instance
            ctypes.c_char_p,  # input
            ctypes.c_char_p,  # config
            ctypes.c_bool,  # punctuation
        ]

        self.lib.opencc_zho_check.restype = ctypes.c_int
        self.lib.opencc_zho_check.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
        ]

        self.lib.opencc_string_free.restype = None
        self.lib.opencc_string_free.argtypes = [ctypes.c_void_p]

        self.lib.opencc_delete.restype = None
        self.lib.opencc_delete.argtypes = [ctypes.c_void_p]

    def __del__(self):
        instance = getattr(self, "_opencc_instance", None)
        lib = getattr(self, "lib", None)

        if instance and lib is not None:
            try:
                lib.opencc_delete(instance)
            except (AttributeError, OSError, ValueError):
                pass

        self._opencc_instance = None

    def convert(self, text: str, punctuation: bool = False) -> str:
        """
        Convert text using the current OpenCC configuration.
        """
        if not hasattr(self, "_opencc_instance") or not self._opencc_instance:
            raise RuntimeError("OpenCC converter not initialized")

        input_bytes = text.encode("utf-8")
        config_bytes = self.config.encode("utf-8")

        result = self.lib.opencc_convert(
            self._opencc_instance,
            input_bytes,
            config_bytes,
            punctuation,
        )

        if result:
            py_result = ctypes.string_at(result).decode("utf-8")
            self.lib.opencc_string_free(result)
            return py_result

        return text

    def zho_check(self, text: str) -> int:
        """
        Detect whether the text is likely Simplified Chinese, Traditional Chinese, or other.
        """
        if not hasattr(self, "_opencc_instance") or not self._opencc_instance:
            raise RuntimeError("OpenCC converter not initialized")

        return self.lib.opencc_zho_check(
            self._opencc_instance,
            text.encode("utf-8"),
        )

    def set_config(self, config: str) -> None:
        """
        Set the current conversion config.
        """
        if not hasattr(self, "_opencc_instance") or not self._opencc_instance:
            raise RuntimeError("OpenCC converter not initialized")

        if config not in self.VALID_CONFIGS:
            raise ValueError(f"Unsupported config: {config}")

        self.config = config

    def get_config(self) -> str:
        """
        Return the current conversion config.
        """
        if not hasattr(self, "_opencc_instance") or not self._opencc_instance:
            raise RuntimeError("OpenCC converter not initialized")

        return self.config
