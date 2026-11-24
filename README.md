# rusttools

Miru's custom Rust linter, the Rust counterpart of [gotools](https://github.com/mirurobotics/gotools)' lint rules.

## Usage

The crate builds one binary, `lint-imports`. Consuming repos install it pinned to a commit and run it on their sources:

```sh
cargo install --quiet --locked --git https://github.com/mirurobotics/rusttools --rev <sha> lint-imports
lint-imports --path src --config .lint-imports.toml
```

Inside this repo, `cargo run -- <flags>` runs the same binary.

| Flag | Default | Purpose |
| --- | --- | --- |
| `--path <dir>` | | Files to check for imports, `funclen`, `filelen`, `nestdepth` and `todoissue` |
| `--fix` | off | Rewrite import blocks in place |
| `--config <file>` | nearest `.lint-imports.toml` | Import group labels and internal crates |
| `--funclen-threshold <n>` | 50 | Max code lines per function; 0 disables |
| `--filelen-threshold <n>` | 300 | Max code lines per file, tests included; 0 disables |
| `--nestdepth-threshold <n>` | 2 | Max nested control-flow blocks (`if`/`else`, `match`, loops) in a function or closure body, tests included: `else if`, match arms and `let`-`else` add no depth, plain `else` contents count like the then-branch, and closures and `async` blocks start at depth 0; 0 disables |
| `--assert-paths <dir>...` | | Test files to check for `field-by-field-assert` |
| `--assert-threshold <n>` | 4 | Field asserts on one receiver that trigger it |

To suppress a rule:

- `funclen`: put `// lint:allow(funclen)` on the line with the `fn` keyword (or a closure's opening `|`), or on the line directly above it.
- `filelen`: put `// lint:allow(filelen)` on the file's first line.
- `nestdepth`: put `// lint:allow(nestdepth)` on the reported `if`, `match` or loop line, or on the line directly above it.
- `todoissue`: put `// lint:allow(todoissue)` in the same run of comment lines as the `TODO` or `FIXME`, written in the same style (`//`, `///` or `//!`).
- `field-by-field-assert`: put `// lint:allow(field-by-field-assert)` anywhere from the line with the test body's opening brace through the line with its closing brace.

## Rules covered by clippy and rustc

Some gotools rules already exist in clippy or rustc, so rusttools doesn't reimplement them. rusttools enforces them on itself; copy the `[lints.rust]` and `[lints.clippy]` tables from its [`Cargo.toml`](Cargo.toml) and its [`clippy.toml`](clippy.toml) into each consuming repo. In a workspace, put them under `[workspace.lints.rust]` and `[workspace.lints.clippy]` in the root manifest and add `lints.workspace = true` to each member; cargo ignores a `[lints]` table in a virtual manifest.

| gotools rule | Lint |
| --- | --- |
| `mutableglobal` | `static_mut_refs` (in part) |
| `paramcount` | `too_many_arguments` |
| `typeassert` | `unwrap_used`, `expect_used` |
| `nofmt` | `print_stdout`, `print_stderr`, `dbg_macro` |

`static_mut_refs` only rejects references to a `static mut`; declaring and assigning one, and interior-mutable statics such as `static Mutex<T>`, still compile, so it covers `mutableglobal` only in part.

rustfmt covers gotools' `collapse`, `funcsig` and `funcinline`, and `linelen` only in part: it leaves long string literals, comments and some macro bodies unbroken.
