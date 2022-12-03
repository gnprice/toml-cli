default: build

CARGO ?= $(shell which cargo)
RUST_TARGET ?= x86_64-unknown-linux-musl

.format:
	${CARGO} fmt -- --check

build: .format
	${CARGO} build --target ${RUST_TARGET} --release
	# Cargo will skip checking if it is already checked
	${CARGO} clippy --bins --tests -- -Dwarnings

clean:
	${CARGO} clean

ut:
	RUST_BACKTRACE=1 ${CARGO} test --workspace -- --skip integration --nocapture

integration:
	# run tests under `test` directory
	RUST_BACKTRACE=1 ${CARGO} test --workspace -- integration --nocapture

test: ut integration
