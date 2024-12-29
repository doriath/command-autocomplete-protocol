# Command Autocompletion Protocol (CAP)

## Status

This project is in very early development stage - most features are not
supported yet and the protocol itself is not stabilized yet. All feedback and
suggestions to influence the protocol are very welcome.

## What is Command Autocompletion Protocol?

Implementing high quality autocompletion is hard and has to be repeated for
every shell. This is a problem that affects three groups - the owners of the
CLIs, the owners of frameworks that support argument parsing and the shell
creators.

The goal of this project is to provide a unified protocol for providing
autocompletions, so that each CLI owner has to implement autocompletion just
once, and it will work with all shells that support it. Similarily, shell owners
can implement this protocol and support autocompletions in many CLIs out of
the box.

## How it works?

Command Autocompletion Protocol (CAP) is strongly inspired by [Language
Server Protocol](https://microsoft.github.io/languag-server-protocol).

When user presses tab in the command line, shell starts the appropriate Command
Autocompletion Server (which can be embedded inside the CLI directly, or can be
done by external command) and exchanges messages to provide autocompletions.

The autocomplition is divided into 3 main parts:

1. Shell bridge
2. Router
3. Command Autocompletion Server (or bridge)

**Shell bridge** is required when shell does not support the protocol natively
(which is a case for all shells currently). The bridge consists of two parts -
a script in a shell specific language for providing completions and a binary that
exports the data from the Command Autocompletion Servers in the format most
usable for given shell.

Every CLI will have its own custom server that provides autocomplitions, so we
need a **Router** that will know which server to execute based on given command
that has to be autocompleted. The **Router** is just a Command Autocompletion
Server, that contains a mapping from a CLI name to server to start (e.g. `cas`
-> `cas complete cap`). It then starts the subserver and proxies requests to it.
The router is not strictly required, and can be implemented directly into the
**Shell bridge**.

Last we have the actual Command Autocompletion Servers. Such servers can be
built directly into the CLI itself (the recommendation is to provide `<CLI>
complete cap` command) or can be provided through separate binary. Bridges
are Command Autocompletion Servers that provide completions for many CLIs by
using external completion mechanisms.

## Goals

1. Standardize autocompletions - CLI owners have to support just one autocompletion protocol,
2. Language agnostic - CLI owners have flexibility to implement completions in the language they prefer,
3. Synchronization - it should be simple to keep the completions up to date as the CLI evolves
4. Fast shell startups - avoid per CLI custom shell scripts, that slow down shell startups
5. Support most common usecases (e.g. plugins and nesting)

## Specification

For details, see [specification](docs/specification.md).

## Installation

Currently, we only support [nushell](https://github.com/nushell/nushell) and use 
[carapace](https://carapace.sh/) as a bridge to support many autocompletions out
of the box.

1. Install carapace binary (just the binary, shell integration not required), by
   following  https://carapace.sh/

2. Clone the repository

3. Install the binary:

  ```
  cargo install --path crates/casp
  ```

4. Configure external completion in `nushell`:

  ```nushell
  let casp_completer = {|spans|
    casp nushell -- ...$spans | from json
  }
  $env.config = {
    completions: {
      external: {
        enable: true,
        max_results: 100,
        completer: $casp_completer,
      },
    },
  }
  ```

## Plan

0.1.0 - Initial Proof of concept

- support only nushell
- suport carapace bridge
- basic specification

0.2.0 - MVP

- support bash & zsh (to ensure we did not over fit into nushell)
- basic support for clap crate (through external crate), to make providing native support for CAP easy.
- support plugins
- support nesting
- make carapace optional

## Open questions

- can we provide configuration less setup (where installing new CLI does
  not require any configuration change to support the completions)?
- can we have the autocompletion servers running continuously to provide
  faster completions?

# License

Command Autocompletion Protocol and Server are available as Open Source
Software, under the Apache 2.0 license. See [LICENSE](LICENSE.md) for details
about copyright and redistribution.
