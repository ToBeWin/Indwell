# Indwell Memory OS

The current Memory OS uses append-only JSONL drawers and generated snapshots.

Implemented:

- `MemoryRecord` schema with kind, wing, room, source, confidence, importance, sensitivity, TTL, tags, and hash.
- Tombstone deletes.
- Search by wing, room, and text.
- PWA add/search/delete/export/JSON inspection controls.
- Delete requests through the tool runtime for AgentRun audit coverage.
- Compacting active records into `compacted.jsonl`.
- Export with generated snapshots:
  - persona identity/preference/emotional/safety entries
  - relationship and recent episode entries
  - index counts by kind and wing/room
- Memory metabolism:
  - TTL expiry
  - low-importance decay after long inactivity
  - preference consolidation into reflection records
  - report of decayed / expired / consolidated records

Next layers:

- Reflection-generated candidate memories.
- Richer user-visible audit reasons.
- Retention budgets for verbatim audio/image references.
- Decay and consolidation schedules.
- Optional vector index for host / phone / stronger hardware.
