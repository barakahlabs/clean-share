# link-cleaner

Finds tracking parameters in links inside shared text and removes them. Used by the app's
`clean_text` command to highlight tracking in red and share the cleaned link.

- `analyze(text)` uses the rules embedded at build time from [`rules/`](../../rules).
- `analyze_with(&rules, text)` uses any other `RuleSet`, e.g. a newer downloaded bundle.
- `check_rule_tests(&rules)` runs every `[[test]]` in the rules.

Which parameters are removed, and why, is documented in [`rules/README.md`](../../rules/README.md).
The rule types and validation live in [`crates/link-rules`](../link-rules), and
[`crates/rules-tool`](../rules-tool) checks, documents and bundles the rules.
