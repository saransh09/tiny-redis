# Async Rust Learning Notes: Graceful Shutdown, Broadcast Channels, and Task Lifecycles

## Context

This note explains the graceful shutdown pattern used in the tiny Redis-like async TCP server.

The server has:

- one task accepting TCP clients
- one task per connected client
- shared state using `Arc<Mutex<Store>>`
- graceful shutdown using `tokio::signal::ctrl_c`
- client notification using `tokio::sync::broadcast`
- task tracking using `JoinHandle`

The main lesson:

> In async Rust, it is not enough to pass data around. You also need to understand who owns the thing that keeps the system alive.

In this case, that thing is the `broadcast::Sender`.

---

## The Problem: Graceful Shutdown

The goal is:

```text
Server receives Ctrl+C
        ↓
Server tells all connected clients:
"SERVER shutting down"
        ↓
Each client exits cleanly
        ↓
Server waits briefly for client tasks
        ↓
Process exits
```

Without graceful shutdown, Ctrl+C simply kills the process. That works, but it does not teach much about async task coordination.

---

## Broadcast Channels

Tokio's `broadcast` channel lets one sender notify many receivers.

Create a channel:

```rust
let (shutdown_tx, _) = broadcast::channel::<()>(1);
```

- `shutdown_tx` is the sender.
- Receivers are created by calling `.subscribe()`.

Example:

```rust
let shutdown_rx_1 = shutdown_tx.subscribe();
let shutdown_rx_2 = shutdown_tx.subscribe();
let shutdown_rx_3 = shutdown_tx.subscribe();
```

Conceptually:

```text
shutdown_tx
   ├── shutdown_rx_1
   ├── shutdown_rx_2
   └── shutdown_rx_3
```

When the server sends:

```rust
let _ = shutdown_tx.send(());
```

all active receivers can observe the shutdown signal.

---

## Why Send `()`?

The channel type is:

```rust
broadcast::channel::<()>(1)
```

The type `()` is Rust's unit type. It carries no meaningful data.

That is fine because the event itself is the message.

This:

```rust
shutdown_tx.send(());
```

means:

```text
Shutdown now.
```

There is no need to send a string like `"shutdown"`.

---

## Why Each Client Needs a Receiver

Each connected client runs in its own task:

```rust
tokio::spawn(async move {
    handle_client(stream, store, shutdown_rx).await
});
```

The client task normally waits for commands from the socket:

```rust
reader.read_line(&mut line).await
```

But for graceful shutdown, it must also listen for a shutdown signal.

So each client needs its own receiver:

```rust
let shutdown_rx = shutdown_tx.subscribe();
```

---

## `tokio::select!`

Inside `handle_client`, the task waits for two possible events:

1. the client sends a command
2. the server sends a shutdown signal

This is done with `tokio::select!`:

```rust
tokio::select! {
    result = reader.read_line(&mut line) => {
        // client sent a command
    }

    result = shutdown_rx.recv() => {
        // server is shutting down
    }
}
```

`tokio::select!` means:

> Run these async operations at the same time. Whichever completes first wins.

Mental model:

```text
handle_client waits at a fork:

      ┌── command arrives?
wait ─┤
      └── shutdown signal arrives?

whichever happens first is handled
```

This is one of the central async Rust patterns.

---

## What Happens When All Senders Are Dropped?

Important rule:

> A channel is alive as long as at least one sender exists.

If all `broadcast::Sender`s are dropped, receivers see the channel as closed.

That means this:

```rust
shutdown_rx.recv().await
```

returns:

```rust
Err(broadcast::error::RecvError::Closed)
```

If the server treats `Closed` as shutdown, then the client will send:

```text
SERVER shutting down
```

This is reasonable behavior in production, but it can surprise tests if the sender is accidentally dropped too early.

---

## The Test Bug: Sender Dropped Too Early

A failing helper looked like this:

```rust
async fn start_test_client() -> (
    BufReader<tokio::net::tcp::OwnedReadHalf>,
    tokio::net::tcp::OwnedWriteHalf,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let store = Arc::new(Mutex::new(Store::new()));
    let (_shutdown_tx, shutdown_rx) = broadcast::channel(1);

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        handle_client(stream, store, shutdown_rx).await.unwrap();
    });

    let stream = TcpStream::connect(addr).await.unwrap();
    let (reader, writer) = stream.into_split();

    (BufReader::new(reader), writer, server_task)
}
```

The problem is:

```rust
let (_shutdown_tx, shutdown_rx) = broadcast::channel(1);
```

`_shutdown_tx` is not returned from the helper. When the helper returns, `_shutdown_tx` is dropped.

Then `shutdown_rx.recv()` inside the client handler sees the channel is closed and immediately wakes up with `RecvError::Closed`.

So tests expected:

```text
PONG
```

but received:

```text
SERVER shutting down
```

---

## The Fix: Return the Sender and Keep It Alive

Fixed helper:

```rust
async fn start_test_client() -> (
    BufReader<tokio::net::tcp::OwnedReadHalf>,
    tokio::net::tcp::OwnedWriteHalf,
    tokio::task::JoinHandle<()>,
    broadcast::Sender<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let store = Arc::new(Mutex::new(Store::new()));
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        handle_client(stream, store, shutdown_rx).await.unwrap();
    });

    let stream = TcpStream::connect(addr).await.unwrap();
    let (reader, writer) = stream.into_split();

    (BufReader::new(reader), writer, server_task, shutdown_tx)
}
```

Then the test keeps the sender alive:

```rust
let (mut reader, mut writer, server_task, _shutdown_tx) = start_test_client().await;
```

The variable is named `_shutdown_tx` because it is intentionally unused, but it still stays alive until the end of the scope.

---

## `_name` vs `_`

These are different.

This keeps the value alive:

```rust
let _shutdown_tx = shutdown_tx;
```

This drops the value immediately:

```rust
let _ = shutdown_tx;
```

Likewise, this keeps the fourth returned value alive:

```rust
let (_, _, _, _shutdown_tx) = start_test_client().await;
```

But this drops the fourth returned value immediately:

```rust
let (_, _, _, _) = start_test_client().await;
```

The leading underscore in `_shutdown_tx` only suppresses the unused variable warning. It does not mean the value is dropped immediately.

---

## The Two-Client Test Bug

A failing two-client test used this pattern:

```rust
for _ in 0..2 {
    let (_shutdown_tx, shutdown_rx) = broadcast::channel(1);

    tokio::spawn(async move {
        handle_client(stream, store, shutdown_rx).await.unwrap();
    });
}
```

This creates a new shutdown channel inside each loop iteration.

Then `_shutdown_tx` is dropped at the end of the loop iteration.

So each client immediately sees a closed channel.

Bad mental model:

```text
loop iteration 1:
  create tx1/rx1
  spawn client with rx1
  drop tx1
  client sees closed channel

loop iteration 2:
  create tx2/rx2
  spawn client with rx2
  drop tx2
  client sees closed channel
```

---

## Correct Pattern for Many Clients

Create one sender outside the loop:

```rust
let (shutdown_tx, _) = broadcast::channel(1);
```

Then each client subscribes to that shared sender:

```rust
for _ in 0..2 {
    let shutdown_rx = shutdown_tx.subscribe();

    tokio::spawn(async move {
        handle_client(stream, store, shutdown_rx).await.unwrap();
    });
}
```

Good mental model:

```text
one shared sender:
  shutdown_tx

many receivers:
  client1_rx
  client2_rx
  client3_rx
```

---

## Why Clone `shutdown_tx`?

When using `tokio::spawn(async move { ... })`, values used inside the spawned task are moved into it.

If the outer test still needs to keep a sender alive, clone the sender first:

```rust
let (shutdown_tx, _) = broadcast::channel(1);
let server_shutdown_tx = shutdown_tx.clone();
```

Then move the clone into the server task:

```rust
let server_task = tokio::spawn(async move {
    for _ in 0..2 {
        let (stream, _) = listener.accept().await.unwrap();
        let shutdown_rx = server_shutdown_tx.subscribe();

        tokio::spawn(async move {
            handle_client(stream, store, shutdown_rx).await.unwrap();
        });
    }
});
```

