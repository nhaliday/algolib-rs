# CLAUDE.md

## Conventions

- proptest tests do not use the `proptest!` macro, because it interferes with code formatting. `TestRunner` is instantiated and run explicitly. `source_file` is filled in the `Config` passed.
