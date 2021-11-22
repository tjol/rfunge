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

RFunge is (in principle) embeddable, and beside the main Rust API, there is a
WASM API used for the web version. It should run on most systems supported by
Rust (tested on Linux, MacOS, Windows and WASM).

## WebAssembly

RFunge can run in the browser. [Try it out here!](https://tjol.eu/rfunge/)
In the browser, `=` executes JavaScript code rather than traditional shell
commands.

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

## Handprint

The native build uses handprint 0x52464e47 ('RFNG'), the WebAssembly build uses
handprint 0x52464e57 ('RFNW').

## How to build (native)

By default, rfunge is built with support for a GUI display for the TURT fingerprint,
but without support for NCRS (ncurses). To build with the default features, run

    cargo build --release

The binary will be placed under `target`.

To build without the GUI, run

    cargo build --release --no-default-features

and to build with NCRS support, run

    cargo build --release --features ncurses

Building with NCRS will only work on a UNIX system (Linux, MacOS) with the ncurses
library and header(s) and a C compiler. It will not work on Windows.

To install, look into the options for `cargo install`.

## How to build (WebAssembly)

You will need [wasm-pack] to build the WASM package. If `wasm-pack` is installed,
you can run `./build_wasm.sh` to build; the WASM binary, and a wrapper script will
be placed in `webapp/rfunge_wasm/`.

To try the actual web UI, navigate into `webapp` and run the `dev` script with your
favourite JavaScript package manager

```
cd webapp
npm install
npm run dev
```


[Funge-98]: https://esolangs.org/wiki/Funge-98
[spec]: https://github.com/catseye/Funge-98/blob/master/doc/funge98.markdown
[Mycology]: https://github.com/Deewiant/Mycology
[cfunge]: https://github.com/VorpalBlade/cfunge
[CCBI]: https://github.com/Deewiant/CCBI
[wasm-pack]: https://rustwasm.github.io/wasm-pack/
