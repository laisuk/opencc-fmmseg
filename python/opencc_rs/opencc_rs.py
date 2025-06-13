import ctypes
import os
import platform
import sys

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

        self.opencc_instance = self.lib.opencc_new()  # Create the opencc object in the constructor
        if self.opencc_instance is None:
            print("Warning: Failed to initialize OpenCC C instance. Operations may not work as expected.",
                  file=sys.stderr)

    def __del__(self):
        # Free the C instance when the Python object is garbage collected
        if hasattr(self, 'opencc_instance') and self.opencc_instance:
            if hasattr(self, 'lib') and hasattr(self.lib, 'opencc_free'):
                self.lib.opencc_delete(self.opencc_instance)
            self.opencc_instance = None  # Mark as freed  # Free the opencc object in the destructor

    def convert(self, text, punctuation=False):
        if not hasattr(self, 'opencc_instance') or self.opencc_instance is None:
            print("Error: OpenCC instance not available for convert.", file=sys.stderr)
            return text
        input_bytes = text.encode('utf-8')
        config_bytes = self.config.encode('utf-8')
        result = self.lib.opencc_convert(self.opencc_instance, input_bytes, config_bytes, punctuation)
        py_result = ctypes.string_at(result).decode('utf-8')
        self.lib.opencc_string_free(result)
        return py_result

    def zho_check(self, text):
        if not hasattr(self, 'opencc_instance') or self.opencc_instance is None:
            print("Error: OpenCC instance not available for zho_check.", file=sys.stderr)
            return -1  # Indicate error, as the function is expected to return an int
        code = self.lib.opencc_zho_check(self.opencc_instance, text.encode('utf-8'))
        return code
