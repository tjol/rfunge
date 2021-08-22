# rfunge – rustiest of funges

RFunge is a standards-compliant [Funge-98] interpreter written in Rust.

It currently supports Unefunge and Befunge in 32-bit and 64-bit modes, and
should support Trefunge (and possibly higher dimensions) in future.
Concurrent Funge-98 is supported.

RFunge follows the [spec] and passes the [Mycology] test suite, but it does not
currently suport many fingerprints. The performance of rfunge is broadly similar
to [cfunge] and [CCBI] in many cases, making it one of the faster Befunge-98
interpreters available.

Much like [cfunge], the rfunge's command-line interface supports a sandbox mode
in which instructions like `i`, `o` and `=` are disabled.

RFunge is (in principle) embeddable, and beside the main Rust API, there are
rudimentary C and WASM APIs

## WebAssembly

RFunge can run in the browser. [Try it out here!](https://tjol.eu/rfunge/)

## Unicode Funge-98

To my knowledge, rfunge is the first interpreter to implement **Unicode Funge-98**.
In Unicode mode, source files are read in as UTF-8, and the instructions `~` and
`,` read and write unicode characters rather than bytes. This shouldn't make a
difference to most programs, unless the files contain bytes > 127, they're
trying to read and write binary data, or they're trying to talk to the user in
an encoding other than Latin-1.

While systems with multi-byte characters are explicitly allowed by the [spec],
rfunge also features a traditional binary mode for compatibility with programs
that require it (such as [Mycology]).

## How to install

To build and install rfunge, you will need an up-to-date installation of rust
and cargo (stable), which you can get with [rustup](https://rustup.rs/).

To build a local checkout, run

    cargo build

or

    cargo build --release

The binary will be placed under `target`.

To install rfunge, you can run

    cargo install --path .

from a local checkout, or, aternatively

    cargo install --git https://github.com/tjol/rfunge


[Funge-98]: https://esolangs.org/wiki/Funge-98
[spec]: https://github.com/catseye/Funge-98/blob/master/doc/funge98.markdown
[Mycology]: https://github.com/Deewiant/Mycology
[cfunge]: https://github.com/VorpalBlade/cfunge
[CCBI]: https://github.com/Deewiant/CCBI
