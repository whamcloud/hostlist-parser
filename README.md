# hostlist-parser

Parses hostlist expressions into a BtreeSet of Strings

This library implements hostlist parsing. It takes a hostlist expression and produces a Result of unique hostnames, or a parse error that can be introspected to see issues.

The fn to parse a hostlist is:

```rust
parse(input: &str,) -> Result<BTreeSet<String>, combine::stream::easy::Errors<char, &str, usize>>
```

This parser can compile to native code and also with the `wasm32-unknown-unknown` target.
