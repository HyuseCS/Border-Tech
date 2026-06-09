#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

#[allow(dead_code)]
pub trait AudioBackend: Send + Sync {
    fn push_samples(&self, samples: &[f32]);
}

#[cfg(target_os = "linux")]
pub use linux::PipewireSink as DefaultAudioBackend;

#[cfg(target_os = "windows")]
pub use windows::WindowsSink as DefaultAudioBackend;
