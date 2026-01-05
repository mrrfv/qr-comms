# QR Comms

Quickly and safely share a link, password or message between your PC and phone using just a QR code.

## Features

- Real-time communication using a WebSocket relay server
- End-to-end encryption with a key shared out of band
- Automatic detection of URLs in messages
- Works on any device with internet access, no Bluetooth or pairing needed

## How it works

1. Open the app on your PC to create a secure, temporary channel and a corresponding QR code.
2. Scan the code with your phone's camera to join the channel.
3. Send text or links back and forth. The channel is encrypted and destroyed once both parties disconnect.

## Project structure

- `frontend/`: A Svelte-based web interface.
- `backend/`: A simple Rust relay server.
