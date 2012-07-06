RUSTC ?= rustc

# ------------------
# Internal variables
dummy1 := $(shell mkdir bin 2> /dev/null)

# ------------------
# Primary targets
all: lib

check: bin/test-mustache
	export RUST_LOG=mustache=1 && ./bin/test-mustache

check1: bin/test-mustache
	export RUST_LOG=mustache=3 && ./bin/test-mustache test_render_sections

clean:
	rm -rf bin

# ------------------
# Binary targets 
# We always build the lib because:
# 1) We don't do it that often.
# 2) It's fast.
# 3) The compiler gives it some crazy name like "libmustache-da45653350eb4f90-0.1.dylib"
# which is dependent on some hash(?) as well as the current platform. (And -o works when
# setting an executable's name, but not libraries).
.PHONY : lib
lib:
	$(RUSTC) --out-dir bin -O mustache.rc

bin/test-mustache: mustache.rc *.rs
	$(RUSTC) --test -o $@ $<

