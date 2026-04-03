---
title: Validation
---

The TCP fragmentation profile uses the same `/baseline11` endpoint as the baseline test. Its validation is covered by the [Baseline validation](../../baseline/validation) checks.

No additional validation checks are specific to this profile — the difference is in benchmark behavior (loopback MTU forced to 69 bytes), not in endpoint correctness. The fragmentation is applied externally by the benchmark runner.
