import ctypes
import os
import platform

# Determine the DLL file based on the operating system
if platform.system() == 'Windows':
    DLL_FILE = 'opencc_fmmseg_capi.dll'
elif platform.system() == 'Darwin':
    DLL_FILE = 'libopencc_fmmseg_capi.dylib'
elif platform.system() == 'Linux':
    DLL_FILE = 'libopencc_fmmseg_capi.so'
else:
    raise OSError("Unsupported operating system")


class OpenCC:
    def __init__(self, config=None):
        config_list = [
            "s2t", "t2s", "s2tw", "tw2s", "s2twp", "tw2sp", "s2hk", "hk2s", "t2tw", "tw2t", "t2twp", "tw2t", "tw2tp",
            "t2hk", "hk2t", "t2jp", "jp2t"
        ]
        self.config = config if config in config_list else "s2t"
        # Load the DLL
        dll_path = os.path.join(os.path.dirname(__file__), DLL_FILE)
        self.lib = ctypes.CDLL(dll_path)
        # Define function prototypes
        self.lib.opencc_new.restype = ctypes.c_void_p
        self.lib.opencc_new.argtypes = []
        self.lib.opencc_convert.restype = ctypes.c_void_p
        self.lib.opencc_convert.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_bool]
        self.lib.opencc_zho_check.restype = ctypes.c_int
        self.lib.opencc_zho_check.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
        self.lib.opencc_string_free.argtypes = [ctypes.c_void_p]
        self.lib.opencc_string_free.restype = None
        self.lib.opencc_delete.argtypes = [ctypes.c_void_p]
        self.lib.opencc_delete.restype = None

        self._openccInstance = self.lib.opencc_new()  # Create the opencc object in the constructor
        if self._openccInstance is None:
            raise RuntimeError("Failed to create OpenCC converter")

    def __del__(self):
        if hasattr(self, '_openccInstance') and self._openccInstance is not None:
            self.lib.opencc_delete(self._openccInstance)  # Free the opencc object in the destructor

    def convert(self, text, punctuation=False):
        if not hasattr(self, '_openccInstance') or self._openccInstance is None:
            raise RuntimeError("OpenCC converter not initialized")
        input_bytes = text.encode('utf-8')
        config_bytes = self.config.encode('utf-8')
        result = self.lib.opencc_convert(self._openccInstance, input_bytes, config_bytes, punctuation)
        if result:
            py_result = ctypes.string_at(result).decode('utf-8')
            self.lib.opencc_string_free(result)
            return py_result
        return text  # Or handle the error appropriately

    def zho_check(self, text):
        if not hasattr(self, '_openccInstance') or self._openccInstance is None:
            raise RuntimeError("OpenCC converter not initialized")
        code = self.lib.opencc_zho_check(self._openccInstance, text.encode('utf-8'))
        return code

    def set_config(self, config):
        if not hasattr(self, '_openccInstance') or self._openccInstance is None:
            raise RuntimeError("OpenCC converter not initialized")
        self.config = config

    def get_config(self) -> str:
        if not hasattr(self, '_openccInstance') or self._openccInstance is None:
            raise RuntimeError("OpenCC converter not initialized")
        return self.config
