import ctypes
from ctypes import wintypes

hid = ctypes.windll.hid
kernel32 = ctypes.windll.kernel32
setupapi = ctypes.windll.setupapi

class GUID(ctypes.Structure):
    _fields_ = [
        ("Data1", wintypes.DWORD),
        ("Data2", wintypes.WORD),
        ("Data3", wintypes.WORD),
        ("Data4", wintypes.BYTE * 8)
    ]

class SP_DEVICE_INTERFACE_DATA(ctypes.Structure):
    _fields_ = [
        ("cbSize", wintypes.DWORD),
        ("InterfaceClassGuid", GUID),
        ("Flags", wintypes.DWORD),
        ("Reserved", ctypes.c_size_t)
    ]

class HIDD_ATTRIBUTES(ctypes.Structure):
    _fields_ = [
        ("Size", wintypes.DWORD),
        ("VendorID", wintypes.WORD),
        ("ProductID", wintypes.WORD),
        ("VersionNumber", wintypes.WORD)
    ]

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

hid_guid = GUID()
hid.HidD_GetHidGuid(ctypes.byref(hid_guid))

DIGCF_PRESENT = 2
DIGCF_DEVICEINTERFACE = 16

dev_info = setupapi.SetupDiGetClassDevsW(
    ctypes.byref(hid_guid), None, None, DIGCF_PRESENT | DIGCF_DEVICEINTERFACE
)

device_interface_data = SP_DEVICE_INTERFACE_DATA()
device_interface_data.cbSize = ctypes.sizeof(SP_DEVICE_INTERFACE_DATA)

index = 0
found_devices = []

while setupapi.SetupDiEnumDeviceInterfaces(dev_info, None, ctypes.byref(hid_guid), index, ctypes.byref(device_interface_data)):
    required_size = wintypes.DWORD()
    setupapi.SetupDiGetDeviceInterfaceDetailW(dev_info, ctypes.byref(device_interface_data), None, 0, ctypes.byref(required_size), None)
    
    detail_size = required_size.value
    buf = (ctypes.c_byte * detail_size)()
    # cbSize at start of SP_DEVICE_INTERFACE_DETAIL_DATA_W: 8 on x64
    struct_cbSize = 8 if ctypes.sizeof(ctypes.c_void_p) == 8 else (4 + ctypes.sizeof(ctypes.c_wchar))
    ctypes.cast(ctypes.byref(buf), ctypes.POINTER(wintypes.DWORD))[0] = struct_cbSize
    
    if setupapi.SetupDiGetDeviceInterfaceDetailW(dev_info, ctypes.byref(device_interface_data), ctypes.byref(buf), detail_size, None, None):
        device_path = ctypes.wstring_at(ctypes.addressof(buf) + 4)
        if "0b05" in device_path.lower():
            # Open device to query attributes
            h = kernel32.CreateFileW(
                device_path, 0, 1 | 2, None, 3, 0, None
            )
            if h != -1 and h != 0:
                attr = HIDD_ATTRIBUTES()
                attr.Size = ctypes.sizeof(HIDD_ATTRIBUTES)
                hid.HidD_GetAttributes(h, ctypes.byref(attr))
                
                preparsed = ctypes.c_void_p()
                caps = HIDP_CAPS()
                if hid.HidD_GetPreparsedData(h, ctypes.byref(preparsed)):
                    hid.HidP_GetCaps(preparsed, ctypes.byref(caps))
                    hid.HidD_FreePreparsedData(preparsed)
                
                found_devices.append({
                    "path": device_path,
                    "vid": hex(attr.VendorID),
                    "pid": hex(attr.ProductID),
                    "ver": hex(attr.VersionNumber),
                    "usage_page": hex(caps.UsagePage),
                    "usage": hex(caps.Usage),
                    "feature_len": caps.FeatureReportByteLength,
                    "input_len": caps.InputReportByteLength,
                    "output_len": caps.OutputReportByteLength,
                })
                kernel32.CloseHandle(h)
            else:
                found_devices.append({
                    "path": device_path,
                    "error": f"CreateFile error {kernel32.GetLastError()}"
                })
    index += 1

setupapi.SetupDiDestroyDeviceInfoList(dev_info)

print(f"Found {len(found_devices)} ASUS HID devices:")
for d in found_devices:
    print(d)
