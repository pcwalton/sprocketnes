RUSTC?=rustc
RUSTFLAGS?=--cfg ndebug --cfg ncpuspew -O
RUSTLDFLAGS?=-L .

.PHONY: all
all:	nes

FAILUREMSG="If this build failed due to missing SDL bindings, please install them from https://github.com/brson/rust-sdl and copy the .dll/.dylib/.so into this directory or use RUSTLDFLAGS."

nes:	nes.rc apu.rs cpu.rs disasm.rs gfx.rs input.rs main.rs mapper.rs mem.rs ppu.rs rom.rs util.rs
	$(RUSTC) $(RUSTFLAGS) $(RUSTLDFLAGS) $< -o $@ || echo "$(FAILUREMSG)"

.PHONY: clean
clean:
	rm -f nes

