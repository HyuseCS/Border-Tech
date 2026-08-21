// wasapi_probe.cpp — probe ALL active capture endpoints (control group + Lampyris)
// and try to open the Lampyris endpoint several ways, printing the exact HRESULT
// at each step:
//   - GetMixFormat + shared-mode Initialize on every endpoint (control vs Lampyris)
//   - Lampyris: shared Initialize with an EXPLICIT 2ch/48k/16 format (GetMixFormat
//     may fail; don't depend on it)
//   - Lampyris: shared + AUDCLNT_STREAMFLAGS_EVENTCALLBACK (how browsers open mics)
//   - Lampyris: EXCLUSIVE-mode Initialize with the explicit format — this bypasses
//     the audio engine's shared pipe / format cache and opens the KS pin directly.
//     If this succeeds, the driver's NewStream fires and the driver side is proven
//     good; the bug is then purely in the shared-mode engine path.
//
// Build (any VS x64 command prompt):
//     cl /EHsc /W3 /MT wasapi_probe.cpp ole32.lib
// Run:
//     wasapi_probe.exe
//
// Run DebugView (kernel capture) at the same time: watch for "NewStream ENTER"
// during the exclusive attempt.

#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <mmdeviceapi.h>
#include <audioclient.h>
#include <mmreg.h>
#include <ks.h>
#include <ksmedia.h>
#include <functiondiscoverykeys_devpkey.h>
#include <stdio.h>
#include <string.h>

