---
weight: 2
title: Workload
---

Multi-endpoint benchmarks that exercise multiple code paths simultaneously under realistic conditions.

{{< cards >}}
  {{< card link="mixed" title="Mixed Workload" subtitle="Realistic mix of baseline, JSON, DB, upload, and compression. Weighted scoring rewards heavy endpoint throughput." icon="collection" >}}
  {{< card link="api-4" title="API-4" subtitle="Same mixed workload constrained to 4 CPUs and 16 GB memory. Measures efficiency under limited resources." icon="chip" >}}
  {{< card link="api-16" title="API-16" subtitle="Same API workload with 16 CPUs and 32 GB memory. Tests performance scaling." icon="chip" >}}
  {{< card link="assets-4" title="Assets-4" subtitle="Static files and JSON with conditional gzip compression, constrained to 4 CPUs. Tests asset serving efficiency." icon="photograph" >}}
  {{< card link="assets-16" title="Assets-16" subtitle="Same asset workload with 16 CPUs and 32 GB memory. Tests asset serving scaling." icon="photograph" >}}
{{< /cards >}}
