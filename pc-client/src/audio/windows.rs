use ringbuf::{
    CachingCons, CachingProd, HeapRb,
    traits::{Consumer, Observer, Producer, Split},
};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info};
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::core::w;

const FILE_DEVICE_LAMPYRIS: u32 = 0x8001;
const LAMPYRIS_FUNC_PUSH_AUDIO: u32 = 0x802;
const METHOD_BUFFERED: u32 = 0;
const FILE_WRITE_ACCESS: u32 = 2;

const fn ctl_code(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
    (device_type << 16) | (access << 14) | (function << 2) | method
}

const IOCTL_LAMPYRIS_PUSH_AUDIO: u32 = ctl_code(
    FILE_DEVICE_LAMPYRIS,
    LAMPYRIS_FUNC_PUSH_AUDIO,
    METHOD_BUFFERED,
    FILE_WRITE_ACCESS,
);

const LAMPYRIS_MAX_AUDIO_PAYLOAD: usize = 4800;

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
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

impl WindowsSink {
    pub fn new(_node_name: String, sample_rate: u32) -> anyhow::Result<Self> {
        let handle = unsafe {
            CreateFileW(
                w!("\\\\.\\LampyrisMic2"),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        };

        let handle =
            handle.map_err(|e| anyhow::anyhow!("Failed to open \\\\.\\LampyrisMic2: {}", e))?;
        let hw = HandleWrapper(handle);

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
        hw: HandleWrapper,
    ) -> anyhow::Result<()> {
        let prebuffer_threshold = (sample_rate / 20) as usize; // 50ms prebuffer
        let mut is_buffering = true;

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
                        let clipped = f.clamp(-1.0, 1.0);
                        let i = (clipped * 32767.0) as i16;
                        pcm_bytes.extend_from_slice(&i.to_le_bytes());
                        samples_read += 1;
                    } else {
                        break;
                    }
                }

                let mut audio_payload = LampyrisAudioPayload {
                    length: 0,
                    data: [0; LAMPYRIS_MAX_AUDIO_PAYLOAD],
                };
                audio_payload.length = (samples_read * 2) as u32;
                audio_payload.data[..samples_read * 2].copy_from_slice(&pcm_bytes);

                let mut bytes_returned = 0u32;
                let success = unsafe {
                    DeviceIoControl(
                        hw.0,
                        IOCTL_LAMPYRIS_PUSH_AUDIO,
                        Some(&audio_payload as *const _ as *const _),
                        std::mem::size_of::<LampyrisAudioPayload>() as u32,
                        None,
                        0,
                        Some(&mut bytes_returned),
                        None,
                    )
                };

                if success.is_err() {
                    error!("Failed to push audio to Lampyris driver");
                }
            } else {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
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