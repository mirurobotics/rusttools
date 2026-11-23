# rusttools

Miru's custom Rust linter, the Rust counterpart of [gotools](https://github.com/mirurobotics/gotools)' lint rules.

## Usage

```sh
cargo run -- --path src --config .lint-imports.toml
```

| Flag | Default | Purpose |
| --- | --- | --- |
| `--path <dir>` | | Files to check for imports and `funclen` |
| `--fix` | off | Rewrite import blocks in place |
| `--config <file>` | nearest `.lint-imports.toml` | Import group labels and internal crates |
| `--funclen-threshold <n>` | 50 | Max code lines per function; 0 disables |
| `--assert-paths <dir>...` | | Test files to check for `field-by-field-assert` |
| `--assert-threshold <n>` | 4 | Field asserts on one receiver that trigger it |

A `// lint:allow(<rule>)` comment on or above the flagged line suppresses `funclen` and `field-by-field-assert`.

## Rules covered by clippy and rustc

Some gotools rules already exist in clippy or rustc, so rusttools doesn't reimplement them. Enable them in each consuming repo:

```toml
# Cargo.toml
[lints.rust]
static_mut_refs = "deny"       # gotools mutableglobal

[lints.clippy]
too_many_arguments = "deny"    # gotools paramcount
unwrap_used = "deny"           # gotools typeassert
expect_used = "deny"           # gotools typeassert
print_stdout = "deny"          # gotools nofmt
print_stderr = "deny"          # gotools nofmt
dbg_macro = "deny"             # gotools nofmt
```

```toml
# clippy.toml
too-many-arguments-threshold = 6
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-print-in-tests = true
allow-dbg-in-tests = true
```

rustfmt covers gotools' `linelen`, `collapse`, `funcsig` and `funcinline`.
