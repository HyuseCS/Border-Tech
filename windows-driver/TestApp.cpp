#include <windows.h>
#include <iostream>
#include <string>

#define FILE_DEVICE_LAMPYRIS 0x8001
#define LAMPYRIS_FUNC_AUTHENTICATE 0x801
#define LAMPYRIS_FUNC_PUSH_AUDIO 0x802
#define METHOD_BUFFERED 0
#define FILE_WRITE_ACCESS 2

#define IOCTL_LAMPYRIS_AUTHENTICATE CTL_CODE(FILE_DEVICE_LAMPYRIS, LAMPYRIS_FUNC_AUTHENTICATE, METHOD_BUFFERED, FILE_WRITE_ACCESS)
#define IOCTL_LAMPYRIS_PUSH_AUDIO CTL_CODE(FILE_DEVICE_LAMPYRIS, LAMPYRIS_FUNC_PUSH_AUDIO, METHOD_BUFFERED, FILE_WRITE_ACCESS)

#define LAMPYRIS_MAX_AUDIO_PAYLOAD 4800

#pragma pack(push, 1)
struct LampyrisAuthPayload {
    UCHAR Token[32];
};

struct LampyrisAudioPayload {
    ULONG Length;
    UCHAR Data[LAMPYRIS_MAX_AUDIO_PAYLOAD];
};
#pragma pack(pop)

int main() {
    std::cout << "[*] Lampyris Driver Connection Test" << std::endl;

    // 1. Read SessionToken from Registry
    HKEY hKey;
    LSTATUS status = RegOpenKeyExW(HKEY_LOCAL_MACHINE, L"SOFTWARE\\Lampyris", 0, KEY_READ, &hKey);
    if (status != ERROR_SUCCESS) {
        std::cerr << "[-] Failed to open HKLM\\SOFTWARE\\Lampyris (Error: " << status << ")" << std::endl;
        std::cerr << "    Did the driver initialize successfully?" << std::endl;
        return 1;
    }

    LampyrisAuthPayload authPayload = {0};
    DWORD tokenLen = 32;
    DWORD valType = 0;
    status = RegQueryValueExW(hKey, L"SessionToken", NULL, &valType, authPayload.Token, &tokenLen);
    RegCloseKey(hKey);

    if (status != ERROR_SUCCESS || tokenLen != 32) {
        std::cerr << "[-] Failed to read 32-byte SessionToken from registry (Error: " << status << ")" << std::endl;
        return 1;
    }

    std::cout << "[+] Successfully read SessionToken from registry." << std::endl;

    // 2. Open Device
    std::cout << "[*] Attempting to open \\\\.\\LampyrisMic2 ..." << std::endl;
    HANDLE hDevice = CreateFileW(
        L"\\\\.\\LampyrisMic2",
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        NULL,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        NULL
    );

    if (hDevice == INVALID_HANDLE_VALUE) {
        std::cerr << "[-] Failed to open device! Error Code: " << GetLastError() << std::endl;
        return 1;
    }

    std::cout << "[+] Device opened successfully. Handle: " << hDevice << std::endl;

    // 3. Authenticate
    std::cout << "[*] Sending IOCTL_LAMPYRIS_AUTHENTICATE payload..." << std::endl;
    DWORD bytesReturned = 0;
    BOOL bResult = DeviceIoControl(
        hDevice,
        IOCTL_LAMPYRIS_AUTHENTICATE,
        &authPayload,
        sizeof(authPayload),
        NULL,
        0,
        &bytesReturned,
        NULL
    );

    if (!bResult) {
        std::cerr << "[-] Authentication failed! Error Code: " << GetLastError() << std::endl;
        CloseHandle(hDevice);
        return 1;
    }

    std::cout << "[+] Authentication SUCCESSFUL! The driver accepted our session token." << std::endl;

    // 4. Test Audio Push (Silent payload)
    std::cout << "[*] Pushing 4800 bytes of silence..." << std::endl;
    LampyrisAudioPayload audioPayload = {0};
    audioPayload.Length = LAMPYRIS_MAX_AUDIO_PAYLOAD;
    
    bResult = DeviceIoControl(
        hDevice,
        IOCTL_LAMPYRIS_PUSH_AUDIO,
        &audioPayload,
        sizeof(ULONG) + LAMPYRIS_MAX_AUDIO_PAYLOAD,
        NULL,
        0,
        &bytesReturned,
        NULL
    );

    if (!bResult) {
        std::cerr << "[-] Audio push failed! Error Code: " << GetLastError() << std::endl;
    } else {
        std::cout << "[+] Audio push SUCCESSFUL!" << std::endl;
    }

    CloseHandle(hDevice);
    std::cout << "[*] Connection test complete." << std::endl;
    return 0;
}
