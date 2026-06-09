use std::sync::{Arc, Mutex};
use tracing::{error, info, debug};
use windows::core::w;
use windows::Win32::Foundation::{HANDLE, CloseHandle, GENERIC_WRITE, GENERIC_READ};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Registry::{
    RegOpenKeyExW, RegQueryValueExW, RegCloseKey, HKEY_LOCAL_MACHINE, HKEY, KEY_READ, REG_VALUE_TYPE
};
use ringbuf::{HeapRb, traits::{Split, Producer, Consumer, Observer}, CachingProd, CachingCons};

const FILE_DEVICE_LAMPYRIS: u32 = 0x8001;
const LAMPYRIS_FUNC_AUTHENTICATE: u32 = 0x801;
const LAMPYRIS_FUNC_PUSH_AUDIO: u32 = 0x802;
const METHOD_BUFFERED: u32 = 0;
const FILE_WRITE_ACCESS: u32 = 2;

const fn ctl_code(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
    (device_type << 16) | (access << 14) | (function << 2) | method
}

const IOCTL_LAMPYRIS_AUTHENTICATE: u32 = ctl_code(FILE_DEVICE_LAMPYRIS, LAMPYRIS_FUNC_AUTHENTICATE, METHOD_BUFFERED, FILE_WRITE_ACCESS);
const IOCTL_LAMPYRIS_PUSH_AUDIO: u32 = ctl_code(FILE_DEVICE_LAMPYRIS, LAMPYRIS_FUNC_PUSH_AUDIO, METHOD_BUFFERED, FILE_WRITE_ACCESS);

const LAMPYRIS_MAX_AUDIO_PAYLOAD: usize = 4800;

#[repr(C)]
struct LampyrisAuthPayload {
    token: [u8; 32],
}

#[repr(C)]
struct LampyrisAudioPayload {
    length: u32,
    data: [u8; LAMPYRIS_MAX_AUDIO_PAYLOAD],
}

pub struct WindowsSink {
    producer: Arc<Mutex<CachingProd<Arc<HeapRb<f32>>>>>,
    quit_tx: Option<std::sync::mpsc::Sender<()>>,
}

impl crate::audio::AudioBackend for WindowsSink {
    fn push_samples(&self, samples: &[f32]) {
        self.push_samples(samples);
    }
}

struct HandleWrapper(HANDLE);
unsafe impl Send for HandleWrapper {}
unsafe impl Sync for HandleWrapper {}
impl Drop for HandleWrapper {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe { let _ = CloseHandle(self.0); }
        }
    }
}

impl WindowsSink {
    pub fn new(_node_name: String, sample_rate: u32) -> anyhow::Result<Self> {
        let mut token = [0u8; 32];
        
        unsafe {
            let mut hkey = HKEY::default();
            // Note: KEY_READ requires WIN32_SYSTEM_REGISTRY features
            let status = RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                w!("SOFTWARE\\Lampyris"),
                0,
                KEY_READ,
                &mut hkey
            );
            if status.is_err() {
                return Err(anyhow::anyhow!("Failed to open registry key: {:?}", status));
            }
            
            let mut token_len = 32u32;
            let mut val_type = 0u32;
            let status = RegQueryValueExW(
                hkey,
                w!("SessionToken"),
                None,
                Some(&mut val_type as *mut _ as *mut _),
                Some(token.as_mut_ptr() as *mut _),
                Some(&mut token_len)
            );
            
            let _ = RegCloseKey(hkey);
            
            if status.is_err() || token_len != 32 {
                return Err(anyhow::anyhow!("Failed to read SessionToken from registry or invalid size"));
            }
        }

        let handle = unsafe {
            CreateFileW(
                w!("\\\\.\\LampyrisMic"),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None
            )
        };
        
        let handle = handle.map_err(|e| anyhow::anyhow!("Failed to open \\\\.\\LampyrisMic: {}", e))?;
        let hw = HandleWrapper(handle);

