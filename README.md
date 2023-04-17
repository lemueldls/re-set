# re-set

Regular expression set parser at compile and run time.

## Macro usage

```rust
use re_set_macros::find;

find!(fn find_identifier "[_[:alpha:]][[:word:]]*");

assert_eq!(find_identifier("foo bar"), Some((0, "foo")));
```

```rust
use re_set_macros::find;

find!(pub fn match_string
  // Single quotes
  r"'(\\'|.)*?'",
  // Double quotes
  r#""(\\"|.)*?""#,
  // Multi-line strings
  r"(?s)`(\\`|.)*?`"
);

let (_, string) = match_string(r#""Hello, world!""#).unwrap();

assert_eq!(string, r#""Hello, world!""#);
```

```rust
use re_set_macros::find;

find!(pub(crate) fn next_token
  // 0: Punctuation
  "[[:punct:]]+",
  // 1: Words
  "[[:word:]]+",
  // 2: Whitespace
  "[[:space:]]+"
);

let result = next_token("foo bar");

assert_eq!(result, Some((1, "foo")));

let (index, token) = result.unwrap();

match index {
  0 => println!("Found punctuation: {token}"),
  1 => println!("Found word: {token}"),
  2 => println!("Found whitespace: {token}"),
  _ => unreachable!(),
}
```
