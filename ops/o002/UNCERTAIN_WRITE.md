# Uncertain-write recovery

An exit, timeout, signal, malformed response, verification failure, or lost
connection after a modifying request may mean the write committed.

1. Stop immediately. Hold the apply queue and preserve the affected natural
   key and run prefix.
2. Unset tokens, client secrets, and passwords. Do not collect shell traces,
   environment dumps, response bodies, payloads, or arbitrary headers.
3. Freeze every UI/API/CI identity writer under the named incident owner.
4. Record only fixed status, kind, project/natural-key identity, artifact
   SHA-256, safe request ID, and time-window metadata.
5. Run complete non-mutating inventory with global/project and marketplace
   visibility as required. Detect invalid markers, duplicates, and incomplete
   visibility; do not select among them.
6. Route the evidence to the O-001 uncertain-write/manual platform remediation
   owner. Resume only after reviewed remediation and a clean complete inventory.
7. Start any correction as a new reviewed invocation and exclusive window.

Blind retry, automatic delete, remote rollback, identity tie-break, rename/move,
and automated cleanup are forbidden. If inventory cannot prove one safe state,
the queue remains held.
