#!/bin/sh
# Multi-process launcher: one worker per CPU core with SO_REUSEPORT
NPROC=$(nproc 2>/dev/null || grep -c ^processor /proc/cpuinfo 2>/dev/null || echo 1)
if [ -f /sys/fs/cgroup/cpu.max ]; then
  read -r quota period < /sys/fs/cgroup/cpu.max
  if [ "$quota" != "max" ]; then
    CGROUP_CPUS=$((quota / period))
    [ "$CGROUP_CPUS" -lt 1 ] && CGROUP_CPUS=1
    [ "$CGROUP_CPUS" -lt "$NPROC" ] && NPROC=$CGROUP_CPUS
  fi
fi

if [ "$NPROC" -le 1 ]; then
  exec /app/server
fi

PIDS=""
cleanup() {
  for pid in $PIDS; do
    kill "$pid" 2>/dev/null
  done
  wait
  exit 0
}
trap cleanup INT TERM

i=0
while [ "$i" -lt "$NPROC" ]; do
  /app/server &
  PIDS="$PIDS $!"
  i=$((i + 1))
done

wait
