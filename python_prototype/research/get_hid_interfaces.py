import winreg
import ctypes
from ctypes import wintypes

kernel32 = ctypes.windll.kernel32
hid = ctypes.windll.hid

class HIDP_CAPS(ctypes.Structure):
    _fields_ = [
        ("Usage", wintypes.USHORT),
        ("UsagePage", wintypes.USHORT),
        ("InputReportByteLength", wintypes.USHORT),
        ("OutputReportByteLength", wintypes.USHORT),
        ("FeatureReportByteLength", wintypes.USHORT),
        ("Reserved", wintypes.USHORT * 17),
        ("NumberLinkCollectionNodes", wintypes.USHORT),
        ("NumberInputButtonCaps", wintypes.USHORT),
        ("NumberInputValueCaps", wintypes.USHORT),
        ("NumberInputDataIndices", wintypes.USHORT),
        ("NumberOutputButtonCaps", wintypes.USHORT),
        ("NumberOutputValueCaps", wintypes.USHORT),
        ("NumberOutputDataIndices", wintypes.USHORT),
        ("NumberFeatureButtonCaps", wintypes.USHORT),
        ("NumberFeatureValueCaps", wintypes.USHORT),
        ("NumberFeatureDataIndices", wintypes.USHORT),
    ]

# In Windows, device interface path for HID is \\?\hid#vid_0b05&pid_19b6&col0X#...#{4d1e55b2-f16f-11cf-88cb-001111000030}
# Let's inspect the registry under HKLM\SYSTEM\CurrentControlSet\Services\HidUsb or DeviceClasses
base_key = r"SYSTEM\CurrentControlSet\Control\DeviceClasses\{4d1e55b2-f16f-11cf-88cb-001111000030}"
with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, base_key) as k:
    i = 0
    while True:
        try:
            sub = winreg.EnumKey(k, i)
            i += 1
            if "0b05" in sub.lower() and "19b6" in sub.lower():
                # Form symbolic path: replace ##?# with \\?\
                sym_path = "\\\\?\\" + sub[4:]
                
                # Try opening
                h = kernel32.CreateFileW(
                    sym_path, 0, 1 | 2, None, 3, 0, None
                )
                if h != -1 and h != 0:
                    preparsed = ctypes.c_void_p()
                    caps = HIDP_CAPS()
                    if hid.HidD_GetPreparsedData(h, ctypes.byref(preparsed)):
                        hid.HidP_GetCaps(preparsed, ctypes.byref(caps))
                        hid.HidD_FreePreparsedData(preparsed)
                        print(f"Col Path: {sym_path}")
                        print(f"  UsagePage: 0x{caps.UsagePage:04X}, Usage: 0x{caps.Usage:04X}")
                        print(f"  InputLen: {caps.InputReportByteLength}, OutputLen: {caps.OutputReportByteLength}, FeatureLen: {caps.FeatureReportByteLength}")
                    kernel32.CloseHandle(h)
                else:
                    err = kernel32.GetLastError()
                    print(f"Col Path: {sym_path} (Failed to open, err={err})")
        except OSError:
            break
