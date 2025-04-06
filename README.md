## CCG Game Server (WIP)

> The actual game does not exist, this project is based on a concept of the essential functionalities of a collectible card game (like MTGA, Heartstone, Legends of Runeterra, etc.).

A lightweight, async game server built in Rust with a custom binary protocol.
Originally designed for a card game that doesn’t exist, this server handles client connections, verifies packets, and broadcasts game state updates with zero fluff and maximum control.

- 🔒 Custom protocol with headers, checksums, and error codes
- 📦 Packet parsing and game state broadcasting
- 📡 Asynchronous TCP server using tokio
- 👥 Client management with per-connection state
- 🛑 Error handling and disconnection fallback logic

## 🧪 Status

Still early. Basic client-server handshake is in. <br>
Protocol is defined. Game logic is next.<br>
Nothing’s production-ready—yet.<br>

## 💀 Disclaimer

This is educational. No encryption, no TLS, no mercy.
Use at your own risk.

## Protocol
``` 
[Protocol Header (Fixed 6 bytes)]
[COBOR-encoded payload]
```