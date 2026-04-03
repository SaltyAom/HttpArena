---
title: Leaderboard
layout: wide
toc: false
---

<style>
article > h1.hx\:text-center { display: none; }
article > br { display: none; }
article { max-width: 100% !important; }
</style>

<div style="margin-bottom:1.5rem;">
<h1 class="not-prose hx:text-4xl hx:font-bold hx:leading-none hx:tracking-tighter hx:md:text-5xl hx:py-2 hx:bg-clip-text hx:text-transparent hx:bg-gradient-to-r hx:from-gray-900 hx:to-gray-600 hx:dark:from-gray-100 hx:dark:to-gray-400">Leaderboard</h1>

{{< round-selector >}}

<div class="lb-card" style="margin-top:0.75rem; padding:0.35rem;">
<div id="http-version-tabs" style="display:flex; gap:0.35rem;">
<span class="http-ver active" data-ver="composite">Composite</span>
<span class="http-ver" data-ver="h1iso">H/1.1 Isolated</span>
<span class="http-ver" data-ver="h1wk">H/1.1 Workload</span>
<span class="http-ver" data-ver="h2">H/2</span>
<span class="http-ver" data-ver="h3">H/3</span>
<span class="http-ver" data-ver="grpc">gRPC</span>
<span class="http-ver" data-ver="ws">WebSocket</span>
</div>
</div>
<style>
.http-ver {
  flex: 1;
  text-align: center;
  padding: 0.65rem 1.2rem;
  font-size: 0.9rem;
  font-weight: 600;
  color: #64748b;
  cursor: pointer;
  border-radius: 4px;
  background: transparent;
  transition: all 0.2s ease;
  user-select: none;
  letter-spacing: -0.01em;
}
.http-ver:hover { color: #1e293b; background: rgba(255,255,255,0.5); }
.http-ver[data-ver="h1iso"].active { color: #1e40af; background: rgba(59,130,246,0.1); box-shadow: 0 2px 8px rgba(59,130,246,0.15), 0 1px 3px rgba(0,0,0,0.08); font-weight: 700; }
.http-ver[data-ver="h1wk"].active { color: #1d4ed8; background: rgba(37,99,235,0.1); box-shadow: 0 2px 8px rgba(37,99,235,0.15), 0 1px 3px rgba(0,0,0,0.08); font-weight: 700; }
.http-ver[data-ver="h2"].active { color: #92400e; background: rgba(234,179,8,0.12); box-shadow: 0 2px 8px rgba(234,179,8,0.15), 0 1px 3px rgba(0,0,0,0.08); font-weight: 700; }
.http-ver[data-ver="h3"].active { color: #166534; background: rgba(34,197,94,0.12); box-shadow: 0 2px 8px rgba(34,197,94,0.15), 0 1px 3px rgba(0,0,0,0.08); font-weight: 700; }
.http-ver[data-ver="composite"].active { color: #9a3412; background: rgba(249,115,22,0.12); box-shadow: 0 2px 8px rgba(249,115,22,0.15), 0 1px 3px rgba(0,0,0,0.08); font-weight: 700; }
.http-ver[data-ver="grpc"].active { color: #7c3aed; background: rgba(124,58,237,0.12); box-shadow: 0 2px 8px rgba(124,58,237,0.15), 0 1px 3px rgba(0,0,0,0.08); font-weight: 700; }
html.dark .http-ver { color: #64748b; }
html.dark .http-ver:hover { color: #94a3b8; background: rgba(255,255,255,0.03); }
html.dark .http-ver[data-ver="h1iso"].active { color: #60a5fa; background: rgba(59,130,246,0.15); }
html.dark .http-ver[data-ver="h1wk"].active { color: #93c5fd; background: rgba(37,99,235,0.15); }
html.dark .http-ver[data-ver="h2"].active { color: #fbbf24; background: rgba(234,179,8,0.15); }
html.dark .http-ver[data-ver="h3"].active { color: #4ade80; background: rgba(34,197,94,0.15); }
html.dark .http-ver[data-ver="composite"].active { color: #fb923c; background: rgba(249,115,22,0.15); }
.http-ver[data-ver="grpc"].active { color: #7c3aed; background: rgba(124,58,237,0.12); box-shadow: 0 2px 8px rgba(124,58,237,0.15), 0 1px 3px rgba(0,0,0,0.08); font-weight: 700; }
html.dark .http-ver[data-ver="grpc"].active { color: #a78bfa; background: rgba(124,58,237,0.15); }
.http-ver[data-ver="ws"].active { color: #0891b2; background: rgba(8,145,178,0.12); box-shadow: 0 2px 8px rgba(8,145,178,0.15), 0 1px 3px rgba(0,0,0,0.08); font-weight: 700; }
html.dark .http-ver[data-ver="ws"].active { color: #22d3ee; background: rgba(8,145,178,0.15); }
</style>
<script>
(function() {
  var tabs = document.querySelectorAll('.http-ver');
  tabs.forEach(function(tab) {
    tab.addEventListener('click', function() {
      tabs.forEach(function(t) { t.classList.remove('active'); });
      tab.classList.add('active');
      var ver = tab.dataset.ver;
      document.getElementById('lb-h1iso-wrapper').style.display = ver === 'h1iso' ? '' : 'none';
      document.getElementById('lb-h1wk-wrapper').style.display = ver === 'h1wk' ? '' : 'none';
      document.getElementById('lb-h2-wrapper').style.display = ver === 'h2' ? '' : 'none';
      document.getElementById('lb-h3-wrapper').style.display = ver === 'h3' ? '' : 'none';
      document.getElementById('lb-composite-wrapper').style.display = ver === 'composite' ? '' : 'none';
      document.getElementById('lb-grpc-wrapper').style.display = ver === 'grpc' ? '' : 'none';
      document.getElementById('lb-ws-wrapper').style.display = ver === 'ws' ? '' : 'none';
      /* Reset type filters to Production + Tuned active */
      document.querySelectorAll('.lb-type-filter').forEach(function(f) {
        var t = f.dataset.type;
        f.classList.toggle('active', t === 'production' || t === 'tuned');
      });
      document.querySelectorAll('.composite-type-filter').forEach(function(f) {
        var t = f.dataset.type;
        f.classList.toggle('active', t === 'production' || t === 'tuned');
      });
      /* Sync language filters — capture active langs, apply to all, then trigger re-filter */
      var activeLangs = new Set();
      var allActive = false;
      document.querySelectorAll('.lb-lang-filter').forEach(function(f) {
        if (f.classList.contains('active')) {
          if (f.dataset.lang === 'all') allActive = true;
          else activeLangs.add(f.dataset.lang);
        }
      });
      document.querySelectorAll('.lb-lang-filter').forEach(function(f) {
        if (f.dataset.lang === 'all') f.classList.toggle('active', allActive);
        else f.classList.toggle('active', allActive || activeLangs.has(f.dataset.lang));
      });
      /* Trigger re-filter on all visible wrappers */
      if (typeof applyH1IsoFilters === 'function') applyH1IsoFilters();
      if (typeof applyH1WkFilters === 'function') applyH1WkFilters();
      if (typeof renderComposite === 'function') renderComposite();
      if (typeof updateCompositeNote === 'function') updateCompositeNote();
    });
  });
})();
</script>
</div>

<div id="lb-h1iso-wrapper" style="display:none;">
{{< leaderboard-h1-isolated >}}
</div>

<div id="lb-h1wk-wrapper" style="display:none;">
{{< leaderboard-h1-workload >}}
</div>

<div id="lb-h2-wrapper" style="display:none;">
{{< leaderboard-h2 >}}
</div>

<div id="lb-h3-wrapper" style="display:none;">
{{< leaderboard-h3 >}}
</div>

<div id="lb-grpc-wrapper" style="display:none;">
{{< leaderboard-grpc >}}
</div>

<div id="lb-ws-wrapper" style="display:none;">
{{< leaderboard-ws >}}
</div>

<div id="lb-composite-wrapper">
{{< leaderboard-composite >}}
</div>
