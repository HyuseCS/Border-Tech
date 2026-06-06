# Graph Report - .  (2026-06-06)

## Corpus Check
- Corpus is ~9,858 words - fits in a single context window. You may not need a graph.

## Summary
- 168 nodes · 213 edges · 28 communities (23 shown, 5 thin omitted)
- Extraction: 83% EXTRACTED · 17% INFERRED · 0% AMBIGUOUS · INFERRED: 36 edges (avg confidence: 0.89)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_Audio Output & Jitter Buffer|Audio Output & Jitter Buffer]]
- [[_COMMUNITY_PC Client Connection Management|PC Client Connection Management]]
- [[_COMMUNITY_Android Audio Capture Service|Android Audio Capture Service]]
- [[_COMMUNITY_Android Main Activity & UI|Android Main Activity & UI]]
- [[_COMMUNITY_Lampyris Logo Graphics & Concepts|Lampyris Logo Graphics & Concepts]]
- [[_COMMUNITY_PC App State Handling|PC App State Handling]]
- [[_COMMUNITY_System Architecture & Security Findings|System Architecture & Security Findings]]
- [[_COMMUNITY_PC Communication Protocol|PC Communication Protocol]]
- [[_COMMUNITY_PC Audio Sink (Pipewire)|PC Audio Sink (Pipewire)]]
- [[_COMMUNITY_Android SSL & Server Security|Android SSL & Server Security]]
- [[_COMMUNITY_PC CLI Entry Point|PC CLI Entry Point]]
- [[_COMMUNITY_Android Application Module Config|Android Application Module Config]]
- [[_COMMUNITY_Protocol Type Parameters|Protocol Type Parameters]]
- [[_COMMUNITY_PC Client CLI Arguments|PC Client CLI Arguments]]

## God Nodes (most connected - your core abstractions)
1. `AudioCaptureService` - 16 edges
2. `AppState::run_connection` - 10 edges
3. `main` - 8 edges
4. `MainScreen` - 8 edges
5. `Audit Findings Report` - 8 edges
6. `MainScreen()` - 7 edges
7. `AppState::run_connection` - 7 edges
8. `PipewireSink::run_loop` - 7 edges
9. `Lampyris SVG Logo` - 7 edges
10. `Orbital Circles Geometry` - 6 edges

## Surprising Connections (you probably didn't know these)
- `Ephemeral Self-Signed Certificates` --semantically_similar_to--> `Secure by Design`  [INFERRED] [semantically similar]
  android-client/app/src/main/java/com/projectm/mic/AudioCaptureService.kt → README.md
- `Jitter Buffer` --semantically_similar_to--> `Finding P-01: Jitter/latency optimization`  [INFERRED] [semantically similar]
  pc-client/src/audio.rs → findings.md
- `main` --semantically_similar_to--> `MainActivity`  [INFERRED] [semantically similar]
  pc-client/src/main.rs → android-client/app/src/main/java/com/projectm/mic/MainActivity.kt
- `MainScreen` --rationale_for--> `15% Accent Rule`  [INFERRED]
  android-client/app/src/main/java/com/projectm/mic/MainActivity.kt → DESIGN.md
- `Lampyris SVG Logo` --semantically_similar_to--> `Sonus SVG Logo`  [INFERRED] [semantically similar]
  pc-client/ui/Lampyris.svg → Logos/Sonus.svg

## Hyperedges (group relationships)
- **Audio Streaming Pipeline** — audiocaptureservice_handleclient, protocol_protocolhandler_read_audio_packet, app_state_appstate_run_connection, audio_pipewiresink_push_samples [INFERRED 0.95]
- **TLS Security Boundary** — app_state_dummyverifier, audiocaptureservice_setupsslcontext, readme_secure_by_design, findings_c_03 [INFERRED 0.95]
- **Android-PC Connection and Tunneling** — app_state_adb_port_forwarding, app_state_appstate_run_connection, audiocaptureservice_runserverloop [INFERRED 0.95]
- **Bioluminescent Concept Elements** — lampyris_glowing_abdomen, lampyris_glow_filter, lampyris_trion_gradient [INFERRED 0.85]
- **Cybernetic Motif Elements** — lampyris_cube, lampyris_orbital_rings, lampyris_trion_gradient [INFERRED 0.85]
- **Firefly Anatomy Concept** — lampyris_central_core, lampyris_sensory_antennae, lampyris_polygonal_wings, lampyris_lantern_abdomen [INFERRED 0.95]
- **Glowing Visual Features** — lampyris_central_core, lampyris_sensory_antennae, lampyris_lantern_abdomen [EXTRACTED 1.00]
- **Sonus Visual Identity** — sonus_logo, sonus_core_shard, sonus_sound_waves, sonus_color_scheme [EXTRACTED 1.00]

