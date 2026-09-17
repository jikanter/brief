---
stack: [Rust 1.98, serde & serde_yaml]
context: [./docs/a<b>.md]
---

# Harden the parser against <script> tags & "quoted" input

## Identity

- The `<brief>` envelope team — owners of A & B.

## Constraints

### Hard
- [`src/**/*.rs`] Reject input containing `<` or `&` unless it is escaped
- Keep `Record<T>` generics intact in error messages

### Soft
- Prefer `&str` over `String` in the "hot" paths

### Ask First
- Changing the <envelope> tag vocabulary

## Sacred
- `src/**/*.rs` — Every Rust source file, including "generated" ones
- `docs/a&b/**` — Ampersand in a path

## Assumptions
- [ ] Input can carry a fenced code block with markup
- [x] Both `'` and `"` survive the round-trip

## Notes

A fenced block with markup in it:

```rust
fn main() {
    if a < b && c > d {
        println!("{}", "<tag attr='v'>");
    }
}
```

## Deliverable
Well-formed XML whose text keeps A & B < C > D and 'single' and "double" quotes intact.