        let mut bytes_returned = 0u32;
        let mut auth_payload = LampyrisAuthPayload { token };
        let success = unsafe {
            DeviceIoControl(
                hw.0,
                IOCTL_LAMPYRIS_AUTHENTICATE,
                Some(&auth_payload as *const _ as *const _),
                std::mem::size_of::<LampyrisAuthPayload>() as u32,
                None,
                0,
                Some(&mut bytes_returned),
                None
            )
        };

        if success.is_err() {
            return Err(anyhow::anyhow!("Failed to authenticate with LampyrisMic: {:?}", success));
        }
        
        let rb = HeapRb::<f32>::new(sample_rate as usize * 2);
        let (prod, cons) = rb.split();
        let producer = Arc::new(Mutex::new(prod));
        
        let (quit_tx, quit_rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            if let Err(e) = Self::run_loop(sample_rate, cons, quit_rx, hw) {
                error!("Windows audio loop error: {}", e);
            }
        });

        Ok(Self { 
            producer,
            quit_tx: Some(quit_tx),
        })
    }

    fn run_loop(
        sample_rate: u32, 
        mut consumer: CachingCons<Arc<HeapRb<f32>>>,
        quit_rx: std::sync::mpsc::Receiver<()>,
        hw: HandleWrapper
    ) -> anyhow::Result<()> {
        let mut is_buffering = true;
        let prebuffer_threshold = (sample_rate / 100) as usize;
        
        let mut audio_payload = LampyrisAudioPayload {
            length: 0,
            data: [0u8; LAMPYRIS_MAX_AUDIO_PAYLOAD],
        };
        
        loop {
            if quit_rx.try_recv().is_ok() {
                break;
            }

            let occupied = consumer.occupied_len();
            if is_buffering {
                if occupied >= prebuffer_threshold {
                    is_buffering = false;
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                    continue;
                }
            } else if occupied == 0 {
                is_buffering = true;
                std::thread::sleep(std::time::Duration::from_millis(2));
                continue;
            }

            let requested_samples = 2400.min(occupied);
            if requested_samples > 0 {
                let mut pcm_bytes = Vec::with_capacity(requested_samples * 2);
                let mut samples_read = 0;
                while samples_read < requested_samples {
                    if let Some(f) = consumer.try_pop() {
                        let clipped = f.max(-1.0).min(1.0);
                        let i = (clipped * 32767.0) as i16;
                        pcm_bytes.extend_from_slice(&i.to_le_bytes());
                        samples_read += 1;
                    } else {
                        break;
                    }
                }
                
                audio_payload.length = (samples_read * 2) as u32;
                audio_payload.data[..samples_read * 2].copy_from_slice(&pcm_bytes);
                
                let mut bytes_returned = 0u32;
                let success = unsafe {
                    DeviceIoControl(
                        hw.0,
                        IOCTL_LAMPYRIS_PUSH_AUDIO,
                        Some(&audio_payload as *const _ as *const _),
                        std::mem::size_of::<u32>() as u32 + audio_payload.length,
                        None,
                        0,
                        Some(&mut bytes_returned),
                        None
                    )
                };
                
                if let Err(e) = success {
                    error!("DeviceIoControl IOCTL_LAMPYRIS_PUSH_AUDIO failed: {:?}", e);
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
            
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        
        info!("Windows audio loop exited cleanly");
        Ok(())
    }

    pub fn push_samples(&self, samples: &[f32]) {
        match self.producer.lock() {
            Ok(mut prod) => {
                if prod.vacant_len() < samples.len() {
                    debug!("Audio buffer overflow, some samples may be dropped or delayed");
                }
                prod.push_slice(samples);
            }
            Err(e) => {
                error!("Audio producer mutex is poisoned: {}", e);
            }
        }
    }
}

impl Drop for WindowsSink {
    fn drop(&mut self) {
        if let Some(tx) = self.quit_tx.take() {
            let _ = tx.send(());
        }
    }
}
