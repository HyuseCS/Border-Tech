// wasapi_probe.cpp — find the Lampyris capture endpoint and try to open it the
// same way a browser / Voice Recorder does, printing the exact HRESULT at each
// step. This tells us WHY IAudioClient::Initialize aborts before the driver's
// NewStream is ever called.
//
// Build (in an x64 Developer/WDK command prompt):
//     cl /EHsc /W3 wasapi_probe.cpp ole32.lib
// Run:
//     wasapi_probe.exe
//
// No admin needed. Make sure Windows mic privacy is on (it is).

#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <mmdeviceapi.h>
#include <audioclient.h>
#include <functiondiscoverykeys_devpkey.h>
#include <stdio.h>
#include <string.h>

static const char* HrName(HRESULT hr)
{
    switch ((unsigned)hr) {
    case 0x00000000: return "S_OK";
    case 0x88890001: return "AUDCLNT_E_NOT_INITIALIZED";
    case 0x88890002: return "AUDCLNT_E_ALREADY_INITIALIZED";
    case 0x88890003: return "AUDCLNT_E_WRONG_ENDPOINT_TYPE";
    case 0x88890004: return "AUDCLNT_E_DEVICE_INVALIDATED";
    case 0x88890005: return "AUDCLNT_E_NOT_STOPPED";
    case 0x88890006: return "AUDCLNT_E_BUFFER_TOO_LARGE";
    case 0x88890007: return "AUDCLNT_E_OUT_OF_ORDER";
    case 0x88890008: return "AUDCLNT_E_UNSUPPORTED_FORMAT";
    case 0x88890009: return "AUDCLNT_E_INVALID_SIZE";
    case 0x8889000a: return "AUDCLNT_E_DEVICE_IN_USE";
    case 0x8889000b: return "AUDCLNT_E_BUFFER_OPERATION_PENDING";
    case 0x8889000c: return "AUDCLNT_E_THREAD_NOT_REGISTERED";
    case 0x8889000e: return "AUDCLNT_E_EXCLUSIVE_MODE_NOT_ALLOWED";
    case 0x8889000f: return "AUDCLNT_E_ENDPOINT_CREATE_FAILED";
    case 0x88890010: return "AUDCLNT_E_SERVICE_NOT_RUNNING";
    case 0x88890011: return "AUDCLNT_E_EVENTHANDLE_NOT_EXPECTED";
    case 0x88890012: return "AUDCLNT_E_EXCLUSIVE_MODE_ONLY";
    case 0x88890013: return "AUDCLNT_E_BUFDURATION_PERIOD_NOT_EQUAL";
    case 0x88890014: return "AUDCLNT_E_EVENTHANDLE_NOT_SET";
    case 0x88890015: return "AUDCLNT_E_INCORRECT_BUFFER_SIZE";
    case 0x88890016: return "AUDCLNT_E_BUFFER_SIZE_ERROR";
    case 0x88890017: return "AUDCLNT_E_CPUUSAGE_EXCEEDED";
    case 0x88890018: return "AUDCLNT_E_BUFFER_ERROR";
    case 0x88890019: return "AUDCLNT_E_BUFFER_SIZE_NOT_ALIGNED";
    case 0x88890020: return "AUDCLNT_E_INVALID_DEVICE_PERIOD";
    case 0x88890021: return "AUDCLNT_E_INVALID_STREAM_FLAG";
    case 0x88890022: return "AUDCLNT_E_ENDPOINT_OFFLOAD_NOT_CAPABLE";
    case 0x88890026: return "AUDCLNT_E_RESOURCES_INVALIDATED";
    default: return "(other)";
    }
}

#define STEP(label, call) do { \
    hr = (call); \
    printf("  %-28s -> 0x%08X  %s\n", label, (unsigned)hr, HrName(hr)); \
} while (0)

