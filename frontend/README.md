# QR Comms Frontend

The web UI for QR Comms, built using SvelteKit.

## Getting started

### Prerequisites

- A JavaScript runtime, like Deno or Node.js
- QR Comms backend

### Local development

1. Clone the repository, navigate to `frontend`.
2. Run `deno task dev` with the `PUBLIC_WS_BASE_URL` environment variable set to the URL of your backend server, e.g.:

```bash
PUBLIC_WS_BASE_URL="ws://127.0.0.1:3005/ws" deno task dev
```

3. Build with `deno task build`, setting the same variable.

### Docker/Podman deployment

1. Build with `docker build --build-arg WS_BASE_URL="ws://127.0.0.1:3000/ws" -t qr-comms-frontend .`, replacing the value of `WS_BASE_URL` with the location of your backend.
2. Run using `docker run -p 3000:80 qr-comms-frontend`
