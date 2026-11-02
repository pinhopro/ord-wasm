# Ord protocol Wasm feasibility probe

This isolated experiment executes the existing `ordinals` crate in core Wasm.
It is a contribution starting point, not a port of the Ord CLI, HTTP server,
inscription parser, historical indexer, or wallet. No production APIs are changed.

## Reproduce

With Rust 1.97.1 and Node installed:

```sh
rustup target add wasm32-unknown-unknown
cd contrib/wasm-probe
bash scripts/build.sh
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --check
```

The wrapper has a separate workspace and lockfile to avoid changing the parent
workspace. A Wasm-capable C compiler and archiver are required for the Bitcoin
dependency. The script selects Homebrew LLVM's archiver when available on macOS.

The build compares 311 native/Wasm vectors, exercises u128 limits, sat boundaries,
varint errors, runestone encipher/decipher and cenotaph cases, and runs 10,000
allocation/process/free cycles. The generated evidence includes the module hash,
zero import assertion and observed memory size. Native/Wasm equality establishes
portability of these cases; it is not independent protocol consensus validation.

## Experimental ABI

Allocate UTF-8 JSON using `ord_alloc(length)`, write the bytes into exported memory,
then call `ord_process(pointer, length)`. The returned i64 packs the output length
in the upper 32 bits and the output pointer in the lower 32 bits. JavaScript must
preserve it as BigInt. Reacquire `memory.buffer` after processing. Read the output
and free both allocations using `ord_free(pointer, length)`.

Requests are bounded to 65,536 bytes. Integer domain values use decimal strings.
Allocation pointers and lengths must be preserved exactly; invalid pointers or
double frees are outside this trusted internal ABI and can corrupt guest memory.
There is no hard memory ceiling or OOM recovery guarantee. The raw ABI is a probe,
not a proposed public interface. WIT-generated bindings should replace it before
exposing untrusted callers.

## I/O and further extraction

This protocol subset needs no I/O and therefore imports neither WASI nor custom
host operations. A separate experiment has demonstrated standard WASI P3 for
HTTP, clocks, entropy and filesystem operations, but this wrapper does not claim
that the upstream indexer uses those interfaces.

The unchanged Ord executable fails Wasm compilation through platform dependencies
including `getrandom`/`mio` for unknown-unknown and `polling` for WASI P2. The
historical updater additionally couples native RPC, threads and redb transactions.
An upstream port should extract deterministic transitions and inject their data
and persistence capabilities, preserving reorg and consensus interpretation tests.
