import ctypes
import os
import platform
from typing import Tuple


def get_native_info() -> Tuple[str, str]:
    """
    Return (platform_arch, library_filename).

    platform_arch examples:
    - win-x64
    - win-x86
    - win-arm64
    - linux-x64
    - linux-arm64
    - macos-x64
    - macos-arm64
    """
    system = platform.system()
    machine = platform.machine().lower()

    if machine in ("amd64", "x86_64", "x64"):
        arch = "x64"
    elif machine in ("i386", "i686", "x86"):
        arch = "x86"
    elif machine in ("arm64", "aarch64"):
        arch = "arm64"
    else:
        raise OSError(f"Unsupported architecture: {machine}")

    if system == "Windows":
        platform_name = "win"
        lib_file = "opencc_fmmseg_capi.dll"
    elif system == "Darwin":
        platform_name = "macos"
        lib_file = "libopencc_fmmseg_capi.dylib"
    elif system == "Linux":
        platform_name = "linux"
        lib_file = "libopencc_fmmseg_capi.so"
    else:
        raise OSError(f"Unsupported operating system: {system}")

    return f"{platform_name}-{arch}", lib_file


def get_native_path() -> str:
    """
    Return absolute path:
        <package_dir>/native/<platform-arch>/<library_file>
    """
    platform_arch, lib_file = get_native_info()
    base_dir = os.path.dirname(os.path.abspath(__file__))
    return os.path.join(base_dir, "native", platform_arch, lib_file)


def load_native_library() -> ctypes.CDLL:
    """
    Load packaged native library.
    """
    lib_path = get_native_path()

    if not os.path.isfile(lib_path):
        raise FileNotFoundError(
            "Native library not found:\n"
            f"  {lib_path}\n"
            "Expected package layout:\n"
            "  native/<platform-arch>/<library-file>"
        )

    return ctypes.CDLL(lib_path)