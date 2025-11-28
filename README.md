# grust

A procedural macro for the grust language.

## Install

1. Install `rustc` and `cargo` (instructions: https://www.rust-lang.org/tools/install)

    ```text
    > curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```

2. Restart terminal
3. Retrieve `grust` repository

    ```text
    > git clone https://github.com/langrust/grust.git
    ```

## Build

Retrieve the creusot dependency:

```text
> git submodule update --init
```

Use cargo to build everything:

```text
# To run with macro diagnostics and Creusot:
# Need nightly or provided rust-toolchain
> cargo build
[...]

# To run without macro diagnostics nor Creusot:
# Doesn't need nightly nor provided rust-toolchain
> cargo build --no-default-features
[...]

# To run tests:
> cargo test --all
[...]
```
