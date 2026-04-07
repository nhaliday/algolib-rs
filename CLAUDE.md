# CLAUDE.md

## Conventions

- Fully qualified names are generally preferred to `use` or `import` statements, with the necessary exceptions (using Rust traits for example).
- Tests are named like "foo_does_X" or "foo_is_X", describing test predicates that could usually serve as complete sentences, sometimes with context.
- proptest tests do not use the `proptest!` macro, because it interferes with code formatting. `TestRunner` is instantiated and run explicitly. `source_file` is filled in the `Config` passed.
