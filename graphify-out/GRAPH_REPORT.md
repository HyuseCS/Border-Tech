# Graph Report - .  (2026-06-05)

## Corpus Check
- Corpus is ~7,632 words - fits in a single context window. You may not need a graph.

## Summary
- 88 nodes · 91 edges · 25 communities (19 shown, 6 thin omitted)
- Extraction: 95% EXTRACTED · 5% INFERRED · 0% AMBIGUOUS · INFERRED: 5 edges (avg confidence: 0.88)
- Token cost: 6,500 input · 4,200 output

## Community Hubs (Navigation)
- [[_COMMUNITY_Android Audio Capture Service|Android Audio Capture Service]]
- [[_COMMUNITY_Rust PC Client State & Main Integration|Rust PC Client State & Main Integration]]
- [[_COMMUNITY_Rust Protocol Handling & Unit Tests|Rust Protocol Handling & Unit Tests]]
- [[_COMMUNITY_Rust Client Connection & Conversions|Rust Client Connection & Conversions]]
- [[_COMMUNITY_System Architecture & Documentation Specs|System Architecture & Documentation Specs]]
- [[_COMMUNITY_Rust Pipewire Audio Playback|Rust Pipewire Audio Playback]]
- [[_COMMUNITY_Neon UI Design & Screen Layouts|Neon UI Design & Screen Layouts]]
- [[_COMMUNITY_Rust Main Entry & Network Helper|Rust Main Entry & Network Helper]]
- [[_COMMUNITY_Android Service Controller Interface|Android Service Controller Interface]]
- [[_COMMUNITY_Android MainActivity Lifecycle|Android MainActivity Lifecycle]]
- [[_COMMUNITY_App Build Gradle Metadata|App Build Gradle Metadata]]

## God Nodes (most connected - your core abstractions)
1. `AudioCaptureService` - 12 edges
2. `AppState::run_connection` - 7 edges
3. `PipewireSink` - 5 edges
4. `AppState` - 5 edges
5. `main` - 5 edges
6. `MC Framing Protocol` - 4 edges
7. `test_valid_mc_packet_parsing()` - 3 edges
8. `test_protocol_resync_mc()` - 3 edges
9. `test_buffer_overflow_protection()` - 3 edges
10. `ProtocolHandler::read_audio_packet` - 3 edges

## Surprising Connections (you probably didn't know these)
- `MainActivity` --semantically_similar_to--> `main`  [INFERRED] [semantically similar]
  android-client/app/src/main/java/com/projectm/mic/MainActivity.kt → pc-client/src/main.rs
- `The Neon Audio Deck Theme` --rationale_for--> `MainActivity`  [INFERRED]
  DESIGN.md → android-client/app/src/main/java/com/projectm/mic/MainActivity.kt
- `15% Accent Rule` --rationale_for--> `MainScreen`  [INFERRED]
  DESIGN.md → android-client/app/src/main/java/com/projectm/mic/MainActivity.kt
- `AudioCaptureService::sendAudioPacket` --shares_data_with--> `ProtocolHandler::read_audio_packet`  [INFERRED]
  android-client/app/src/main/java/com/projectm/mic/AudioCaptureService.kt → pc-client/src/protocol.rs
- `MC Framing Protocol` --rationale_for--> `AudioCaptureService::sendAudioPacket`  [EXTRACTED]
  pc-client/src/protocol.rs → android-client/app/src/main/java/com/projectm/mic/AudioCaptureService.kt

## Hyperedges (group relationships)
- **End-to-End Audio Streaming Pipeline** — audiocaptureservice_audiocaptureservice, protocol_mc_framing_protocol, app_state_appstate, audio_pipewiresink [INFERRED 0.95]
- **Tactical Neon UI Theme Implementation** — design_neon_audio_deck, main_main, mainactivity_mainactivity [INFERRED 0.95]

## Communities (25 total, 6 thin omitted)

### Community 1 - "Rust PC Client State & Main Integration"
Cohesion: 0.18
Nodes (12): ADB Port Reverse Forwarding, AppState, AppState::connect, AppState::disconnect, AppState::run_connection, Jitter Buffer, PipewireSink, PipewireSink::push_samples (+4 more)

### Community 2 - "Rust Protocol Handling & Unit Tests"
Cohesion: 0.43
Nodes (5): ProtocolHandler, ProtocolHandler<S>, test_buffer_overflow_protection(), test_protocol_resync_mc(), test_valid_mc_packet_parsing()

### Community 4 - "System Architecture & Documentation Specs"
Cohesion: 0.32
Nodes (6): AudioCaptureService, Foreground Capture Service, AudioCaptureService::runCaptureLoop, AudioCaptureService::sendAudioPacket, MC Framing Protocol, ProtocolHandler::read_audio_packet

### Community 6 - "Neon UI Design & Screen Layouts"
Cohesion: 0.4
Nodes (4): 15% Accent Rule, The Neon Audio Deck Theme, MainActivity, MainScreen

### Community 7 - "Rust Main Entry & Network Helper"
Cohesion: 0.67
Nodes (3): Args, get_local_ip(), main()

## Knowledge Gaps
- **10 isolated node(s):** `Args`, `ProtocolHandler`, `ConnectionState`, `app/build.gradle.kts`, `get_local_ip` (+5 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **6 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AppState::run_connection` connect `Rust PC Client State & Main Integration` to `System Architecture & Documentation Specs`?**
  _High betweenness centrality (0.043) - this node is a cross-community bridge._
- **Why does `AudioCaptureService` connect `Android Audio Capture Service` to `Android Service Controller Interface`?**
  _High betweenness centrality (0.041) - this node is a cross-community bridge._
- **Why does `main` connect `Rust PC Client State & Main Integration` to `Neon UI Design & Screen Layouts`?**
  _High betweenness centrality (0.030) - this node is a cross-community bridge._
- **What connects `Args`, `ProtocolHandler`, `ConnectionState` to the rest of the system?**
  _10 weakly-connected nodes found - possible documentation gaps or missing edges._