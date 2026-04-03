#!/bin/sh
CPUS=$(nproc)
if [ -f /sys/fs/cgroup/cpu.max ]; then
  read -r quota period < /sys/fs/cgroup/cpu.max
  if [ "$quota" != "max" ]; then
    CGROUP_CPUS=$((quota / period))
    [ "$CGROUP_CPUS" -lt 1 ] && CGROUP_CPUS=1
    [ "$CGROUP_CPUS" -lt "$CPUS" ] && CPUS=$CGROUP_CPUS
  fi
fi
exec /server/bin/server "$CPUS"