int wmain()
{
    HRESULT hr = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(hr)) { printf("CoInitializeEx failed 0x%08X\n", (unsigned)hr); return 1; }

    IMMDeviceEnumerator* pEnum = nullptr;
    hr = CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL,
                          __uuidof(IMMDeviceEnumerator), (void**)&pEnum);
    if (FAILED(hr)) { printf("CoCreateInstance(MMDeviceEnumerator) 0x%08X\n", (unsigned)hr); return 1; }

    IMMDeviceCollection* pColl = nullptr;
    hr = pEnum->EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE, &pColl);
    if (FAILED(hr)) { printf("EnumAudioEndpoints 0x%08X\n", (unsigned)hr); return 1; }

    UINT count = 0; pColl->GetCount(&count);
    printf("Active capture endpoints: %u\n", count);

    IMMDevice* pLampyris = nullptr;
    for (UINT i = 0; i < count; ++i) {
        IMMDevice* pDev = nullptr;
        if (FAILED(pColl->Item(i, &pDev))) continue;
        IPropertyStore* pProps = nullptr;
        wchar_t name[256] = L"(unknown)";
        if (SUCCEEDED(pDev->OpenPropertyStore(STGM_READ, &pProps))) {
            PROPVARIANT v; PropVariantInit(&v);
            if (SUCCEEDED(pProps->GetValue(PKEY_Device_FriendlyName, &v)) && v.vt == VT_LPWSTR)
                wcsncpy_s(name, v.pwszVal, _TRUNCATE);
            PropVariantClear(&v);
            pProps->Release();
        }
        wprintf(L"  [%u] %s\n", i, name);
        if (!pLampyris && wcsstr(name, L"Lampyris")) {
            pLampyris = pDev; pLampyris->AddRef();
        }
        pDev->Release();
    }

    if (!pLampyris) { printf("\nNo capture endpoint with 'Lampyris' in the name.\n"); return 2; }
    printf("\nOpening the Lampyris endpoint:\n");

    IAudioClient* pClient = nullptr;
    STEP("Activate(IAudioClient)", pLampyris->Activate(__uuidof(IAudioClient), CLSCTX_ALL, nullptr, (void**)&pClient));
    if (FAILED(hr)) return 3;

    WAVEFORMATEX* pMix = nullptr;
    STEP("GetMixFormat", pClient->GetMixFormat(&pMix));
    if (pMix)
        printf("      mix: %u ch, %lu Hz, %u bit, tag %u\n",
               pMix->nChannels, pMix->nSamplesPerSec, pMix->wBitsPerSample, pMix->wFormatTag);

    if (pMix) {
        WAVEFORMATEX* pClosest = nullptr;
        STEP("IsFormatSupported(SHARED)", pClient->IsFormatSupported(AUDCLNT_SHAREMODE_SHARED, pMix, &pClosest));
        if (pClosest) CoTaskMemFree(pClosest);
    }

    // Attempt 1: plain shared-mode, timer-driven (no event) — the simplest open.
    printf("\n-- Attempt 1: shared, NO event flag --\n");
    STEP("Initialize", pClient->Initialize(AUDCLNT_SHAREMODE_SHARED, 0, 2000000 /*200ms*/, 0, pMix, nullptr));
    if (SUCCEEDED(hr)) {
        UINT32 frames = 0; STEP("GetBufferSize", pClient->GetBufferSize(&frames));
        printf("      buffer frames: %u\n", frames);
        IAudioCaptureClient* pCap = nullptr;
        STEP("GetService(CaptureClient)", pClient->GetService(__uuidof(IAudioCaptureClient), (void**)&pCap));
        STEP("Start", pClient->Start());
        printf("      >>> OPEN SUCCEEDED. Capture stream is live. <<<\n");
        if (pCap) pCap->Release();
        pClient->Stop();
        if (pMix) CoTaskMemFree(pMix);
        return 0;
    }

    // Attempt 1 failed. Re-activate a fresh client and try event-driven mode,
    // which is what the audio engine's capture pump uses (INF opts into it).
    printf("\n-- Attempt 2: shared, WITH event callback flag --\n");
    pClient->Release(); pClient = nullptr;
    STEP("Activate(IAudioClient)#2", pLampyris->Activate(__uuidof(IAudioClient), CLSCTX_ALL, nullptr, (void**)&pClient));
    if (SUCCEEDED(hr)) {
        HANDLE hEv = CreateEventW(nullptr, FALSE, FALSE, nullptr);
        STEP("Initialize(EVENTCALLBACK)", pClient->Initialize(AUDCLNT_SHAREMODE_SHARED,
                 AUDCLNT_STREAMFLAGS_EVENTCALLBACK, 2000000, 0, pMix, nullptr));
        if (SUCCEEDED(hr)) {
            STEP("SetEventHandle", pClient->SetEventHandle(hEv));
            STEP("Start", pClient->Start());
            printf("      >>> EVENT-DRIVEN OPEN SUCCEEDED. <<<\n");
            pClient->Stop();
        }
        if (hEv) CloseHandle(hEv);
    }

    printf("\nBoth attempts failed above -> the HRESULT names the reason.\n");
    if (pMix) CoTaskMemFree(pMix);
    return 4;
}
