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

## Releasing

1. Raise the version (a patch, 0.x.Y, for fixes and data or manifest additions; a minor, 0.X.0, for new API or any
   breaking change before 1.0; then `cargo check`, so `Cargo.lock` follows) and put the change's CHANGELOG lines under a
   `## <version> (<date>)` heading, in one commit.
2. Push it to main. Once every check has passed, release-plz publishes each crate whose version crates.io does not have
   yet (trusted publishing: no token is stored anywhere), tags the commit `v<version>` and creates the GitHub Release
   with that CHANGELOG section as its notes.

A commit that raises no version only runs CI. Never release by hand.

A new crate's first version can't come from trusted publishing: the token holder publishes it by hand, then adds its
trusted-publishing config (workflow `ci.yml`).

## Reporting a file it cannot read

Open an issue with the "Unreadable file" template. Include:

- the complete error text (it names the byte offset and what was expected);
- which function refused it, `bundle_be32` or `rig_be32`;
- the game, platform and version, and the name of the file or archive entry;
- the file's size, and a hex dump of the first 128 bytes of the record's payload and of the
  128 bytes around the reported offset.

Do not attach game files or archives, or extracts of them. They are copyrighted, and a hex
dump of a few hundred bytes is enough to start from.
