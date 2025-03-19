# Protocol Header
| Type (1 Byte) | Body Length (2 Bytes) | Check sum (2 Bytes) |

# Connect Player Protocol Body
| Uuid (16 bytes) |
| Nickname (16 bytes) |

# Last byte should be 0x1A



# Server Flow
  With the server instance ready:
  1. Receives the packet
  2. Validates the header
  3. Validates the payload with the checksum
  4. Send payload to the right handler
  5. Handles it
  6. Creates response packet
  7. Sends response
