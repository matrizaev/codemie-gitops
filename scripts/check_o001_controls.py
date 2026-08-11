#!/usr/bin/env python3
"""Reproducible local O-001 configuration check and activation evidence gate."""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from o001_inventory import InventoryInputError, analyze_inventory

ROOT = Path(__file__).resolve().parents[1]
POLICY_PATH = ROOT / "ops/o001/control-policy.json"
ENVIRONMENT = re.compile(r"^[a-z0-9][a-z0-9-]{0,62}$")
EVIDENCE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$")
UTC_TIMESTAMP = re.compile(
    r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,6})?Z$"
)
GITHUB_UPLOAD_ARTIFACT_SHA = "ea165f8d65b6e75b540449e92b4886f43607fa02"
GITHUB_DOWNLOAD_ARTIFACT_SHA = "d3f86a106a0bac45b974a628896c90dbdf5c8093"
GITLAB_PROTECTED_DEFAULT_RULE = (
    "    - if: '$CODEMIE_GITOPS_APPLY == \"true\" && "
    "$CI_COMMIT_REF_PROTECTED == \"true\" && "
    "$CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH'"
)


class ControlError(ValueError):
    """A checked-in control or supplied activation record is incomplete."""


def _load_json(path: Path) -> Any:
    def reject_duplicate_names(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for name, value in pairs:
            if name in result:
                raise ControlError("O-001 JSON control artifact is invalid")
            result[name] = value
        return result

    def reject_nonstandard_constant(_: str) -> None:
        raise ControlError("O-001 JSON control artifact is invalid")

    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(
                handle,
                object_pairs_hook=reject_duplicate_names,
                parse_constant=reject_nonstandard_constant,
            )
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, ControlError) as error:
        raise ControlError("O-001 JSON control artifact is invalid") from error


def _read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise ControlError("O-001 CI control artifact is unavailable") from error


def _extract_yaml_block(content: str, key: str, indent: int) -> str:
    lines = content.splitlines()
    header = f"{' ' * indent}{key}:"
    matches = [index for index, line in enumerate(lines) if line == header]
    if len(matches) != 1:
        raise ControlError("O-001 CI control structure is incomplete")
    start = matches[0]
    end = len(lines)
    for index in range(start + 1, len(lines)):
        line = lines[index]
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        leading = len(line) - len(line.lstrip(" "))
        if leading <= indent:
            end = index
            break
    return "\n".join(lines[start:end])


def _has_exact_line(content: str, line: str) -> bool:
    return re.search(rf"(?m)^{re.escape(line)}[ \t]*$", content) is not None


def _require_lines(content: str, lines: tuple[str, ...]) -> None:
    if any(not _has_exact_line(content, line) for line in lines):
        raise ControlError("O-001 CI control structure is incomplete")


def _mapping_block_keys(content: str, indent: int) -> set[str]:
    prefix = " " * indent
    keys = re.findall(
        rf"(?m)^{re.escape(prefix)}([A-Za-z0-9_-]+):(?:[ \t].*)?$", content
    )
    duplicates = [key for key, count in Counter(keys).items() if count > 1]
    if duplicates:
        raise ControlError("O-001 CI control contains duplicate mapping keys")
    return set(keys)