## Communities (28 total, 5 thin omitted)

### Community 0 - "Audio Output & Jitter Buffer"
Cohesion: 0.09
Nodes (27): ADB Port Forwarding, AppState::connect, AppState::run_connection, calculate_peak_amplitude, convert_s16_to_f32, DummyVerifier, Jitter Buffer, Lock-Free Ring Buffer (+19 more)

### Community 1 - "PC Client Connection Management"
Cohesion: 0.11
Nodes (20): ADB Port Reverse Forwarding, AppState, AppState::disconnect, AppState::new, AppState::connect, AppState::disconnect, AppState::run_connection, PipewireSink::push_samples (+12 more)

### Community 3 - "Android Main Activity & UI"
Cohesion: 0.19
Nodes (13): AmbientBlobs(), getLocalIpAddress(), GlassCard(), MainActivity, MainScreen(), PowerPulse(), SelectorButton(), SonusTheme (+5 more)

### Community 4 - "Lampyris Logo Graphics & Concepts"
Cohesion: 0.26
Nodes (15): Glow Antennae Geometry, Background Slate Gradient Definition, Central Nucleus Core, Isometric 3D Cube Definition, SVG Glow Filter Definition, Bioluminescent Light Organ, Isometric Cube Data Symbol, Glowing Lantern Abdomen (+7 more)

### Community 5 - "PC App State Handling"
Cohesion: 0.16
Nodes (3): AppState, AsyncStream, DummyVerifier

### Community 6 - "System Architecture & Security Findings"
Cohesion: 0.17
Nodes (11): AudioCaptureService.stopService, 15% Accent Rule, The Neon Audio Deck Theme, Finding L-01: 8.8.8.8 dependency for local IP, Fallback IP Discovery, get_local_ip, Cinema Mobile Design, getLocalIpAddress (+3 more)

### Community 7 - "PC Communication Protocol"
Cohesion: 0.43
Nodes (5): ProtocolHandler, ProtocolHandler<S>, test_buffer_overflow_protection(), test_protocol_resync_mc(), test_valid_mc_packet_parsing()

### Community 9 - "Android SSL & Server Security"
Cohesion: 0.4
Nodes (6): Ephemeral Self-Signed Certificates, AudioCaptureService.generateSelfSignedCertificate, AudioCaptureService.runServerLoop, AudioCaptureService.setupSslContext, AudioCaptureService.startService, Finding C-01: TCP server on all interfaces

### Community 10 - "PC CLI Entry Point"
Cohesion: 0.67
Nodes (3): Args, get_local_ip(), main()

## Knowledge Gaps
- **28 isolated node(s):** `Args`, `ProtocolHandler`, `ConnectionState`, `app/build.gradle.kts`, `PipewireSink::push_samples` (+23 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **5 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `main` connect `PC Client Connection Management` to `Audio Output & Jitter Buffer`, `System Architecture & Security Findings`?**
  _High betweenness centrality (0.051) - this node is a cross-community bridge._
- **Why does `Audit Findings Report` connect `Audio Output & Jitter Buffer` to `Android SSL & Server Security`, `System Architecture & Security Findings`?**
  _High betweenness centrality (0.034) - this node is a cross-community bridge._
- **Are the 3 inferred relationships involving `MainScreen` (e.g. with `AudioCaptureService` and `15% Accent Rule`) actually correct?**
  _`MainScreen` has 3 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Args`, `ProtocolHandler`, `ConnectionState` to the rest of the system?**
  _28 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Audio Output & Jitter Buffer` be split into smaller, more focused modules?**
  _Cohesion score 0.09 - nodes in this community are weakly interconnected._
- **Should `PC Client Connection Management` be split into smaller, more focused modules?**
  _Cohesion score 0.11 - nodes in this community are weakly interconnected._