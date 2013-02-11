`sprocketnes` is an emulator for the Nintendo Entertainment System written in
the Rust programming language.

Its purpose is to serve as a *technology demonstration* to show how the Rust
programming language is suitable for systems software such as emulators. It
has many shortcomings and is not intended to be a production-quality emulator.
`sprocketnes` is also designed to be a relatively clean example codebase,
showing off various Rust idioms.

The Rust garbage collector is not used in this project. Also, because unsafe
code is only used to call OS and SDL functions, this emulator should be type-
and memory-safe.

The NES was chosen for this project because:

* It's familiar to most hackers.

* It's a reasonably simple system to emulate.

* Because of its popularity, its workings are relatively well-documented.

* It's CPU-bound, so it can serve as a benchmark to help optimize Rust code.

The controls are as follows:

* A: Z

* B: X

* Start: Enter

* Select: Right shift

* D-Pad: Arrows

If you want to build `sprocketnes`, you will first need `rust-sdl`, available
at https://github.com/brson/rust-sdl. You will also need the Rust master
branch; no Rust release can build `sprocketnes`.

There are numerous demos and games available for free for use with this
emulator at http://nesdev.com/.

Enjoy!

