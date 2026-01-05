# QR Comms Backend

A WebSocket relay server for the QR Comms project. It allows two devices (e.g., a PC and a smartphone) to discover each other via a shared UUID and exchange encrypted data over WebSockets without requiring account creation or persistent storage.

## Features

- High performance (written in Rust :)
- Configurable rate and message size limiting
- Truly epheremal (all state is kept in memory)

## Getting started

Rust and cargo are required. Run, build and test like any other Rust project.

### Docker

A `Dockerfile` is included.

#### Build
```bash
docker build -t qr-comms-backend .
```

#### Run
```bash
docker run -d \
  -p 3000:3000 \
  -e SERVER_PORT=3000 \
  -e MAX_CHANNEL_COUNT=1000 \
  -e RATE_LIMIT_PER_SECOND=10 \
  --name qr-backend \
  qr-comms-backend
```


### Configuration

The application is configured via environment variables. You can set these in a `.env` file or pass them to the Docker container.

| Variable | Description | Default |
| :--- | :--- | :--- |
| `SERVER_PORT` | Port to listen on. | *None (Required)* |
| `MAX_CHANNEL_COUNT` | Maximum number of active concurrent channels. | *None (Required)* |
| `MAX_MESSAGE_SIZE` | Max size of a single WebSocket message (in bytes). | `65536` (64KB) |
| `RATE_LIMIT_BURST` | Maximum burst of messages allowed per connection. | `10` |
| `RATE_LIMIT_PER_SECOND` | Number of messages allowed per second per connection. | `5` |

## API Reference

### `GET /health`

- Liveness probe.
- Returns `OK` (code `200`)

### `GET /ws/{channel_id}`

- WebSocket upgrade endpoint.
- `{channel_id}` is any UUIDv4.
- Behavior:
  - If channel doesn't exist, it is created.
  - If channel exists (1 participant), you join.
  - If channel is full (2 participants) or limits reached, connection closes.
