# Architecture

## Runtime model

`forum-core` owns validation, authentication primitives, identifiers, and domain
types. Runtime crates own transport and I/O only.

- Workers: Rust → `wasm32-unknown-unknown` → workers-rs → D1/KV.
- Native: Rust → PostgreSQL/Redis adapter (next milestone).
- Optional extensions: isolated WebAssembly components described by WIT.

KV is never an authority for permissions, session revocation, locks, or counters.
The Worker may consult a KV hint, but it verifies every active session in D1.
Strongly consistent edge coordination will use Durable Objects.

## Multi-language rule

Rust is the default. A non-Rust module must be independently buildable, must not
receive raw database bindings or secrets, and must communicate through the WIT
interface. This permits Go, C, C++, C#, Python, and other Wasm-producing toolchains
without weakening the core's ownership or security boundaries.

