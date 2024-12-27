# Command Autocomplete Server Protocol (CASP)

## Status

This project is in very early development stage - most features are not
supported yet and the protocol itself is not stabilized yet. All feedback and
suggestions to influence the protocol are very welcome.

## What is Command Autocomplete Server Protocol?

Implementing high quality autocompletion is hard and has to be repeated for
every shell. This is a problem that affects three areas - the owners of the
CLIs, the owners of frameworks that support argument parsing and the shell
creators.

The goal of this project is to provide the unified protocol for providing
autocompletions, so that each CLI owner has to implement autocompletion just
once, and it will work with all shells that support it. Similarily, shell owners
can implement this protocol and support autocompletions in many CLIs out of
the box.

## How it works?

Command Autocomplete Server Protocol (CASP) is strongly inspired by [Language
Server Protocol](https://microsoft.github.io/languag-server-protocol).

When user presses tab in the command line, shell starts the appropriate command
autocomplete server and exchanges messages to provide autocompletions.

Some advantages of using autocomplete server:

- the autocomplete server can be implemented directly into the CLI, ensuring
  that autocompletions do not get out of sync with the actual binaries
- the autocompletion can be implemented in the same language as the CLI
- faster shell startup, as it does not need to load custom autocompletion scripts
- faster autocompletion, as data can be cached in memory by the running server

## Specification

Version: 0.0.1

- the communication uses https://jsonlines.org
- similar to JSON RPC with small changes:

  - no need for jsonrpc field,
  - ids are always strings
  - no notifications (every request requires a response)

### Examples

Note: The examples might have new lines inside one json message, to improve
readability in the documentation, but the actual messages can't have them.

Client -> Server

```json
{
  "id": "1",
  "method": "",
  "params": {
    "dir": "/home/USER", 
    "env": {
      "PATH": "/usr/bin/env"
    },
    "args": [
      "casp",
      ""
    ]
  }
}
```

Server -> Client

```json
{
  "id": "1",
  "result": {
    "values": [{
      "value": "--help",
      "display": "--help",
      "description": "Shows the help for the command",
      "style": "",
      "tag": ""
    }]
  }
}
```

## Plan

This project is in experimental state.

Current plans:

- create initial specification
- create multiplexer binary, that will have configuration for supported binaries
- support the multiplexer binary as a bridge in carapace, to support many shells out of the box
- support carapace as a bridge in casp binary

Features:

- simple autocompletion
- hinter

## Open questions

- can we somehow provide configuration less setup (where installing new CLI does
  not require any configuration change to support the completions)?
- how to keep state (can it be done without native integration with a shell ?)
