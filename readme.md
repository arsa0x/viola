# Viola Script

Viola Script is a lightweight scripting language built with Rust. It is designed to be embedded into applications and executed by a custom bytecode virtual machine.

> **Status:** Work in Progress

## Features

- Bytecode compiler
- Stack-based virtual machine
- Native function interface
- Basic value types
  - Integer
  - Float
  - Boolean
  - String
  - Array
  - Object
  - Nil
- Variables
- Arithmetic and comparison operators
- Conditional execution
- Async native function support

## Project Structure

```
src/
├── ast.rs
├── chunk.rs
├── emitter.rs
├── error.rs
├── lexer.rs
├── lib.rs
├── native.rs
├── parser.rs
├── resolver.rs
├── token.rs
└── vm.rs
```

## Example

```vi
@name hello

username = "Viola"

if username == "Viola" {
    :send .text("Hello!")
}
```

## Running Tests

```bash
cargo test
```

## Running Benchmarks

The project uses **Divan** for benchmarking.

```bash
cargo bench
```

Current benchmarks include:

- Lexer
- Parser
- Compiler
- Virtual Machine

## Goals

- Low memory allocation
- Fast lexing through zero-copy string slices
- Small runtime footprint
- Efficient stack-based virtual machine
- Easy embedding into Rust applications
