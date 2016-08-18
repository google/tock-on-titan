.PHONY: target/target/release/tango
target/target/release/tango:
	@cargo build --release --target=../../chips/hotel/target.json

.PHONY: doc
doc:
	@cargo doc --target=../../chips/hotel/target.json

.PHONY: clean
clean:
	@cargo clean

