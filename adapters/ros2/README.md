# ROS 2 adapter

This directory owns physical robot and instrument execution. The initial target is ROS 2 Jazzy on Ubuntu 24.04 with telemetry recorded directly to MCAP.

The adapter must:

- fail before motion when required devices or capabilities are unavailable;
- enforce request deadlines, step limits, workspace limits, and an external emergency stop;
- never hold an execution lease across unrelated network work;
- record commands, observations, measurements, and safety events with a shared clock;
- emit the same typed outcome contract used by simulator runtimes.

Physical actuation is intentionally absent from the initial commit. Hardware-specific safety review is required before implementation.
