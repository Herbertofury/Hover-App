# Zero Resource Exceptions

No zero idle work exceptions have been accepted.

Policy: any non-zero idle CPU, disk, network, GPU, timer, hook, polling, allocation churn, decode work, or render work in `Dormant` is a defect until fixed or documented here with repro steps, ETW/profiler evidence, and mitigation attempts.

## Pending measurements

- [blocked] Final Dormant Task Manager/WPR/Process Explorer measurements require running the finished native Windows app in an interactive Windows desktop session.