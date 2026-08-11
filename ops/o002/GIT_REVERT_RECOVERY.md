# Git revert and new apply

Use this procedure when reviewed desired state was wrong but the prior write is
known to have completed.

1. Stop the serialized apply queue and keep other identity writers frozen.
2. Create and review a new Git revert or corrective commit. Do not rewrite
   history and do not edit the prior CI artifact.
3. Run locked formatting, tests, offline lint, and the example/control checks.
4. Build once without deployment credentials, record its SHA-256, and promote
   that exact artifact to the protected job.
5. Re-establish complete-visibility inventory and the exclusive writer window.
6. Execute one new serialized `apply` for each explicitly reviewed declaration.
7. Retain only approved outcome and checksum evidence, then release the queue.

A Git revert changes desired state; it does not roll back the remote record.
Never automatically delete, rename, move, tie-break, or retry an uncertain
write. If completion is uncertain, stop here and use `UNCERTAIN_WRITE.md`.