static const char* HrName(HRESULT hr)
{
    switch ((unsigned)hr) {
    case 0x00000000: return "S_OK";
    case 0x00000001: return "S_FALSE (close match returned)";
    case 0x80004003: return "E_POINTER";
    case 0x80070005: return "E_ACCESSDENIED";
    case 0x80070490: return "E_NOTFOUND";
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
    printf("  %-34s -> 0x%08X  %s\n", label, (unsigned)hr, HrName(hr)); \
} while (0)

// The exact device format the driver advertises (micinwavtable.h entry 0).
static WAVEFORMATEXTENSIBLE MakeDeviceFormat()
{
    WAVEFORMATEXTENSIBLE wfx = {};
    wfx.Format.wFormatTag = WAVE_FORMAT_EXTENSIBLE;
    wfx.Format.nChannels = 2;
    wfx.Format.nSamplesPerSec = 48000;
    wfx.Format.nAvgBytesPerSec = 192000;
    wfx.Format.nBlockAlign = 4;
    wfx.Format.wBitsPerSample = 16;
    wfx.Format.cbSize = sizeof(WAVEFORMATEXTENSIBLE) - sizeof(WAVEFORMATEX);
    wfx.Samples.wValidBitsPerSample = 16;
    wfx.dwChannelMask = KSAUDIO_SPEAKER_STEREO;
    wfx.SubFormat = KSDATAFORMAT_SUBTYPE_PCM;
    return wfx;
}

static void PrintFormat(const WAVEFORMATEX* f)
{
    if (!f) return;
    printf("      fmt: %u ch, %lu Hz, %u bit, tag 0x%04X\n",
           f->nChannels, f->nSamplesPerSec, f->wBitsPerSample, f->wFormatTag);
}

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
    printf("Active capture endpoints: %u\n\n", count);

    IMMDevice* pLampyris = nullptr;

    // ---- Pass 1: control sweep — GetMixFormat + plain shared Initialize on EVERY endpoint.
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
        wprintf(L"[%u] %s\n", i, name);

        IAudioClient* pClient = nullptr;
        STEP("Activate(IAudioClient)", pDev->Activate(__uuidof(IAudioClient), CLSCTX_ALL, nullptr, (void**)&pClient));
        if (SUCCEEDED(hr)) {
            WAVEFORMATEX* pMix = nullptr;
            STEP("GetMixFormat", pClient->GetMixFormat(&pMix));
            PrintFormat(pMix);
            if (pMix) {
                STEP("Initialize(SHARED, mix fmt)", pClient->Initialize(AUDCLNT_SHAREMODE_SHARED, 0, 2000000, 0, pMix, nullptr));
                if (SUCCEEDED(hr)) printf("      (open OK — endpoint healthy)\n");
                CoTaskMemFree(pMix);
            }
            pClient->Release();
        }
        printf("\n");

        if (!pLampyris && wcsstr(name, L"Lampyris")) { pLampyris = pDev; pLampyris->AddRef(); }
        pDev->Release();
    }

    if (!pLampyris) { printf("No capture endpoint with 'Lampyris' in the name.\n"); return 2; }

    // ---- Pass 2: Lampyris deep probe with an EXPLICIT format (never depend on GetMixFormat).
    WAVEFORMATEXTENSIBLE wfx = MakeDeviceFormat();
    WAVEFORMATEX* pFmt = &wfx.Format;
    printf("=== Lampyris deep probe (explicit 2ch/48000/16 WAVEFORMATEXTENSIBLE) ===\n\n");

    // 2a. IsFormatSupported, shared + exclusive.
    {
        IAudioClient* pClient = nullptr;
        STEP("Activate(IAudioClient)", pLampyris->Activate(__uuidof(IAudioClient), CLSCTX_ALL, nullptr, (void**)&pClient));
        if (SUCCEEDED(hr)) {
            WAVEFORMATEX* pClosest = nullptr;
            STEP("IsFormatSupported(SHARED, dev fmt)", pClient->IsFormatSupported(AUDCLNT_SHAREMODE_SHARED, pFmt, &pClosest));
            if (pClosest) { PrintFormat(pClosest); CoTaskMemFree(pClosest); }
            STEP("IsFormatSupported(EXCL, dev fmt)", pClient->IsFormatSupported(AUDCLNT_SHAREMODE_EXCLUSIVE, pFmt, nullptr));
            pClient->Release();
        }
        printf("\n");
    }

    // 2b. Shared mode with the explicit format.
    {
        IAudioClient* pClient = nullptr;
        printf("-- Attempt: SHARED, explicit dev fmt, no event --\n");
        STEP("Activate(IAudioClient)", pLampyris->Activate(__uuidof(IAudioClient), CLSCTX_ALL, nullptr, (void**)&pClient));
        if (SUCCEEDED(hr)) {
            STEP("Initialize", pClient->Initialize(AUDCLNT_SHAREMODE_SHARED, 0, 2000000, 0, pFmt, nullptr));
            if (SUCCEEDED(hr)) {
                STEP("Start", pClient->Start());
                printf("      >>> SHARED OPEN SUCCEEDED <<<\n");
                pClient->Stop();
            }
            pClient->Release();
        }
        printf("\n");
    }

    // 2c. Shared + event callback (the browser path).
    {
        IAudioClient* pClient = nullptr;
        printf("-- Attempt: SHARED + EVENTCALLBACK, explicit dev fmt --\n");
        STEP("Activate(IAudioClient)", pLampyris->Activate(__uuidof(IAudioClient), CLSCTX_ALL, nullptr, (void**)&pClient));
        if (SUCCEEDED(hr)) {
            HANDLE hEv = CreateEventW(nullptr, FALSE, FALSE, nullptr);
            STEP("Initialize(EVENTCALLBACK)", pClient->Initialize(AUDCLNT_SHAREMODE_SHARED,
                     AUDCLNT_STREAMFLAGS_EVENTCALLBACK, 2000000, 0, pFmt, nullptr));
            if (SUCCEEDED(hr)) {
                STEP("SetEventHandle", pClient->SetEventHandle(hEv));
                STEP("Start", pClient->Start());
                printf("      >>> EVENT-DRIVEN OPEN SUCCEEDED <<<\n");
                pClient->Stop();
            }
            if (hEv) CloseHandle(hEv);
            pClient->Release();
        }
        printf("\n");
    }

    // 2d. EXCLUSIVE mode — bypasses the shared engine pipe / format cache entirely and
    //     opens the KS pin directly. Watch DebugView for "NewStream ENTER" here.
    {
        IAudioClient* pClient = nullptr;
        printf("-- Attempt: EXCLUSIVE, explicit dev fmt (bypasses engine pipe; watch for NewStream) --\n");
        STEP("Activate(IAudioClient)", pLampyris->Activate(__uuidof(IAudioClient), CLSCTX_ALL, nullptr, (void**)&pClient));
        if (SUCCEEDED(hr)) {
            STEP("Initialize(EXCLUSIVE)", pClient->Initialize(AUDCLNT_SHAREMODE_EXCLUSIVE, 0, 1000000, 0, pFmt, nullptr));
            if (SUCCEEDED(hr)) {
                UINT32 frames = 0; STEP("GetBufferSize", pClient->GetBufferSize(&frames));
                printf("      buffer frames: %u\n", frames);
                IAudioCaptureClient* pCap = nullptr;
                STEP("GetService(CaptureClient)", pClient->GetService(__uuidof(IAudioCaptureClient), (void**)&pCap));
                STEP("Start", pClient->Start());
                printf("      >>> EXCLUSIVE OPEN SUCCEEDED — driver-side pin creation WORKS <<<\n");
                if (pCap) pCap->Release();
                pClient->Stop();
            }
            pClient->Release();
        }
    }

    printf("\nDone. Interpretation:\n"
           "  control mic also fails            -> VM audio engine broken, not our driver\n"
           "  EXCLUSIVE succeeds, SHARED fails  -> driver fine; shared-mode engine pipe is the bug\n"
           "  EXCLUSIVE fails too               -> the HRESULT + DebugView (NewStream?) name the KS-level reason\n");
    return 0;
}
