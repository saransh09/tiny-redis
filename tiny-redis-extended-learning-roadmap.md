# Tiny Redis-like Server: Extended Async Rust Learning Roadmap

## Project Goal

Build a small Redis-inspired TCP key-value server in Rust as a practical async/concurrency learning lab.

The goal is not production Redis compatibility. The goal is to progressively encounter and understand core Rust and async Rust concepts through increasingly realistic server features.

## Guiding Principle

Each milestone should teach one or two major ideas. Avoid adding features only because Redis has them. Prefer features that force you to practice ownership, async control flow, shared state, task coordination, cancellation, testing, and design trade-offs.

---

## Milestone 1: Synchronous Core Logic

### Concepts

- Enums
- Pattern matching
- `Result`
- `HashMap`
- Unit testing
- Separating parsing from execution

### Build

- `Command` enum
- Parser for line-based commands
- In-memory `Store`
- Basic commands:
  - `PING`
  - `SET key value`
  - `GET key`
  - `DEL key`
  - `EXISTS key`

### Success Criteria

- Parser works without starting a TCP server
- Store behavior is covered by unit tests
- Invalid commands return useful errors

---

## Milestone 2: Single-Client Async TCP Server

### Concepts

- Tokio runtime
- `async fn`
- TCP listener
- Async reading/writing
- `BufReader`
- Line-based protocols

### Build

- Start a TCP listener
- Accept one client
- Read commands line by line
- Parse commands
- Execute commands against the store
- Write responses back to the client

### Success Criteria

- `cargo run -- 127.0.0.1:6379` starts the server
- `nc 127.0.0.1 6379` can connect
- Commands work over TCP
- Client disconnects are handled gracefully

---

## Milestone 3: Concurrent Clients with Shared State

### Concepts

- `tokio::spawn`
- `async move`
- `Arc`
- `tokio::sync::Mutex`
- Shared mutable state
- Task-per-connection architecture
- Avoiding lock misuse

### Build

- Accept clients in a loop
- Spawn one task per client
- Share one store across all client tasks
- Use `Arc<Mutex<Store>>`

### Success Criteria

- Multiple clients can connect at once
- One client can `SET` a key and another client can `GET` it
- Locks are not held during network writes
- Integration test proves two clients share state

---

## Milestone 4: Better Protocol Surface and Parser Design

### Concepts

- Cleaner domain errors
- Error enums
- User-facing protocol errors vs internal errors
- Parser ergonomics
- More thorough testing

### Build

- Replace `Result<Command, String>` with a custom `ParseError`
- Convert parser errors into response strings separately
- Add commands:
  - `KEYS`
  - `FLUSHALL`
- Improve argument validation
- Decide how to handle values with spaces, for example:
  - keep them unsupported for now
  - or parse `SET key rest of line` as value

### Success Criteria

- Parser errors are typed
- Tests assert error variants, not only strings
- Client still receives simple text errors
- New commands work through TCP

---

## Milestone 5: Graceful Shutdown and Task Coordination

### Concepts

- `tokio::select!`
- `tokio::signal::ctrl_c`
- Cancellation
- Breaking async loops
- Task lifecycle management

### Build

- Listen for Ctrl+C
- Stop accepting new clients when shutdown starts
- Print a clean shutdown message
- Optionally notify existing client tasks

### Success Criteria

- Ctrl+C exits the server cleanly
- The accept loop stops intentionally
- The server does not panic on shutdown

### Stretch

- Use `tokio::sync::broadcast` to notify active clients
- Let clients receive a final shutdown message
- Track spawned client tasks and wait for them briefly

---

## Milestone 6: Client Timeouts and Cancellation

### Concepts

- `tokio::time::timeout`
- Idle connection handling
- Future cancellation
- Resource cleanup

### Build

- Disconnect clients that are idle for a configured duration
- Return a timeout message before disconnecting, if desired
- Make timeout duration configurable

### Success Criteria

- Idle clients are disconnected automatically
- Active clients continue working
- Timeout behavior is covered by an async test

---

## Milestone 7: Expiring Keys and Background Tasks

### Concepts

- Background async tasks
- `tokio::time::interval`
- Time-based state
- Lock contention
- Separating storage model from command handling

### Build

- Add expiring key support:
  - `SETEX key seconds value`
  - `TTL key`
- Store expiration metadata
- Spawn a cleanup task that periodically removes expired keys

### Success Criteria

- `SETEX session 2 abc` stores a key with expiry
- `TTL session` returns remaining lifetime
- Expired keys disappear automatically
- Background cleanup does not block clients unnecessarily

### Design Question

Decide whether expired keys should be removed:

- lazily when accessed
- eagerly by the cleanup task
- both

Implementing both is a good learning exercise.

---

## Milestone 8: Configuration and CLI Polish

### Concepts

- CLI argument parsing
- Configuration structs
- Defaults
- Separating app setup from runtime logic

### Build

- Support options such as:
  - bind address
  - idle timeout
  - cleanup interval
  - optional maximum number of clients
- Start simple with `std::env::args`
- Optionally use a CLI crate later

### Success Criteria

- Server can be configured from command-line arguments
- Defaults are sensible
- README documents how to run the server

---

## Milestone 9: Observability and Debuggability

### Concepts