def _reject_duplicate_yaml_mapping_keys(content: str) -> None:
    """Accept only the canonical YAML subset used by the O-001 controls."""

    frames: list[tuple[int, tuple[str, ...]]] = [(-1, ("root",))]
    seen: dict[tuple[str, ...], set[str]] = defaultdict(set)
    sequence_counts: dict[tuple[tuple[str, ...], int], int] = defaultdict(int)
    block_scalar_indent: int | None = None
    mapping = re.compile(r"^([A-Za-z0-9_-]+):(?:[ \t]*(.*))?$")
    plain_sequence_scalar = re.compile(r"^[A-Za-z0-9./(][^\r\n]*$")
    mapping_delimiter = re.compile(r":[ \t]|:$")

    def record(context: tuple[str, ...], key: str) -> None:
        if key in seen[context]:
            raise ControlError("O-001 CI control contains duplicate mapping keys")
        seen[context].add(key)

    for line in content.splitlines():
        if not line.strip():
            continue
        indent = len(line) - len(line.lstrip(" "))
        if block_scalar_indent is not None:
            if indent > block_scalar_indent:
                continue
            block_scalar_indent = None
        stripped = line[indent:]
        if stripped.startswith("#"):
            continue
        while len(frames) > 1 and indent <= frames[-1][0]:
            frames.pop()
        parent = frames[-1][1]

        if stripped == "-" or stripped.startswith("- "):
            rest = stripped[1:].strip()
            if not rest:
                raise ControlError("O-001 CI control uses unsupported YAML syntax")
            sequence = (parent, indent)
            item_index = sequence_counts[sequence]
            sequence_counts[sequence] += 1
            item = parent + (f"item-{indent}-{item_index}",)
            frames.append((indent, item))
            if rest in {"|", "|-", "|+", ">", ">-", ">+"}:
                block_scalar_indent = indent
                continue
            match = mapping.fullmatch(rest)
            if match is None:
                if (
                    plain_sequence_scalar.fullmatch(rest) is None
                    or mapping_delimiter.search(rest) is not None
                ):
                    raise ControlError("O-001 CI control uses unsupported YAML syntax")
                continue
            key, value = match.groups()
            record(item, key)
            value = (value or "").strip()
            if not value:
                frames.append((indent + 2, item + (key,)))
            elif value in {"|", "|-", "|+", ">", ">-", ">+"}:
                block_scalar_indent = indent
            continue

        match = mapping.fullmatch(stripped)
        if match is None:
            raise ControlError("O-001 CI control uses unsupported YAML syntax")
        key, value = match.groups()
        record(parent, key)
        value = (value or "").strip()
        if not value:
            frames.append((indent, parent + (key,)))
        elif value in {"|", "|-", "|+", ">", ">-", ">+"}:
            block_scalar_indent = indent


def _require_full_sha_actions(content: str) -> None:
    actions = re.findall(r"(?m)^\s+uses:\s+[^@\s]+@([^\s#]+)[ \t]*$", content)
    if not actions or any(re.fullmatch(r"[0-9a-f]{40}", ref) is None for ref in actions):
        raise ControlError("O-001 GitHub actions must use immutable commit SHAs")


