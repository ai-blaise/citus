# Upgrade Runbook

This runbook is a starting point for the first release train.

The upgrade workflow must:

1. Fetch upstream Citus.
2. Run `make -f Makefile.ai-blaise patches-check`.
3. Rebuild overlay images.
4. Run the cohabitation and e2e gates.
5. Publish release notes with `NEW_FEATURES.md` deltas.
