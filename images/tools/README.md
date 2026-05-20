# tools image

Container image family for ai-blaise tooling.

Build with `scripts/citus-scale/build-app-images.sh`. The chart keeps tools
disabled by default; when enabled, the image provides `citusctl` for operational
inspection.
