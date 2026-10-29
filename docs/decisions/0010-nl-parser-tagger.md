# 0010 — Smart add ships the tagger parser

- **Status**: accepted
- **Context**: Spec §5 picks each model by benchmark against a baseline. Task 030 compared the rules baseline, rules plus the `interim` date crate, and an averaged-perceptron token tagger (`docs/models/nl-parser.md`).
- **Decision**:
  - `default_parser()` returns `TaggerParser` (90.2% vs 89.0% exact, p99 ~0.1 ms). Callers use only `default_parser()` or the `Parser` trait.
  - The model is bundled as `parse/src/tagger/model.bin` (FNV-1a feature keys, so it doesn't depend on std's hasher), produced by `cargo run -p polykov-parse --release --example bench -- --save-model` from `parse/bench/train.jsonl`. No training at runtime.
  - The tagger only chooses token spans; the rules matchers turn them into values, so output formats and syntax stay those of ADR 0009.
- **Consequences**: Changing rules, features or training data means retraining `model.bin` and re-recording the score; `parse/tests/bench.rs` fails if exact match drops more than 1 point. `RulesParser` stays as baseline and fallback. Learning from user corrections is M11.