def validate_github_workflow(content: str) -> None:
    _reject_duplicate_yaml_mapping_keys(content)
    _mapping_block_keys(content, 0)
    target_input = _extract_yaml_block(content, "target_environment", 6)
    _mapping_block_keys(target_input, 8)
    _require_lines(target_input, ("        type: environment",))

    concurrency = _extract_yaml_block(content, "concurrency", 0)
    _mapping_block_keys(concurrency, 2)
    _require_lines(
        concurrency,
        (
            "  group: codemie-gitops-apply-${{ inputs.target_environment }}",
            "  queue: max",
        ),
    )
    if re.search(r"(?m)^\s*cancel-in-progress:", concurrency):
        raise ControlError("O-001 GitHub apply must not cancel an in-flight writer")

    jobs = _extract_yaml_block(content, "jobs", 0)
    if _mapping_block_keys(jobs, 2) != {"prepare", "controlled"}:
        raise ControlError("O-001 GitHub workflow job set is not closed")
    prepare = _extract_yaml_block(jobs, "prepare", 2)
    controlled = _extract_yaml_block(jobs, "controlled", 2)
    _mapping_block_keys(prepare, 4)
    _mapping_block_keys(controlled, 4)
    if _has_exact_line(prepare, "    environment:"):
        raise ControlError("O-001 build/test job must not enter a protected environment")
    _require_lines(
        prepare,
        (
            "    permissions:",
            "      contents: read",
            "        run: python3 scripts/check_o001_controls.py --ci-preflight",
            "          O001_MODE: ${{ inputs.mode }}",
            "        run: make lint",
            "        run: make test",
            "        run: cargo build --locked --release --target x86_64-unknown-linux-musl",
            "        run: ./target/x86_64-unknown-linux-musl/release/codemie-gitops --version",
            "          sha256sum codemie-gitops > codemie-gitops.sha256",
            f"        uses: actions/upload-artifact@{GITHUB_UPLOAD_ARTIFACT_SHA}",
            "          path: o001-artifact/",
            "          if-no-files-found: error",
            "          overwrite: false",
        ),
    )
    if "secrets.CODEMIE_" in prepare:
        raise ControlError("O-001 build/test job must not receive production secrets")

    _require_lines(
        controlled,
        (
            "    needs: prepare",
            "    permissions:",
            "      contents: read",
            "    environment:",
            "      name: ${{ inputs.target_environment }}",
            "        if: ${{ inputs.mode == 'mutex-drill' }}",
            f"        uses: actions/download-artifact@{GITHUB_DOWNLOAD_ARTIFACT_SHA}",
            "          path: o001-artifact",
            "          sha256sum --check --strict codemie-gitops.sha256",
            "          chmod 0555 codemie-gitops",
            "          CODEMIE_URL: ${{ secrets.CODEMIE_URL }}",
            "          CODEMIE_TOKEN: ${{ secrets.CODEMIE_TOKEN }}",
            "        run: ./o001-artifact/codemie-gitops apply --file \"$DECLARATION_PATH\" --repo-root \"$GITHUB_WORKSPACE\"",
        ),
    )
    if any(command in controlled for command in ("cargo ", "python3 ", "make test")):
        raise ControlError("O-001 protected GitHub job must consume, not rebuild, the artifact")
    if content.count("secrets.CODEMIE_URL") != 1 or content.count(
        "secrets.CODEMIE_TOKEN"
    ) != 1:
        raise ControlError("O-001 GitHub secrets must be limited to the apply step")
    checksum = controlled.find("sha256sum --check --strict")
    execution = controlled.find("./o001-artifact/codemie-gitops apply")
    if checksum < 0 or execution < 0 or checksum >= execution:
        raise ControlError("O-001 GitHub checksum must precede artifact execution")
    _require_full_sha_actions(content)


