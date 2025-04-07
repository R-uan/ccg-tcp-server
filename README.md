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

# Binary Protocol 

This protocol defines a custom binary format for sending and receiving structured messages over TCP. Each message consists of a fixed-size header followed by a variable-size payload.

## Packet Layout
``` 
+---------+-------------------+-------------+------------+-----------+
| 0x00    | 0x01 - 0x02       | 0x03 - 0x04 | 0x05       | 0x06..N   |
| Type    | Payload Length    | Checksum    | Delimiter  | Payload   |
+---------+-------------------+-------------+------------+-----------+
| 1 byte  | 2 bytes (big-end) | 2 bytes     | 1 byte     | N bytes   |
```

## Checksum

- A 16-bit XOR-based checksum.
- Calculated over the payload:
- Stored as a u16 in the header (checksum field). 

```rust
let mut checksum: u16 = 0;
for byte in payload {
    checksum ^= *byte as u16;
}
```
