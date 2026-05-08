# Svix Webhooks / Hashes Demo

This is my submission for the Svix takehome assignment. I'd recently read
about [hexagonal architecture in Rust](https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust)
and liked the patterns, so I decided to go with that, even though it made
me take longer than I'd have liked (about 4h), but I'm happy with the end result.

I used AI only in the form of edit-predictions (Zed AI), and to do an initial
copy of the `domain/webhook_tasks` module to `hash_tasks`, but it honestly
didn't save me any time because I had to fix so much manually.

General flow:

The app uses a `sqlite` database which records a `deadline` for each task,
and a nullable `executed_at` field which starts as NULL but which a timestamp
is written to when the task is completed. The state of a task is `pending`
when the deadline is in the future, `ready` when the deadline is passed but
the entry not yet processed, or `finished` when successfully processed.

There is a `schedule_worker` which is an async state machine that operates
on intervals, there are individually-configurable intervals for Webhook
tasks and Hash tasks. When an interval is triggered, the scheduler queries
the database (via the respective task's service) for any `ready` tasks,
then spawns a tokio-handler-task per webhook/hash-task to handle fulfilling
the task. Webhook tasks use `reqwest` to emit requests (tested successfully
on Svix play), and Hash tasks are spawned with `spawn_blocking` so as not to
block the tokio executor.

I used `async-shutdown`, a favorite crate of mine, to ensure that the system
will shut down gracefully. It has two phases of shutdown: `triggered` and
`complete`. The shutdown will move to "triggered" on user-exit (^C), or if
any essential task (the HTTP server and the ScheduleWorker) quits for any reason.
The shutdown will _not_ move to "completed" until any outstanding worker tasks
all wrap up and complete (enforced by a drop token), ensuring that in-flight
processing is not interrupted. However, after a configurable timeout
(5s by default), the process will still force-quit.

## Getting Started

Make sure you have the `sqlx` CLI installed with the `sqlite` feature:

```
cargo installl sqlx-cli --features sqlite
```

In the project directory, create the database and run the migrations:

```
sqlx database create
sqlx migrate run
```

I recommend setting these variables in a `.env` file:

(`DATABASE_URL` defaults to this value, so it should work regardless)

```
#!/usr/bin/env bash

export RUST_LOG="info"
export DATABASE_URL="sqlite:app.db"
```

Then, build and run the project:

```
cargo run
```

The following command-line options are available:

```
Commands:
  print-deadline <DEADLINE> [e.g.: 60s]
    This optional command will print a compatible timestamp on startup which
    is DEADLINE time in the future, allowing easy copy-paste for testing

Options:
      --database-url <DATABASE_URL>
          [env: DATABASE_URL=] [default: sqlite:app.db]
      --http-port <HTTP_PORT>
          [env: HTTP_PORT=] [default: 8080]
      --shutdown-timeout <SHUTDOWN_TIMEOUT>
          [default: 5s]
      --hash-dispatch-period <HASH_DISPATCH_PERIOD>
          The time between checking for new hash tasks to processs [default: 5s]
      --webhook-dispatch-period <WEBHOOK_DISPATCH_PERIOD>
          The time between checking for new webhook tasks to dispatch [default: 5s]
```

## Status: What's Complete

I got to the end of about 4 hours of working on this and have these features working:

- Create a Webhook task with:

> POST /api/webhook_tasks

```
curl localhost:8080/api/webhook_tasks -d'{"deadline":"2026-05-08T03:09:08.449223Z", "url":"https://play.svix.com/in/e_TJm7JAY08ao9VaaoigLBGdtriX4/","body":"Hello, world"}' -H"Content-Type: application/json"
```

> note: to easily get a future deadline to test with, use `cargo run -- print-deadline 60s`,
> this will print a timestamp in the correct format to stdout on launch

- Create a Hash task:

> POST /api/hash_tasks

```
curl localhost:8080/api/hash_tasks -d'{"deadline":"2026-05-08T03:09:08.449223Z", "secret":"hello"}' -H"Content-Type: application/json"
```

- Get a Webhook task by ID:

> GET /api/webhook_tasks?id=...

```
curl "localhost:8080/api/webhook_tasks?id=8752ba2b-f679-4a24-8b41-3a09e908fe57"
```

- Get a Webhook task by state (`pending`, `ready`, or `finished`)

> GET /api/webhook_tasks?state=...

> note: you'll pretty much never see `ready` unless you query after
> the deadline has passed but before the scheduler has fulfilled the task

```
curl "localhost:8080/api/webhook_tasks?state=pending"
curl "localhost:8080/api/webhook_tasks?state=finished"
```

## Status: What's Incomplete

I didn't have time to get to the following, but with slightly more time it would be trivial to do so:

- Delete Webhook or Hash tasks by ID
- Get Hash tasks by ID
- Salting the hashes

## How to read the codebase

Here's a quick overview of the codebase and where you can find the interesting things:

- I think `src/outbound/schedule_worker.rs` is my favorite part of the codebase, it's the
  part that checks for task deadlines and executes ready tasks. It's modeled as a state
  machine over a finite set of input types, and spawns tokio tasks to fulfill each ready task.

- The HTTP endpoints are in `src/inbound/http`, the `webhook_tasks.rs` endpoints are more complete
  than the Hash endpoints

- HTTP types get converted to Domain types, which live in `src/domain/`, again `webhook_tasks/`
  is more complete than the Hash domain types.

  - The domain contains the Service definitions, which manipulate the domain objects by calling
    into underlying systems, such as a Repository, which is a trait describing domain object
    persistence. In this case a Service and a Repository are in 1:1 correlation, but in more
    complete implementations the Service may also sit on top of other components such as a
    Metrics recording interface.

- Persistence is in `src/outbound/sqlite/`, where domain objects get converted to and from
  database rows. There are some "DTO" conversion types that help with auto-marshalling rows into
  structs when reading from the DB.