def validate_gitlab_pipeline(content: str) -> None:
    _reject_duplicate_yaml_mapping_keys(content)
    if "$TARGET_ENVIRONMENT" in content:
        raise ControlError("O-001 GitLab environments must be static checked-in names")
    if _mapping_block_keys(content, 0) != {
        "stages",
        "variables",
        "codemie_gitops_build",
        "codemie_gitops_production_apply",
        "codemie_gitops_production_mutex_drill",
        "codemie_gitops_staging_mutex_drill",
    }:
        raise ControlError("O-001 GitLab pipeline job set is not closed")
    variables = _extract_yaml_block(content, "variables", 0)
    _mapping_block_keys(variables, 2)
    _require_lines(
        variables,
        (
            '  CODEMIE_GITOPS_APPLY: "false"',
            '  INVENTORY_APPROVED: "false"',
            '  WRITER_FREEZE_APPROVED: "false"',
            '  CONTROL_EVIDENCE: ""',
        ),
    )

    build = _extract_yaml_block(content, "codemie_gitops_build", 0)
    _mapping_block_keys(build, 2)
    if _has_exact_line(build, "  environment:") or any(
        secret in build for secret in ("CODEMIE_TOKEN", "CODEMIE_URL")
    ):
        raise ControlError("O-001 GitLab build/test job must be unprivileged")
    _require_lines(
        build,
        (
            "  stage: codemie-build",
            "  interruptible: false",
            "    - codemie-gitops-musl-build",
            GITLAB_PROTECTED_DEFAULT_RULE,
            "    - TARGET_ENVIRONMENT=production O001_MODE=apply python3 scripts/check_o001_controls.py --ci-preflight",
            "      command -v musl-gcc",
            "      rustup target list --installed | grep -Fxq x86_64-unknown-linux-musl",
            "      export CARGO_BUILD_TARGET=x86_64-unknown-linux-musl",
            "      make lint",
            "      make test",
            "      cargo build --locked --release --target x86_64-unknown-linux-musl",
            "      target/x86_64-unknown-linux-musl/release/codemie-gitops --version",
            "      install -m 0755 target/x86_64-unknown-linux-musl/release/codemie-gitops o001-artifact/codemie-gitops",
            "      sha256sum codemie-gitops > codemie-gitops.sha256",
            "  artifacts:",
            "    when: on_success",
            "      - o001-artifact/codemie-gitops",
            "      - o001-artifact/codemie-gitops.sha256",
        ),
    )

    apply = _extract_yaml_block(content, "codemie_gitops_production_apply", 0)
    _mapping_block_keys(apply, 2)
    _require_lines(
        apply,
        (
            "  stage: codemie-apply",
            "  interruptible: false",
            "    - job: codemie_gitops_build",
            "      artifacts: true",
            GITLAB_PROTECTED_DEFAULT_RULE,
            "      when: manual",
            "      allow_failure: false",
            "  environment:",
            "    name: production",
            "  resource_group: codemie-gitops-apply-production",
            "    - codemie-gitops-production-protected",
            "    - (cd o001-artifact && sha256sum --check --strict codemie-gitops.sha256) && chmod 0555 o001-artifact/codemie-gitops",
            "    - ./o001-artifact/codemie-gitops apply --file \"$DECLARATION_PATH\" --repo-root \"$CI_PROJECT_DIR\"",
        ),
    )
    if any(command in apply for command in ("cargo ", "python3 ", "make test")):
        raise ControlError("O-001 protected GitLab apply must consume the build artifact")
    checksum = apply.find("sha256sum --check --strict")
    execution = apply.find("./o001-artifact/codemie-gitops apply")
    if checksum < 0 or execution < 0 or checksum >= execution:
        raise ControlError("O-001 GitLab checksum must precede artifact execution")

    for job, environment in (
        ("codemie_gitops_production_mutex_drill", "production"),
        ("codemie_gitops_staging_mutex_drill", "staging"),
    ):
        drill = _extract_yaml_block(content, job, 0)
        _mapping_block_keys(drill, 2)
        _require_lines(
            drill,
            (
                "  interruptible: false",
                "  needs: []",
                GITLAB_PROTECTED_DEFAULT_RULE,
                "      when: manual",
                "      allow_failure: false",
                "  environment:",
                f"    name: {environment}",
                f"  resource_group: codemie-gitops-apply-{environment}",
                "    - codemie-gitops-production-protected",
                "    - sleep 30",
            ),
        )


def _schema_version_one(value: Any) -> bool:
    return type(value) is int and value == 1


def validate_policy(policy: Any) -> None:
    expected_roles = {
        "serialization": "Release Engineering",
        "identityWriterGovernance": "CodeMie Platform Administrator",
        "identityInventory": "CodeMie Platform Identity Administrator",
        "remediation": "CodeMie Platform Operations Incident Commander",
    }
    if (
        not isinstance(policy, dict)
        or not _schema_version_one(policy.get("schemaVersion"))
        or policy.get("ownerRoles") != expected_roles
    ):
        raise ControlError("O-001 named role ownership is incomplete")
    serialization = policy.get("serialization", {})
    if (
        serialization.get("githubConcurrencyPrefix")
        != "codemie-gitops-apply-"
        or serialization.get("githubQueue") != "max"
        or serialization.get("gitlabResourceGroupPrefix")
        != "codemie-gitops-apply-"
        or serialization.get("gitlabProductionEnvironment") != "production"
        or serialization.get("gitlabIndependentDrillEnvironment") != "staging"
        or serialization.get("remoteProtectedEnvironmentRequired") is not True
    ):
        raise ControlError("O-001 serialization policy is incomplete")
    if policy.get("artifactPromotion") != {
        "target": "x86_64-unknown-linux-musl",
        "gitlabBuildRunnerTag": "codemie-gitops-musl-build",
        "buildAndTestWithoutProductionEnvironment": True,
        "applyConsumesBuildArtifact": True,
        "sha256VerificationBeforeExecution": True,
        "rebuildInProtectedApplyJob": False,
    }:
        raise ControlError("O-001 exact-artifact promotion policy is incomplete")
    governed = policy.get("identityWriterGovernance", {}).get("entities")
    if governed != ["Workflow", "Skill", "Datasource"]:
        raise ControlError("O-001 identity writer coverage is incomplete")
    gates = policy.get("inventory", {}).get("gates", {})
    if set(gates) != {
        "invalidWorkflowMarkers",
        "duplicateWorkflowNaturalKeys",
        "invalidSkillRecords",
        "duplicateSkillNaturalKeys",
        "invalidDatasourceRecords",
        "duplicateDatasourceNaturalKeys",
    } or any(type(value) is not int or value != 0 for value in gates.values()):
        raise ControlError("O-001 inventory gates are incomplete")
    remediation = policy.get("remediation", {})
    if any(
        remediation.get(name) is not False
        for name in (
            "automaticRetry",
            "automaticDelete",
            "automaticRollback",
            "automaticTieBreak",
        )
    ) or remediation.get("resumeRequiresCleanInventory") is not True:
        raise ControlError("O-001 remediation guardrails are incomplete")


