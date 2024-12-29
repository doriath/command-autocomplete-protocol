# Specification - Command Autocompletion Protocol

Version: 0.0.1

## Overview

This document defines a Command Autocompletion Protocol. It is an RPC like
protocol between client (e.g. shell) and server (e.g. CLI providing the
autocompletions).

## Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD",
"SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be
interpreted as described in [RFC 2119](http://www.ietf.org/rfc/rfc4627.txt).

The type definitions are currently defined using typescript. `...` at the end of the
interface means it is `open`, and new fields might be added in the future. The
implementations should not fail if additional fields are set. When `...` is not
present, the interface is `closed`. The implementations should fail if unknown field
is found in `closed` interface. Adding a new field to such struct is considered
a breaking change.

To represent a lack of value, we always use lack of field in the object. No
field should ever be set to `null`. This is chosen as we will want to extend
objects over time with new fields, so all implementations should always handle
new, unknown fields in the `open` interfaces.

TODO: consider if we should allow nulls, and just treat them always the same way
as missing field.

TODO: find if there is a better way to define the protocol that does not depend
on typescript.

## Protocol

The protocol uses https://jsonlines.org - line separated, utf8 json objects.
Every object MUST be a valid `Message` (as defined below).

```typescript
export type Message = Request | Response;
```

```typescript
interface Request {
  id: string;
  method: string;
  params: object;
}
```

```typescript
export type Response = ResponseOk | ResponseError; 

interface ResponseOk {
  id: string;
  result: object;
}

interface ResponseError {
  id: string;
  error: Error;
}

interface Error {
  code: string;
  message: string;
  ...
}
```

### Protocol errors

If the line in the communication protocol is `invalid`, the communication MUST
be closed.

The line is invalid if there is no way to properly respond to such message.
More concretely:

- the line is not a valid utf8
- the line is not a valid JSON object (unparsable)
- the line is not a valid Message
- the `Response.id` does not match any incoming `Request.id`
- the `Request.id` was already received in another request

The following are NOT considered invalid protocol lines:

- the `Request.method` is unknown (error MUST be returned with `INVALID_REQUEST` code)
- the `Request.params` does not match the method (error MUST be returned with `INVALID_REQUEST` code)
- the result contains unexpected result (TODO: what to do here?)

TODO: consider if instead of closing the communication, we should instead
introduce a custom ErrorRequest that should be sent when error was encountered.

## Messages

TODO: figure out if we should always have explicit `initialize` method, to
communicate the version / capabilities of both sides.

### Complete

```typescript
interface CompleteParams {
  args: string[];
}

interface CompleteResult {
  values: CompleteValue[];
}

interface CompleteValue {
  value: string;
  description?: string;
}
```

## Miscellaneus

### Transport

CLIs should use `stdin` and `stdout` to send and receive the requests.
