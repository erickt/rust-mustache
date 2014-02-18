RUSTC ?= rustc
RUST_FLAGS ?= -O

all:
	$(RUSTC) $(RUST_FLAGS) src/mustache/lib.rs

check:
	mkdir -p bin
	$(RUSTC) $(RUST_FLAGS) --test -o bin/test-mustache src/mustache/lib.rs
	./bin/test-mustache

clean:
	rm -rf bin build lib