def check_local_controls() -> dict[str, Any]:
    policy = _load_json(POLICY_PATH)
    validate_policy(policy)
    validate_github_workflow(
        _read_text(ROOT / ".github/workflows/codemie-gitops-apply.yml")
    )
    validate_gitlab_pipeline(_read_text(ROOT / ".gitlab-ci.yml"))

    prefix = policy["serialization"]["githubConcurrencyPrefix"]
    same_a = f"{prefix}production"
    same_b = f"{prefix}production"
    different = f"{prefix}staging"
    if same_a != same_b or same_a == different:
        raise ControlError("O-001 mutex key drill failed")

    clean = analyze_inventory(
        _load_json(ROOT / "ops/o001/fixtures/inventory-clean.json")
    )
    broken = analyze_inventory(
        _load_json(ROOT / "ops/o001/fixtures/inventory-broken.json")
    )
    if clean["status"] != "pass" or broken["status"] != "fail":
        raise ControlError("O-001 inventory fixture drill failed")

    return {
        "status": "local-pass",
        "activationStatus": "required",
        "checks": [
            "github-environment-mutex",
            "gitlab-environment-mutex",
            "exact-artifact-promotion",
            "named-role-ownership",
            "workflow-skill-datasource-inventory",
            "non-destructive-remediation",
        ],
    }


def check_ci_preflight() -> dict[str, str]:
    environment = os.environ.get("TARGET_ENVIRONMENT", "")
    evidence = os.environ.get("CONTROL_EVIDENCE", "")
    mode = os.environ.get("O001_MODE", "apply")
    if ENVIRONMENT.fullmatch(environment) is None:
        raise ControlError("O-001 target environment is not canonical")
    if os.environ.get("INVENTORY_APPROVED", "").lower() != "true":
        raise ControlError("O-001 identity inventory is not approved")
    if os.environ.get("WRITER_FREEZE_APPROVED", "").lower() != "true":
        raise ControlError("O-001 external writer freeze is not approved")
    if EVIDENCE.fullmatch(evidence) is None:
        raise ControlError("O-001 control evidence reference is invalid")
    if mode not in {"apply", "mutex-drill"}:
        raise ControlError("O-001 mode is invalid")
    return {"status": "preflight-pass", "mode": mode}


def _required_string(value: Any) -> bool:
    return isinstance(value, str) and EVIDENCE.fullmatch(value) is not None


def _required_utc_timestamp(value: Any) -> bool:
    if not isinstance(value, str) or UTC_TIMESTAMP.fullmatch(value) is None:
        return False
    try:
        parsed = datetime.fromisoformat(f"{value[:-1]}+00:00")
    except ValueError:
        return False
    return parsed.tzinfo == timezone.utc


def _different_environment(value: Any, primary: str) -> bool:
    return (
        isinstance(value, str)
        and ENVIRONMENT.fullmatch(value) is not None
        and value != primary
    )


def _distinct_references(section: dict[str, Any], fields: tuple[str, ...]) -> bool:
    references = [section.get(field) for field in fields]
    return all(_required_string(value) for value in references) and len(
        set(references)
    ) == len(references)


