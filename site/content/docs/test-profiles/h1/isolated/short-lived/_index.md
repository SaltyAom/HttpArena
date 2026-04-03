---
title: Short-lived Connection
---

Same workload as baseline, but each connection is closed and re-established after 10 requests, forcing frequent TCP handshakes.

{{< cards >}}
  {{< card link="implementation" title="Implementation Guidelines" subtitle="Endpoint specification, expected request/response format, and type-specific rules." icon="code" >}}
  {{< card link="validation" title="Validation" subtitle="All checks executed by the validation script for this test profile." icon="check-circle" >}}
{{< /cards >}}
