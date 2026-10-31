---
id: 055
title: Make the search latency test robust to a loaded VM without loosening the budget
milestone: M4
status: done
deps: []
touches: [server/tests/search_perf.rs, server/Cargo.toml, Cargo.lock]
spec: ["10. Non-Functional", "Organization"]
---

## Goal
`server/tests/search_perf.rs` still enforces p95 < 100 ms over 100k objects, but it no longer fails just because other jobs are sharing the VM's CPU.

## Acceptance criteria
- [ ] The budget stays 100 ms p95, and the short-query check keeps its current limit. Neither number goes up.
- [ ] Each query is measured as **thread CPU time** of the request (e.g. `libc::clock_gettime(CLOCK_THREAD_CPUTIME_ID)` around a `oneshot` on a current-thread runtime, so the handler runs on the measuring thread) (default — owner may change). Wall-clock p50/p95 are still printed. If thread CPU time can't be read, the test falls back to wall clock with up to 3 full re-runs, and passes if any one run is under budget.
- [ ] Shown to work: run the test once alone and once while `stress-ng --cpu 4` (or `yes > /dev/null` ×4) runs. Both pass, and the PR lists both printed p95 numbers.
- [ ] The PR says how the CPU-time measurement still counts the handler's own work (it isn't just the time of the spawning thread), and shows it with a temporary busy-loop query that fails the test. Remove that query before you commit.
- [ ] Any new dependency (e.g. `libc`) is a `[dev-dependencies]` entry only.
- [ ] `scripts/verify.sh` passes.

## Out of scope
- Changing the search code or ADR 0014.

## Notes for next
- The test uses process CPU time (`CLOCK_PROCESS_CPUTIME_ID`), not thread CPU time. Handlers run their store calls via `http::blocking` (`spawn_blocking`), so a per-thread clock on the requesting thread misses most of the work, even on a current-thread runtime.
- The process clock is accurate only while nothing else runs in the process during a request. Keep `search_perf.rs` to one test on the default single-threaded `#[tokio::test]` runtime.
- The wall-clock fallback (3 full runs, pass if any one is under budget) applies only when `clock_gettime` fails.