def check_activation_evidence(path: Path) -> dict[str, str]:
    evidence = _load_json(path)
    if not isinstance(evidence, dict):
        raise ControlError("O-001 activation evidence is invalid")
    environment = evidence.get("environment")
    if (
        not _schema_version_one(evidence.get("schemaVersion"))
        or environment != "production"
    ):
        raise ControlError("O-001 activation environment evidence is invalid")
    github = evidence.get("github", {})
    gitlab = evidence.get("gitlab", {})
    writers = evidence.get("writerGovernance", {})
    if not all(isinstance(section, dict) for section in (github, gitlab, writers)):
        raise ControlError("O-001 activation provider evidence is invalid")
    if (
        github.get("protectedEnvironmentEnabled") is not True
        or github.get("requiredReviewersEnabled") is not True
        or github.get("deploymentBranchPolicyEnabled") is not True
        or github.get("sameEnvironmentOverlapObserved") is not False
        or not _different_environment(github.get("differentEnvironment"), environment)
        or github.get("differentEnvironmentIndependentObserved") is not True
        or github.get("artifactChecksumVerified") is not True
        or github.get("verifiedByRole") != "Release Engineering"
        or not _distinct_references(
            github,
            (
                "sameEnvironmentRunA",
                "sameEnvironmentRunB",
                "differentEnvironmentRun",
                "artifactPromotionRun",
            ),
        )
        or not _required_utc_timestamp(github.get("verifiedAt"))
    ):
        raise ControlError("O-001 GitHub activation evidence is incomplete")
    if (
        gitlab.get("protectedEnvironmentEnabled") is not True
        or gitlab.get("requiredApprovalEnabled") is not True
        or gitlab.get("protectedVariablesEnabled") is not True
        or gitlab.get("deploymentBranchPolicyEnabled") is not True
        or gitlab.get("muslBuildRunnerReady") is not True
        or gitlab.get("protectedRunnerEnabled") is not True
        or gitlab.get("protectedRefEnabled") is not True
        or gitlab.get("pipelineVariableOverridesRestricted") is not True
        or gitlab.get("environmentScopedSecrets")
        != ["CODEMIE_TOKEN", "CODEMIE_URL"]
        or gitlab.get("secretEnvironmentScope") != environment
        or gitlab.get("sameEnvironmentOverlapObserved") is not False
        or gitlab.get("differentEnvironment") != "staging"
        or gitlab.get("differentEnvironmentIndependentObserved") is not True
        or gitlab.get("artifactChecksumVerified") is not True
        or gitlab.get("verifiedByRole") != "Release Engineering"
        or not _distinct_references(
            gitlab,
            (
                "sameEnvironmentPipelineA",
                "sameEnvironmentPipelineB",
                "differentEnvironmentPipeline",
                "artifactPromotionPipeline",
            ),
        )
        or not _required_utc_timestamp(gitlab.get("verifiedAt"))
    ):
        raise ControlError("O-001 GitLab activation evidence is incomplete")
    if (
        writers.get("approvedByRole") != "CodeMie Platform Administrator"
        or writers.get("inventoryClean") is not True
        or writers.get("externalWritersFrozen") is not True
        or not _required_string(writers.get("freezeEvidence"))
        or not _required_string(writers.get("inventoryEvidence"))
        or writers.get("freezeEvidence") == writers.get("inventoryEvidence")
    ):
        raise ControlError("O-001 writer governance evidence is incomplete")
    return {"status": "activation-pass", "environment": environment}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--ci-preflight", action="store_true")
    parser.add_argument("--activation-evidence", type=Path)
    args = parser.parse_args()
    try:
        if args.ci_preflight:
            result: Any = check_ci_preflight()
        else:
            result = check_local_controls()
            if args.activation_evidence is not None:
                result = {
                    "local": result,
                    "activation": check_activation_evidence(args.activation_evidence),
                }
    except (ControlError, InventoryInputError):
        print("O-001 control check failed", file=sys.stderr)
        return 2 if args.ci_preflight else 1
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
