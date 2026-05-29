# Tiny Redis-like TCP Server

## Overview

Build a small Redis-inspired key-value server in Rust. The server accepts TCP client connections, parses simple text commands, stores data in memory, and returns text responses.

The goal is not to clone Redis fully. The goal is to practice core Rust concepts in a realistic async systems project while keeping the scope manageable for about one month of hobby programming.

## Learning Goals

This project is designed to practice:

- Async Rust with Tokio
- TCP networking
- Spawning tasks for concurrent clients
- Shared mutable state with `Arc` and async locks
- Command parsing with enums and pattern matching
- Error handling with `Result`
- Ownership and borrowing across async boundaries
- Unit and integration testing
- Clean project structure

## Core Features

### 1. TCP Server

The server should:

- Listen on a configurable local port
- Accept multiple client connections
- Handle each connection independently
- Read line-based commands from clients
- Write line-based responses back to clients

### 2. In-Memory Key-Value Store

The store should support string keys and string values.

Initial commands:

- `SET key value`
- `GET key`
- `DEL key`
- `EXISTS key`
- `PING`

Example session:

```text
PING
PONG

SET name alice
OK

GET name
alice

EXISTS name
1

DEL name
1

GET name
(nil)
```

### 3. Command Parser

Create a parser that converts input strings into command values.

The parser should handle:

- Empty input
- Unknown commands
- Missing arguments
- Extra whitespace
- Case-insensitive command names if desired

Parsing should be testable without starting the TCP server.

### 4. Response Format

Keep the response format simple and human-readable.

Suggested responses:

```text
OK
PONG
(nil)
1
0
ERR unknown command
ERR wrong number of arguments
```

Do not implement the full Redis RESP protocol initially.

## Suggested Architecture

Possible modules:

```text
src/
  main.rs
  command.rs
  store.rs
  server.rs
  error.rs
```

### `command.rs`

Responsible for:

- Defining the command enum
- Parsing input lines into commands
- Unit tests for parsing

### `store.rs`

Responsible for:

- Holding key-value data
- Implementing store operations
- Keeping storage logic separate from networking

### `server.rs`

Responsible for:

- Accepting TCP connections
- Reading commands from clients
- Calling store operations
- Writing responses

### `error.rs`

Optional. Use this if custom error handling becomes useful.

## Milestones

## Week 1: Synchronous Core Logic

Build the non-networking parts first.

Tasks:

- Define a command enum
- Implement command parsing
- Define the in-memory store
- Implement `set`, `get`, `del`, `exists`, and `ping` behavior
- Add unit tests for parser and store

Success criteria:

- Commands can be parsed from strings
- Store behavior works through unit tests
- No TCP server required yet

## Week 2: Basic TCP Server

Add Tokio and networking.

Tasks:

- Start a TCP listener
- Accept a client connection
- Read input line by line
- Parse commands
- Send responses
- Handle one client correctly

Success criteria:

- A terminal client can connect and run commands
- The server responds correctly
- Invalid input does not crash the server

## Week 3: Concurrent Clients

Support multiple clients at the same time.

Tasks:

- Spawn a Tokio task per connection
- Share the store safely across tasks
- Use `Arc` with an async lock
- Handle client disconnects gracefully

Success criteria:

- Multiple clients can read and write shared keys
- Server remains responsive under concurrent clients

## Week 4: Polish and Extensions

Add one or two scoped extensions.

Possible extensions:

- `KEYS` command
- `FLUSHALL` command
- Expiring keys with `SETEX key seconds value`
- Background cleanup task for expired keys
- Simple persistence to disk
- Graceful shutdown on Ctrl+C
- Integration tests using a real TCP connection
- Basic benchmark or load test script

Success criteria:

- Project feels complete and usable
- Code is organized and tested
- README explains how to run and use the server

## Stretch Features

Only add these after the core project works.

- RESP-like protocol support
- Namespaces or databases
- Authentication token
- Append-only persistence log
- Pub/sub channels
- Transactions with `MULTI` / `EXEC`
- Simple replication between two server instances

## Important Scope Rules

Avoid these early:

- Full Redis protocol compatibility
- Complex persistence engine
- Cluster mode
- TLS
- Authentication and permissions
- Advanced eviction policies
- Production-grade performance tuning

This project should stay small enough to finish.

## Recommended Command Semantics

### `PING`

Returns:

```text
PONG
```

### `SET key value`

Stores `value` under `key`.

Returns:

```text
OK
```

### `GET key`

Returns the value if present, otherwise:

```text
(nil)
```

### `DEL key`

Deletes the key if present.

Returns:

```text
1
```

if deleted, otherwise:

```text
0
```

### `EXISTS key`

Returns:

```text
1
```

if present, otherwise:

```text
0
```

## Testing Plan

### Unit Tests

Test:

- Valid command parsing
- Invalid command parsing
- Missing arguments
- Store operations
- Deleting nonexistent keys
- Getting nonexistent keys

### Integration Tests

Later, test:

- Server accepts TCP connections
- Commands work over the network
- Multiple clients share state
- Invalid commands return errors

## Final Deliverable

A Rust CLI application that runs a small async TCP key-value server.

Example usage:

```text
cargo run -- 127.0.0.1:6379
```

Example client usage:

```text
nc 127.0.0.1 6379
```

Then type:

```text
PING
SET language rust
GET language
DEL language
```

## Guiding Principle

Keep the server simple, but design it cleanly.

The real goal is to make Rust concepts click:

- enums for commands
- structs for state
- traits only when abstraction becomes useful
- async tasks for clients
- channels and locks when coordination is needed
- tests to keep the scary parts under control
