# Match Server (TCP Server)(WIP)
> The actual game does not exist, this project is based on a concept of the essential functionalities of a collectable card game (like MTGA, Heartstone, Legends of Runeterra, etc.).

A lightweight, async game server built in Rust with a custom binary protocol.
Originally designed for a card game that doesn’t exist, this server handles client connections, verifies packets, and broadcasts game state updates with zero fluff and maximum control.

- 🔒 Custom protocol with headers, checksum, and error codes
- 📦 Packet parsing and game state broadcasting
- 📡 Asynchronous TCP server using tokio
- 👥 Client management with per-connection state
- 🛑 Error handling and disconnection fallback logic

This server is spawned when two players are successfully matched by the matchmaking service. Its primary job is to host a single game match between two authenticated players.

Once the server is created, both players will connect and authenticate using their **authentication tokens** issued by the **Player Auth Server**.
### 🛠 Responsibilities:
- **Lua Scripting**: Upon startup, the server loads all Lua scripts used to define card behaviours.
- **Player Authentication**: Verifies both connecting players by contacting the **Player Auth Server** with their tokens.
- **Game State Management**:
    - Initialises and maintains the complete state of the match.
    - Retrieves and caches both players’ deck data using the **Deck Collection Server**.
    - Fetches and stores the detailed card information using the **Card Catalog Server**.
- **Action Handling**:
    - Receives and validates player actions such as playing cards, attacking, and activating effects.
    - Executes card effects by calling embedded Lua scripts.
- **Client Sync**: Periodically broadcasts the current game state to both clients to keep them in sync.
- **Communication**: Integrates with **Synapse-Net** to interact directly with the C++/C# game clients.
### 📡 Protocol Specification
The server uses a custom binary protocol to communicate with clients. Each packet follows this format:
- **Message Type** (1 byte)
- **Payload Checksum** (2 bytes)
- **Message Length** (2 bytes)
- **End Byte** (`0x00`)
The **payload** is encoded using **CBOR** (Concise Binary Object Representation), offering a compact binary alternative to JSON. Payloads are (de)serialised using existing CBOR libraries.
#### 🔗 Connection Flow
1. Client connects to the Match Server.
2. Sends authentication token.
3. Server verifies identity via the **Player Auth Server**.
4. On success, player data is loaded and stored in memory.
#### ♟ Game Flow
Once both players are authenticated:
1. A new match state is initialized.
2. Both players are added to the game state.
3. Game loop begins, including:
    - Receiving and applying player actions.
    - Broadcasting updated game state to both clients at regular intervals.
#### 🧙 Player Action Handling
When a player performs an action (e.g., playing a card, attacking), the server handles it as follows:
##### Playing a Card
- Verify it's the player's turn.
- Confirm the card exists in the player's hand.
- Place the card onto the board (note: card order is not preserved server-side).
- Check if the card has an **on-play** event:
    - If so, locate the associated Lua script and execute it.
- Check all other cards on the board for any **triggered events**, such as:
    - On ally/enemy card played
    - On effect triggered
    - On summon, etc.
### 💀 Disclaimer
This is educational. No encryption, no TLS, no mercy. Use at your own risk