Now there are two sender handles:

```text
shutdown_tx          lives in the test
server_shutdown_tx   lives in the server task
```

They point to the same channel.

The channel remains alive as long as at least one sender exists.

---

## Similarity to `Arc::clone`

This pattern is similar to cloning an `Arc`:

```rust
let store = Arc::new(Mutex::new(Store::new()));
let server_store = Arc::clone(&store);
```

`Arc::clone(&store)` does not clone the entire `Store`.

It clones a pointer to the same store.

Similarly:

```rust
let server_shutdown_tx = shutdown_tx.clone();
```

does not create a separate shutdown system.

It creates another sender handle to the same broadcast channel.

---

## `tokio::spawn(async move { ... })`

This creates a new concurrent task:

```rust
tokio::spawn(async move {
    handle_client(stream, store, shutdown_rx).await.unwrap();
});
```

The `move` keyword means:

> Move ownership of captured variables into this async block.

Usually, this is required because the task may outlive the current function.

The task must own what it uses:

```rust
stream
store
shutdown_rx
```

---

## `JoinHandle`

`tokio::spawn` returns a `JoinHandle`:

```rust
let task = tokio::spawn(async move {
    // async work
});
```

A `JoinHandle` lets you wait for the task to finish:

```rust
task.await.unwrap();
```

In the server, client tasks can be tracked:

```rust
let mut client_tasks = Vec::new();

let task = tokio::spawn(async move {
    handle_client(stream, store, shutdown_rx).await
});

client_tasks.push(task);
```

On shutdown, the server can wait for them.

---

## `timeout`

Waiting forever during shutdown is risky. A client task might never finish.

Tokio provides:

```rust
tokio::time::timeout
```

Example:

```rust
timeout(Duration::from_secs(5), task).await
```

This means:

> Wait for the task, but only for five seconds.

Possible outcomes:

```rust
Ok(join_result)
```

The task finished in time.

```rust
Err(_)
```

The timeout elapsed.

This prevents shutdown from hanging forever.

---

## Full Graceful Shutdown Flow

### Normal runtime

```text
run()
  creates store
  creates shutdown_tx
  accepts clients forever

client connects
  clone Arc store
  create shutdown_rx from shutdown_tx
  spawn handle_client task
```

### Client task

```text
handle_client()
  loop:
    wait for either:
      - command from client
      - shutdown signal

    if command:
      parse
      execute
      respond

    if shutdown:
      write "SERVER shutting down"
      break
```

### Ctrl+C

```text
Ctrl+C arrives

run() select! chooses ctrl_c branch
  break accept loop
  shutdown_tx.send(())
  wait for client tasks
  exit
```

---

## Cheat Sheet

### One sender, many clients

```rust
let (shutdown_tx, _) = broadcast::channel(1);

let rx1 = shutdown_tx.subscribe();
let rx2 = shutdown_tx.subscribe();
```

### Keep sender alive in tests

```rust
let (reader, writer, task, _shutdown_tx) = start_test_client().await;
```

### Do not create and drop a channel inside the client loop

Bad:

```rust
for _ in 0..2 {
    let (_tx, rx) = broadcast::channel(1);
}
```

Good:

```rust
let (tx, _) = broadcast::channel(1);

for _ in 0..2 {
    let rx = tx.subscribe();
}
```

### Clone handles before moving into tasks

```rust
let tx_for_task = tx.clone();

let task = tokio::spawn(async move {
    let rx = tx_for_task.subscribe();
});
```

### `_name` keeps a value alive, `_` discards it

Keeps alive:

```rust
let _shutdown_tx = shutdown_tx;
```

Discards immediately:

```rust
let _ = shutdown_tx;
```

---

## Core Takeaway

If there is one thing to remember:

> In Tokio channel-based coordination, keep the sender alive for as long as receivers should keep waiting.

This bug was not just a syntax issue. It was a real concurrency lifecycle lesson involving ownership, dropping, task lifetimes, channels, and async coordination.