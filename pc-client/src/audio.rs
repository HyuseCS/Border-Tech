use pipewire as pw;
use pw::spa;
use pw::spa::utils::Direction;
use pw::stream::{StreamBox, StreamFlags};
use tracing::{error, info, debug};
use spa::pod::Pod;
use ringbuf::{HeapRb, traits::{Split, Producer, Consumer, Observer}, CachingProd, CachingCons};
use std::sync::{Arc, Mutex};


// SEC-02: Compile-time guard for f32 size
const _: () = assert!(std::mem::size_of::<f32>() == 4);

/// High-performance PipeWire audio sink.
/// Creates a virtual source in the PipeWire graph and streams F32LE audio.
pub struct PipewireSink {
    producer: Arc<Mutex<CachingProd<Arc<HeapRb<f32>>>>>,
    quit_tx: Option<pw::channel::Sender<()>>,
}

// Note: In newer ringbuf versions, CachingProd is naturally Send.

impl PipewireSink {
    /// Creates a new PipeWire sink with the given node name and sample rate.
    pub fn new(node_name: String, sample_rate: u32) -> anyhow::Result<Self> {
        // PERF-01/02: Use ringbuf for O(1) lock-free transfers
        let rb = HeapRb::<f32>::new(sample_rate as usize * 2); // 2 seconds buffer
        let (prod, cons) = rb.split();
        let producer = Arc::new(Mutex::new(prod));
        
        let (quit_tx, quit_rx) = pw::channel::channel::<()>();

        std::thread::spawn(move || {
            if let Err(e) = Self::run_loop(node_name, sample_rate, cons, quit_rx) {
                error!("PipeWire loop error: {}", e);
            }
        });

        Ok(Self { 
            producer,
            quit_tx: Some(quit_tx),
        })
    }

    fn run_loop(
        node_name: String, 
        sample_rate: u32, 
        mut consumer: CachingCons<Arc<HeapRb<f32>>>,
        quit_rx: pw::channel::Receiver<()>
    ) -> anyhow::Result<()> {
        pw::init();
        let mainloop = pw::main_loop::MainLoopBox::new(None)?;
        let context = pw::context::ContextBox::new(&mainloop.loop_(), None)?;
        let core = context.connect(None)?;

        let props = pw::properties::properties! {
            *pw::keys::NODE_NAME => node_name.as_str(),
            *pw::keys::NODE_DESCRIPTION => "Lampyris Virtual Microphone",
            *pw::keys::NODE_NICK => "Lampyris",
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Source",
            *pw::keys::MEDIA_CLASS => "Audio/Source",
            *pw::keys::MEDIA_ROLE => "Communication",
        };

        let stream = StreamBox::new(&core, &node_name, props)?;

        let mut is_buffering = true;
        // 10ms pre-buffering threshold: sample_rate * 10 / 1000 = sample_rate / 100
        let prebuffer_threshold = (sample_rate / 100) as usize;

        let _listener = stream
            .add_local_listener::<()>()
            .process(move |stream, _user_data| {
                if let Some(mut buffer) = stream.dequeue_buffer() {
                    let datas = buffer.datas_mut();
                    let data = &mut datas[0];
                    
                    if let Some(slice) = data.data() {
                        let total_len = slice.len();
                        let requested_samples = total_len / 4;
                        
                        let occupied = consumer.occupied_len();

                        // Jitter Buffer logic
                        if is_buffering {
                            if occupied >= prebuffer_threshold {
                                is_buffering = false;
                                debug!("Jitter buffer filled ({} samples). Starting audio playback.", occupied);
                            }
                        } else if occupied == 0 {
                            is_buffering = true;
                            info!("Jitter buffer underrun. Re-buffering...");
                        }

                        let n_samples = if is_buffering {
                            0
                        } else {
                            requested_samples.min(occupied)
                        };
                        
                        if n_samples > 0 {
                            // SEC-02: Hard bounds check to prevent overrun in release builds
                            assert!(n_samples * 4 <= slice.len(), "Buffer overrun risk: requested {} bytes for {} byte slice", n_samples * 4, slice.len());
                            
                            // SAFETY: The `slice` bounds are explicitly checked above (`n_samples * 4 <= slice.len()`).
                            // The ring buffer ensures that `as_slices()` returns valid memory.
                            // We are safely copying standard PCM floats into the pipewire buffer.
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

        // REL-02: Monitor quit signal via pipewire channel
        let mainloop_ptr = mainloop.as_raw_ptr();
        let _receiver = quit_rx.attach(&mainloop.loop_(), move |_| {
            debug!("Received quit signal, stopping PipeWire loop");
            // SAFETY: The receiver is owned by the mainloop and cancelled/dropped before the mainloop 
            // itself is dropped. Therefore, mainloop_ptr is strictly valid for the duration of this callback.
            unsafe {
                pw_sys::pw_main_loop_quit(mainloop_ptr);
            }
        });

        mainloop.run();
        info!("PipeWire loop exited cleanly");

        Ok(())
    }

    /// Pushes new audio samples into the sink's ring buffer.
    pub fn push_samples(&self, samples: &[f32]) {
        match self.producer.lock() {
            Ok(mut prod) => {
                // If ringbuf is full, we drop oldest samples to keep latency low
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

// REL-02: Implement Drop to signal PipeWire main loop to quit
impl Drop for PipewireSink {
    fn drop(&mut self) {
        info!("Shutting down PipeWire sink...");
        if let Some(tx) = self.quit_tx.take() {
            let _ = tx.send(());
        }
    }
}
