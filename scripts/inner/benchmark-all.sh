#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

FRAMEWORKS_DIR="$ROOT_DIR/frameworks"
BENCHMARK="$SCRIPT_DIR/benchmark.sh"
LOG_DIR="$ROOT_DIR/results/logs"
mkdir -p "$LOG_DIR"

# Frameworks to skip (already benchmarked)
SKIP_LIST="actix"

# Collect enabled frameworks from meta.json
frameworks=()
for meta in "$FRAMEWORKS_DIR"/*/meta.json; do
    dir="$(dirname "$meta")"
    name="$(basename "$dir")"
    if echo "$SKIP_LIST" | grep -qw "$name"; then
        echo "SKIP  $name (already done)"
        continue
    fi
    enabled=$(python3 -c "import json; print(json.load(open('$meta')).get('enabled', True))" 2>/dev/null || echo "True")
    if [ "$enabled" = "True" ]; then
        frameworks+=("$name")
    else
        echo "SKIP  $name (disabled)"
    fi
done

total=${#frameworks[@]}
echo "=== Benchmarking $total enabled frameworks ==="
echo ""

passed=0
failed=0
failed_list=()

for i in "${!frameworks[@]}"; do
    fw="${frameworks[$i]}"
    n=$((i + 1))
    log="$LOG_DIR/$fw.log"

    echo "[$n/$total] $fw"
    if "$BENCHMARK" "$fw" --save > "$log" 2>&1; then
        echo "         PASS"
        ((++passed))
    else
        echo "         FAIL (see $log)"
        ((++failed))
        failed_list+=("$fw")
    fi

    # Cool-down between frameworks
    if [ "$n" -lt "$total" ]; then
        echo "         Waiting 15s..."
        sleep 15
    fi
done

echo ""
echo "=== Done ==="
echo "Passed: $passed / $total"
if [ "$failed" -gt 0 ]; then
    echo "Failed: $failed — ${failed_list[*]}"
    exit 1
fi
