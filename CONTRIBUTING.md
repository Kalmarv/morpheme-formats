# Contributing

## Build and test

```sh
cargo build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

The tests use synthetic fixtures built in the test code, so they need no game files. Change
behaviour together with a test that fails without the change; a new refusal needs a test
that checks both its offset and its message.

## Reporting a file it cannot read

Open an issue with the "Unreadable file" template. Include:

- the complete error text (it names the byte offset and what was expected);
- which function refused it, `bundle_be32` or `rig_be32`;
- the game, platform and version, and the name of the file or archive entry;
- the file's size, and a hex dump of the first 128 bytes of the record's payload and of the
  128 bytes around the reported offset.

Do not attach game files or archives, or extracts of them. They are copyrighted, and a hex
dump of a few hundred bytes is enough to start from.