- Structured logging
- Runtime visibility
- Debugging concurrent systems
- Operational thinking

### Build

- Add logs for:
  - startup
  - client connect/disconnect
  - parse errors
  - shutdown
  - cleanup task activity
- Optionally add a `STATS` command returning:
  - number of keys
  - number of connected clients
  - uptime

### Success Criteria

- You can understand what the server is doing from logs
- `STATS` gives useful runtime information
- Logs are not excessively noisy

---

## Milestone 10: Actor-Model Store Rewrite

### Concepts

- Message passing
- `tokio::sync::mpsc`
- `tokio::sync::oneshot`
- Single-owner state
- Avoiding shared mutable state
- Comparing concurrency architectures

### Build

Create a separate branch and replace:

```text
Arc<Mutex<Store>>
```

with:

```text
client task -> mpsc command channel -> store task -> oneshot response
```

Each client task sends requests to a dedicated store task. The store task owns the `Store` directly and processes messages sequentially.

### Success Criteria

- Clients still share state
- No `Mutex<Store>` is needed
- Each command gets a response through a `oneshot` channel
- You can explain the trade-off between lock-based shared state and actor-based state

### Key Learning Question

Which version is easier to reason about?

- `Arc<Mutex<Store>>`
- actor task with channels

There is no universally correct answer. The point is to experience both.

---

## Milestone 11: Bounded Channels, Backpressure, and Load Behavior

### Concepts

- Bounded vs unbounded channels
- Backpressure
- Overload behavior
- Fairness
- Failure modes

### Build

- Use a bounded `mpsc` channel for store requests
- Decide what happens when the channel is full:
  - wait
  - timeout
  - return an error
- Add a simple load script or test that opens many clients

### Success Criteria

- Server behavior under load is intentional
- Overload does not cause unbounded memory growth
- You can observe and explain backpressure

---

## Milestone 12: Persistence Experiment

### Concepts

- Async file I/O
- Durability trade-offs
- Append-only logs
- Startup recovery
- Serialization format choices

### Build

- Add a simple append-only file for write commands:
  - `SET`
  - `DEL`
  - `FLUSHALL`
  - `SETEX`
- Replay the file at startup to rebuild the store
- Keep the format simple and human-readable

### Success Criteria

- Data survives server restart
- Startup replay restores the store
- Corrupt or unknown log lines do not crash the server

### Stretch

- Add log compaction
- Add a `SAVE` command
- Compare sync vs async file writes

---

## Milestone 13: Integration Test Harness

### Concepts

- Async integration testing
- Spawning servers in tests
- Dynamic ports
- Test isolation
- End-to-end verification

### Build

- Add tests under `tests/`
- Start the server on `127.0.0.1:0`
- Connect real TCP clients
- Test command behavior end to end
- Test concurrent clients
- Test timeouts and shutdown if practical

### Success Criteria

- End-to-end tests are reliable
- Tests do not require a fixed port
- Tests clean up after themselves

---

## Milestone 14: Protocol Upgrade Experiment

### Concepts

- Protocol design
- Framing
- Parsing byte streams
- Partial reads
- Compatibility trade-offs

### Build

Choose one:

1. Improve your line-based protocol
2. Add a tiny RESP-like protocol subset

For a RESP-like subset, support:

- simple strings
- errors
- bulk strings
- arrays for commands

### Success Criteria

- Parser can handle partial input correctly
- Protocol parsing is tested independently from networking
- You understand why real protocols need framing

---

## Milestone 15: Performance and Lock Contention Exploration

### Concepts

- Measuring before optimizing
- Lock contention
- Read/write locks
- Sharding
- Throughput vs latency

### Build

- Add a basic benchmark or load script
- Compare:
  - `Mutex<Store>`
  - `RwLock<Store>`
  - sharded store with multiple locks
  - actor-model store

### Success Criteria

- You can measure rough throughput
- You can explain where contention appears
- You avoid premature optimization, but understand the trade-offs

---

## Suggested Learning Branches

Use Git branches to experiment freely:

```bash
git checkout -b graceful-shutdown
git checkout -b setex-expiry
git checkout -b actor-store
git checkout -b persistence-log
git checkout -b resp-protocol
```

Branches are useful because some experiments may get messy. That is expected and valuable.

---

## Recommended Order

If your main goal is async Rust learning, the best sequence is:

1. Finish shared-state concurrency tests
2. Add graceful shutdown
3. Add client timeouts
4. Add expiring keys with a background cleanup task
5. Try the actor-model rewrite
6. Add bounded channels and backpressure
7. Add persistence
8. Add protocol framing or RESP-like parsing
9. Explore performance trade-offs

---

## Things to Avoid Too Early

Avoid these until the concurrency lessons are solid:

- Full Redis compatibility
- Cluster mode
- TLS
- Authentication and permissions
- Production-grade persistence
- Advanced eviction policies
- Complex benchmarking frameworks
- Premature abstraction with traits everywhere

---

## Final Outcome

By the end, this project should teach you:

- how async Rust tasks are spawned and coordinated
- when to use shared state with locks
- when to use channels and actors
- how cancellation and timeouts work
- how to manage background tasks
- how to test async TCP systems
- how protocol design affects parser complexity
- how to reason about backpressure and overload

The final server does not need to be production-ready. It should be a well-structured playground where async Rust concepts become concrete.