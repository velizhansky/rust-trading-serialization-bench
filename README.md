# rust-trading-serialization-bench

Performance evaluation of serialization protocols for low-latency trading systems.

## Protocols

- JSON (serde_json)
- Bincode (bincode-next)
- Rkyv (zero-copy)
- Protobuf (prost)
- FlatBuffers (generated)

## Build

```bash
cargo build --release
```

## Test

```bash
cargo test
```

## Run

```bash
cargo run
```
