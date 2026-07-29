# [0.3.1] - 2026-07-25

This release patches a bug that caused the progress update loop to break.
In that case, the simulation output is still synced on frame change, but not otherwise.