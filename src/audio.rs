use pipewire as pw;
use pw::spa;
use pw::spa::utils::Direction;
use pw::stream::{StreamBox, StreamFlags};
use tracing::{error, info, debug};
use spa::pod::Pod;
use ringbuf::{HeapRb, traits::{Split, Producer, Consumer, Observer}, CachingProd, CachingCons};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

// SEC-02: Compile-time guard for f32 size
const _: () = assert!(std::mem::size_of::<f32>() == 4);

/// High-performance PipeWire audio sink.
/// Creates a virtual source in the PipeWire graph and streams F32LE audio.
pub struct PipewireSink {
    producer: Arc<Mutex<CachingProd<Arc<HeapRb<f32>>>>>,
    quit_flag: Arc<AtomicBool>,
}

// SAFETY: PipewireSink is safe to send between threads because:
// 1. The Producer (ringbuf) is wrapped in Arc<Mutex<>>, and its underlying storage is Arc<HeapRb>.
// 2. The quit_flag is an AtomicBool wrapped in Arc.
// 3. We do not hold any thread-local PipeWire objects (like MainLoopBox or Context) in this struct.
//    Those are either moved into the background thread or exist only within the run_loop function.
unsafe impl Send for PipewireSink {}

impl PipewireSink {
    /// Creates a new PipeWire sink with the given node name and sample rate.
    pub fn new(node_name: String, sample_rate: u32) -> anyhow::Result<Self> {
        // PERF-01/02: Use ringbuf for O(1) lock-free transfers
        let rb = HeapRb::<f32>::new(sample_rate as usize * 2); // 2 seconds buffer
        let (prod, cons) = rb.split();
        let producer = Arc::new(Mutex::new(prod));
        let quit_flag = Arc::new(AtomicBool::new(false));
        let quit_flag_clone = quit_flag.clone();

        std::thread::spawn(move || {
            if let Err(e) = Self::run_loop(node_name, sample_rate, cons, quit_flag_clone) {
                error!("PipeWire loop error: {}", e);
            }
        });

        Ok(Self { 
            producer,
            quit_flag,
        })
    }

    fn run_loop(
        node_name: String, 
        sample_rate: u32, 
        mut consumer: CachingCons<Arc<HeapRb<f32>>>,
        quit_flag: Arc<AtomicBool>
    ) -> anyhow::Result<()> {
        pw::init();
        let mainloop = pw::main_loop::MainLoopBox::new(None)?;
        let context = pw::context::ContextBox::new(&mainloop.loop_(), None)?;
        let core = context.connect(None)?;

        let props = pw::properties::properties! {
            *pw::keys::NODE_NAME => node_name.as_str(),
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Source",
            *pw::keys::MEDIA_ROLE => "Communication",
        };

        let stream = StreamBox::new(&core, &node_name, props)?;

        let _listener = stream
            .add_local_listener::<()>()
            .process(move |stream, _user_data| {
                if let Some(mut buffer) = stream.dequeue_buffer() {
                    let datas = buffer.datas_mut();
                    let data = &mut datas[0];
                    
                    if let Some(slice) = data.data() {
                        let total_len = slice.len();
                        let n_samples = (total_len / 4).min(consumer.occupied_len());
                        
                        if n_samples > 0 {
                            // SEC-02: Defensive bounds check
                            debug_assert!(n_samples * 4 <= slice.len());
                            
                            unsafe {
                                // PERF: Read directly from ring buffer into PipeWire slice
                                let (s1, s2) = consumer.as_slices();
                                let first_len = s1.len().min(n_samples);
                                
                                std::ptr::copy_nonoverlapping(
                                    s1.as_ptr() as *const u8,
                                    slice.as_mut_ptr(),
                                    first_len * 4,
                                );
                                
                                if first_len < n_samples {
                                    let second_len = n_samples - first_len;
                                    std::ptr::copy_nonoverlapping(
                                        s2.as_ptr() as *const u8,
                                        slice.as_mut_ptr().add(first_len * 4),
                                        second_len * 4,
                                    );
                                }
                            }
                            consumer.skip(n_samples);
                            
                            let chunk = data.chunk_mut();
                            *chunk.offset_mut() = 0;
                            *chunk.size_mut() = (n_samples * 4) as u32;
                            *chunk.stride_mut() = 4;
                        } else {
                            // SEC-02: Use safe fill(0) for silence
                            slice.fill(0);
                            let chunk = data.chunk_mut();
                            *chunk.offset_mut() = 0;
                            *chunk.size_mut() = total_len as u32;
                            *chunk.stride_mut() = 4;
                        }
                    }
                }
            })
            .register()?;

        let mut audio_info = spa::param::audio::AudioInfoRaw::new();
        audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
        audio_info.set_rate(sample_rate);
        audio_info.set_channels(1);

        let values: Vec<u8> = pw::spa::pod::serialize::PodSerializer::serialize(
            std::io::Cursor::new(Vec::new()),
            &pw::spa::pod::Value::Object(pw::spa::pod::Object {
                type_: spa_sys::SPA_TYPE_OBJECT_Format,
                id: spa_sys::SPA_PARAM_EnumFormat,
                properties: audio_info.into(),
            }),
        )
        .map_err(|e| anyhow::anyhow!("Serialization failed: {:?}", e))?
        .0
        .into_inner();

        let mut params = [Pod::from_bytes(&values).ok_or_else(|| anyhow::anyhow!("Pod from_bytes failed"))?];

        info!("Connecting PipeWire stream as a source node: {}", node_name);

        stream.connect(
            Direction::Output,
            None,
            StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS,
            &mut params,
        )?;

        // REL-02: Monitor quit flag using a repeating timer
        let mainloop_ptr = mainloop.as_raw_ptr();
        let mainloop_loop = mainloop.loop_();
        let timer = mainloop_loop.add_timer(move |_expirations| {
            if quit_flag.load(Ordering::Relaxed) {
                debug!("Received quit signal, stopping PipeWire loop");
                unsafe {
                    pw_sys::pw_main_loop_quit(mainloop_ptr);
                }
            }
        });
        
        // Arm as repeating timer: value = 100ms, interval = 100ms
        timer.update_timer(
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_millis(100)),
        );

        mainloop.run();
        info!("PipeWire loop exited cleanly");

        Ok(())
    }

    /// Pushes new audio samples into the sink's ring buffer.
    pub fn push_samples(&self, samples: &[f32]) {
        let mut prod = self.producer.lock().unwrap();
        // If ringbuf is full, we drop oldest samples to keep latency low
        if prod.vacant_len() < samples.len() {
            debug!("Audio buffer overflow, some samples may be dropped or delayed");
        }
        prod.push_slice(samples);
    }
}

// REL-02: Implement Drop to signal PipeWire main loop to quit
impl Drop for PipewireSink {
    fn drop(&mut self) {
        info!("Shutting down PipeWire sink...");
        self.quit_flag.store(true, Ordering::Relaxed);
    }
}
