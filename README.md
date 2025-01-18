# Command Autocomplete Protocol (CAP)

## Status

This project is in very early development stage - most features are not
supported yet and the protocol itself is not stabilized yet. All feedback and
suggestions to influence the protocol are very welcome.

## What is the Command Autocomplete Protocol?

Implementing high quality completions is hard and has to be repeated for every
shell. This is a problem that affects three groups - the owners of the CLIs, the
owners of frameworks that support argument parsing and the shell creators.

The goal of this project is to create a unified protocol for providing
command line completions, so that each CLI owner has to implement autocomplete
just once, and it will work with all shells that support it. Similarly, shell
owners can implement this protocol and support completions in many CLIs out of
the box.

## How it works?

Command Autocomplete Protocol (CAP) is strongly inspired by [Language
Server Protocol](https://microsoft.github.io/languag-server-protocol).

When user presses tab in the command line, shell starts the appropriate Command
Autocomplete Server (which can be embedded inside the CLI directly or can be
done by external command) and exchanges messages to provide completions.

The autocompletion is divided into 3 main parts:

1. Shell bridge
2. Router
3. Command Autocomplete Server (or bridge)

**Shell bridge** is required when shell does not support the protocol natively
(which is a case for all shells currently). The bridge consists of two parts -
a script in a shell specific language for providing completions and a binary
that exports the data from the Command Autocomplete Servers in the format most
usable for a given shell.

Every CLI will have its own custom server that provides completions, so we
need a **Router** that will know which server to execute based on given command
for which completions should be provided. The **Router** is just a Command
Autocomplete Server, that contains a mapping from a CLI name to server to start
(e.g. `jj` -> `jj complete`). It then starts the subserver and proxies requests
to it. The router is not strictly required and can be implemented directly into
the **Shell bridge**.

Last, we have the actual Command Autocomplete Servers. Such servers can be built
directly into the CLI itself (the recommendation is to provide `<CLI> complete`
command) or can be provided through separate binary. Bridges are Command
Autocomplete Servers that provide completions for many CLIs by using external
completions mechanisms.

## Goals

1. Standardize completions - CLI owners have to support just one autocomplete protocol,
2. Language agnostic - CLI owners have flexibility to implement completions in the language they prefer,
3. Synchronization - it should be simple to keep the completions up to date as the CLI evolves,
4. Fast shell startups - avoid per CLI custom shell scripts, that slow down shell startups,
5. Support most common use cases (e.g. plugins and nesting).

## Specification

For details, see [specification](docs/specification.md).

## Installation

This repository provides the implementation of the Command Autocomplete Protocol
through `command-autocomplete` binary. This binary currently only supports
[nushell](https://github.com/nushell/nushell) and uses [carapace](https://carapace.sh/)
as a bridge to support many completions out of the box.

1. Install carapace binary (just the binary, shell integration not required), by
   following https://carapace.sh/

2. Clone the repository

3. Install the binary:

  ```
  cargo install --path crates/command-autocomplete
  ```

4. Configure external completions in `nushell`:

  ```nushell
  let cap_completer = {|spans|
    command-autocomplete shell nushell -- ...$spans | from json
  }
  $env.config = {
    completions: {
      external: {
        enable: true,
        max_results: 100,
        completer: $cap_completer,
      },
    },
  }
  ```

## Open questions

- can we provide configuration less setup (where installing new CLI does
  not require any configuration change to support the completions)?
- can we have the autocomplete servers running continuously to provide
  faster completions?

# License

Command Autocomplete Protocol and Server are available as Open Source
Software, under the Apache 2.0 license. See [LICENSE](LICENSE.md) for details
about copyright and redistribution.
