# CLAUDE.md

## Conventions

- Abbreviations are generally avoided, except for standard ones like DFS, MSB, etc. "Abjad-style" abbreviations, like "evt", "frq", should be avoided. Abbreviation by shortening, like "freq", "ans", "ret", "res", is preferred, when unambiguous, and only for low-scope items like local variables.
- Fully qualified names are generally preferred to `use` or `import` statements, with the necessary exceptions (using Rust traits for example).
- Tests are named like "foo_does_X" or "foo_is_X", describing test predicates that could usually serve as complete sentences, sometimes with context. With this said, it is good if the test names sort related with related and it is acceptable to prefix the sentence with a topic if that would help with the sorting.
- proptest tests do not use the `proptest!` macro, because it interferes with code formatting. `TestRunner` is instantiated and run explicitly. `source_file` is filled in the `Config` passed.
