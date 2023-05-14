# Protocol types

### Legend

- `1` 1 message
- `*` N messages (~ 1 per client)
- `>>` Server to Client
- `<<` Client to Server
- `[...]` general overview of the objects in the message, for sizing
- `T` type id, the type of the message (u8)
- `SSSS` sequence id, a counter for protocol requirements (u64), also sometimes named message id
- `EEEE` An entity id. (u32+u32 = 64 bit)
- `CC` Component type id, encoded/generated from the initial mapping (u16 ?), meant to avoid sending the component name as a string.

### Protocol messages

These messages are used to complete the protocol.

- `SequenceConfirm` from client to server to confirm receival up to that sequence number of packets.

  `1 << [TSSSS]`

- `SequenceRepeat` from client to server, asking to repeat the message with that sequence id

  `1 << [TSSSS]`

### Entity messages

Entities are immutable IDs so all they need is insert+delete, no update.

  - `EntitySpawn` from client, received on server.  Send entity created from client side to server. Id is the ID of the entity on the client side.
    
    `1 << [TSSSSEEEE]`

  - `EntitySpawnBack`, if client originally created the entity, this is received for that client alone, instead of an EntitySpaw. EntitySpawn will still be send to all the others. EntitySpawn will still be the one stored in the replay queue. This message will have the same sequence id as the EntitySpawn message.
    
    `1 >> [TSSSSEEEEEEEE]`

  - `EntitySpawn` from server, received on client. Send entity created to all clients minus the one that received the EntityCreatedOnClientConfirmation if there was one.
    
    `* >> [TSSSSEEEE]` 

  - `EntityDelete` from client, received on server. Entity ID is the server's id that the client already has.

    `1 << [TSSSSEEEE]`

  - `EntityDelete` from server, to all clients. Entity has been removed, to all clients

    `* >> [TSSSSEEEE]`

### Replay queue

Server keeps a queue of messages for replay. This queue will be emptied up to the lowest sequence number of all clients. Clients update their sequence number to the server using the `SequenceConfirm` message.

### Local queue

Clients keep a local queue of unprocessed messages so they can be picked up by the engine thread.
It's a priority queue, it checks against sequence numbers being sequential, if not then keep the queue stuck and request the missing items.
