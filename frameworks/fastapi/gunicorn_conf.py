import os

def _cpu_count():
    try:
        quota, period = open('/sys/fs/cgroup/cpu.max').read().strip().split()
        if quota != 'max':
            cpus = int(quota) // int(period)
            if cpus >= 1:
                return cpus
    except Exception:
        pass
    return len(os.sched_getaffinity(0))

bind = "0.0.0.0:8080"
workers = _cpu_count()
worker_class = "uvicorn.workers.UvicornWorker"
keepalive = 120
