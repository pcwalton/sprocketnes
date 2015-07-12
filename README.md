`sprocketnes` is an emulator for the Nintendo Entertainment System written in
the Rust programming language.

Its purpose is to serve as a *technology demonstration* to show how the Rust
programming language is suitable for systems software such as emulators. It
has many shortcomings and is not intended to be a production-quality emulator.
`sprocketnes` is also designed to be a relatively clean example codebase,
showing off various Rust idioms.

The NES was chosen for this project because:

* It's familiar to most hackers.

* It's a reasonably simple system to emulate.

* Because of its popularity, its workings are relatively well-documented.

* It's CPU-bound, so it can serve as a benchmark to help optimize Rust code.

* The audio requires some measure of real-time operation, which tests Rust's
  real-time capabilities.

The main controls are as follows:

* A: Z

* B: X

* Start: Enter

* Select: Right shift

* D-Pad: Arrows

Other keys:

* Save state: S

* Load state: L

* Quit: Escape

If you want to build `sprocketnes`, you will first need the Speex codec library
installed; on the Mac you can install it with `brew install speex`.

To build (add `--release` if you actually want playable speed):

    cargo build

There are numerous demos and games available for free for use with this
emulator at http://nesdev.com/.

Enjoy!
