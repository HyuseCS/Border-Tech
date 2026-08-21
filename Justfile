# Top-level build orchestration for Project-M

# Builds both the PC client and Android client, and verifies the Windows driver configuration exists
build-all:
	cd pc-client && cargo build --release
	cd android-client && ./gradlew assembleRelease

# Runs all PC and Android test suites
test-all:
	cd pc-client && cargo test
	cd android-client && ./gradlew test

# Formats and lints the PC and Android projects
lint-all:
	cd pc-client && cargo fmt --check && cargo clippy -- -D warnings
	cd android-client && ./gradlew lint

# Performs security audits on dependencies
audit:
	cd pc-client && cargo deny check
	cd android-client && ./gradlew dependencyCheckAnalyze

# Runs latency benchmarks on the PC client
bench-latency:
	cd pc-client && cargo bench
