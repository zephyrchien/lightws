# Lightws

[![Released API docs](https://docs.rs/lightws/badge.svg)](https://docs.rs/lightws)

Lightweight websocket implement for stream transmission.

## Features

- Avoid heap allocation.
- Avoid buffering frame payload.
- Use vectored-io if available.
- Transparent Read/Write over the underlying IO source.

## High-level API

Stream:

```rust
{
    // establish connection, handshake
    let stream = ...
    // read some data
    stream.read(&mut buf);
    // write some data
    stream.write(&buf);
}
```

## Low-level API

FrameHead(Fin, OpCode, Mask, PayloadLen):

```rust
{
    // encode a frame head
    let head = FrameHead::new(...);
    let offset = unsafe {
        head.encode_unchecked(&mut buf);
    }

    // decode a frame head
    let (head, offset) = FrameHead::decode(&buf).unwrap();
}
```

Handshake:

```rust
{
    // make a client handshake request
    let mut custom_headers = HttpHeader::new_storage();
    let request = Request::new(&mut custom_headers);
    let offset = request.encode(&mut buf).unwrap();

    // parse a server handshake response
    let mut custom_headers = HttpHeader::new_storage();
    let mut response = Response::new(&mut custom_headers);
    let offset = response.decode(&buf).unwrap();
}
```
