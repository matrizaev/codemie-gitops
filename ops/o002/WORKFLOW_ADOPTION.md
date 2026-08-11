# Workflow adoption

Adoption is only for one reviewed, unmarked legacy Workflow whose immutable
server UUID is supplied out of band for a single invocation.

1. Hold the serialized queue and freeze every other Workflow identity writer.
2. With complete project and marketplace visibility, prove no valid reserved
   identity marker already exists for the intended project/slug.
3. Review the exact UUID, exact project, write permission, and mergeable
   non-reserved `meta_config`. Display name never selects or vetoes the row.
4. Lint the declaration and use the exact tested, checksummed binary.
5. Invoke `apply --adopt-workflow-id <reviewed-uuid>` exactly once for that
   Workflow declaration.
6. Inventory all scopes again and require exactly one valid reserved identity.
7. Record only safe outcome/checksum evidence and release the queue.

Do not store the UUID in YAML, config, a selector file, or normal output. Do not
adopt by name, choose first/newest/current-principal, overwrite a malformed or
conflicting marker, or invoke adoption again after uncertainty. An ambiguous or
uncertain result routes to `UNCERTAIN_WRITE.md`.
