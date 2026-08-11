#!/usr/bin/env python3
"""Structural, offline O-002A example and recovery-policy checker."""

from __future__ import annotations

import argparse
import copy
import os
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path
from typing import Any

try:
    import yaml
except ImportError as error:  # pragma: no cover - deployment prerequisite
    raise SystemExit("O-002 checker requires PyYAML") from error


ROOT = Path(__file__).resolve().parents[1]
REPOSITORY = ROOT / "examples/repository"
DECLARATIONS = (
    Path("assistants/example-assistant.yaml"),
    Path("workflows/example-workflow.yaml"),
    Path("skills/example-skill.yaml"),
    Path("datasources/example-datasource.yaml"),
)
REQUIRED_PATHS = (
    Path("README.md"),
    Path("examples/README.md"),
    Path("examples/repository/.codemie/config.yaml"),
    *(Path("examples/repository") / path for path in DECLARATIONS),
    Path("examples/repository/skills/example-skill.md"),
    Path("examples/ci/github-actions.yml"),
    Path("examples/ci/gitlab-ci.yml"),
    Path("ops/o002/README.md"),
    Path("ops/o002/GIT_REVERT_RECOVERY.md"),
    Path("ops/o002/WORKFLOW_ADOPTION.md"),
    Path("ops/o002/UNCERTAIN_WRITE.md"),
)
ACTION_SHA = re.compile(r"^[0-9a-f]{40}$")
ENTERPRISE_MARKERS = ("codemie.lab.epam.com", "api.codemie.epam.com")


class ExampleError(ValueError):
    """The checked-in O-002 examples fail a closed safety invariant."""


class StrictLoader(yaml.SafeLoader):
    """Safe YAML loader that retains CI keys such as `on` as strings."""


StrictLoader.yaml_implicit_resolvers = copy.deepcopy(yaml.SafeLoader.yaml_implicit_resolvers)
for initial, resolvers in list(StrictLoader.yaml_implicit_resolvers.items()):
    StrictLoader.yaml_implicit_resolvers[initial] = [
        (tag, pattern)
        for tag, pattern in resolvers
        if tag != "tag:yaml.org,2002:bool"
    ]


def _mapping(loader: StrictLoader, node: yaml.MappingNode, deep: bool = False) -> dict[Any, Any]:
    pairs = loader.construct_pairs(node, deep=deep)
    counts = Counter(key for key, _ in pairs)
    if any(count != 1 for count in counts.values()):
        raise ExampleError("O-002 YAML contains duplicate mapping keys")
    return dict(pairs)


StrictLoader.add_constructor(
    yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG,
    _mapping,
)


def load_yaml_text(content: str) -> Any:
    try:
        documents = list(yaml.load_all(content, Loader=StrictLoader))
    except (yaml.YAMLError, ExampleError) as error:
        raise ExampleError("O-002 YAML is structurally invalid") from error
    if len(documents) != 1:
        raise ExampleError("O-002 YAML must contain exactly one document")
    return documents[0]


def load_yaml(path: Path) -> Any:
    try:
        return load_yaml_text(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError) as error:
        raise ExampleError("O-002 artifact is unavailable") from error


def _mapping_value(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or any(not isinstance(key, str) for key in value):
        raise ExampleError(f"O-002 {label} must be a mapping")
    return value


def _steps(job: dict[str, Any], label: str) -> list[dict[str, Any]]:
    raw = job.get("steps")
    if not isinstance(raw, list) or not raw:
        raise ExampleError(f"O-002 {label} steps are incomplete")
    result: list[dict[str, Any]] = []
    for step in raw:
        result.append(_mapping_value(step, f"{label} step"))
    return result


def _script_text(job: dict[str, Any]) -> str:
    script = job.get("script")
    if not isinstance(script, list) or not all(isinstance(item, str) for item in script):
        raise ExampleError("O-002 GitLab script must be a scalar sequence")
    return "\n".join(script)


def _shell_lines(content: str) -> list[str]:
    return [line.strip() for line in content.splitlines() if line.strip() and not line.lstrip().startswith("#")]


def _ordered_indices(lines: list[str], required: tuple[str, ...], label: str) -> list[int]:
    try:
        indices = [lines.index(command) for command in required]
    except ValueError as error:
        raise ExampleError(f"O-002 {label} command gate is incomplete") from error
    if indices != sorted(indices) or len(set(indices)) != len(indices):
        raise ExampleError(f"O-002 {label} command order is invalid")
    return indices


def _validate_build_commands(lines: list[str], label: str) -> None:
    required = (
        "make lint",
        "make test",
        "cargo build --locked --release",
        "python3 scripts/check_o002_examples.py",
        "install -D -m 0555 target/release/codemie-gitops o002-artifact/codemie-gitops",
        "( cd o002-artifact && sha256sum codemie-gitops > codemie-gitops.sha256 )",
    )
    _ordered_indices(lines, required, label)
    if lines != ["set -euo pipefail", *required]:
        raise ExampleError(f"O-002 {label} command surface is not closed")
    if any("check_o002_examples.py" in line and line != required[3] for line in lines):
        raise ExampleError(f"O-002 {label} must run the full checker")


def _assert_safe_shell(lines: list[str]) -> None:
    scanned = "\n".join(line for line in lines if line != 'echo "::add-mask::$token"')
    forbidden = (
        r"(?:^|\s)--(?:token|client-secret|password)(?:\s|=)",
        r"(?:^|\s)--insecure(?:\s|[\"')]|$)",
        r"\bsource\s+[^\n]*\.env\b",
        r"(?:^|[;&]\s*)\.\s+[^\n]*\.env\b",
        r"(?:^|\s)set\s+-x(?:\s|$)",
        r"\bRUST_LOG\b",
        r"\bcurl\s+[^\n]*-(?:[^\s]*v|-[^\s]*verbose)\b",
        r"\b(?:env|printenv)(?:\s|$)",
        r"\b(?:declare|export)\s+-p\b",
        r"/proc/(?:self|[0-9]+)/environ",
        r"\b(?:tee|cp|mv|scp|rsync)\b[^\n]*(?:token|login)",
        r"(?:GITHUB_ENV|GITHUB_OUTPUT)[^\n]*(?:TOKEN|token)",
        r"(?:artifact|cache|dotenv)[^\n]*(?:TOKEN|token)",
        r"\b(?:echo|printf)\b[^\n]*\$(?:\{)?(?:CODEMIE_TOKEN|token)",
        r"(?:CODEMIE_TOKEN|\$token)[^\n]*(?:>>?|\|)",
    )
    if any(re.search(pattern, scanned, re.IGNORECASE | re.MULTILINE) for pattern in forbidden):
        raise ExampleError("O-002 provider example contains an unsafe credential/logging pattern")
    if ".env" in scanned:
        raise ExampleError("O-002 CI examples must never load a dotenv file")


def validate_github(content: str) -> None:
    document = _mapping_value(load_yaml_text(content), "GitHub document")
    triggers = _mapping_value(document.get("on"), "GitHub triggers")
    if set(triggers) != {"pull_request", "workflow_dispatch"}:
        raise ExampleError("O-002 GitHub trigger set must be closed and fork-safe")
    jobs = _mapping_value(document.get("jobs"), "GitHub jobs")
    if set(jobs) != {"build", "apply"}:
        raise ExampleError("O-002 GitHub job set must be build/apply only")
    build = _mapping_value(jobs["build"], "GitHub build job")
    apply = _mapping_value(jobs["apply"], "GitHub apply job")
    build_steps = _steps(build, "GitHub build")
    build_text = "\n".join(str(step.get("run", "")) for step in build_steps)
    build_lines = _shell_lines(build_text)
    apply_steps = _steps(apply, "GitHub apply")
    apply_text = "\n".join(str(step.get("run", "")) for step in apply_steps)
    apply_lines = _shell_lines(apply_text)
    if "environment" in build or any("env" in step for step in build_steps) or re.search(r"(?:secrets\.|CODEMIE_)", str(build)):
        raise ExampleError("O-002 GitHub build/PR boundary must be secret-free")
    _validate_build_commands(build_lines, "GitHub build")
    condition = apply.get("if")
    if not isinstance(condition, str) or "workflow_dispatch" not in condition or "refs/heads/main" not in condition:
        raise ExampleError("O-002 GitHub apply must be manual and trusted-ref only")
    environment = _mapping_value(apply.get("environment"), "GitHub protected environment")
    if not isinstance(environment.get("name"), str) or not environment["name"]:
        raise ExampleError("O-002 GitHub apply must name a protected environment")
    concurrency = _mapping_value(apply.get("concurrency"), "GitHub concurrency")
    if concurrency.get("group") != "codemie-gitops-apply-codemie-apply" or concurrency.get("cancel-in-progress") not in (False, "false"):
        raise ExampleError("O-002 GitHub apply serialization is incomplete")
    if apply.get("needs") != "build":
        raise ExampleError("O-002 GitHub apply must consume the build artifact")
    uses = [step.get("uses") for step in _steps(build, "GitHub build") + apply_steps if "uses" in step]
    if not uses or any(not isinstance(use, str) or "@" not in use or not ACTION_SHA.fullmatch(use.rsplit("@", 1)[1]) for use in uses):
        raise ExampleError("O-002 GitHub actions must use immutable commit SHAs")
    if any(term in apply_text for term in ("cargo build", "make test", "make lint", "check_o002_examples.py")):
        raise ExampleError("O-002 protected GitHub job must not rebuild the artifact")
    checksum_command = "( cd o002-artifact && sha256sum --check --strict codemie-gitops.sha256 )"
    login_command = 'token="$(../../o002-artifact/codemie-gitops login)"'
    mask_command = 'echo "::add-mask::$token"'
    indices = _ordered_indices(
        apply_lines,
        (checksum_command, "cd examples/repository", login_command, mask_command),
        "GitHub apply",
    )
    if indices[3] != indices[2] + 1:
        raise ExampleError("O-002 GitHub token masking/checksum order is unsafe")
    expected_apply_lines = [
        "set -euo pipefail",
        "set +x",
        checksum_command,
        "cd examples/repository",
        login_command,
        mask_command,
        'export CODEMIE_TOKEN="$token"',
        "unset CODEMIE_CLIENT_SECRET CODEMIE_PASSWORD",
        "../../o002-artifact/codemie-gitops apply --file skills/example-skill.yaml --repo-root .",
        "../../o002-artifact/codemie-gitops apply --file assistants/example-assistant.yaml --repo-root .",
        "../../o002-artifact/codemie-gitops apply --file workflows/example-workflow.yaml --repo-root .",
        "unset CODEMIE_TOKEN token",
    ]
    if apply_lines != expected_apply_lines:
        raise ExampleError("O-002 GitHub protected command surface is not closed")
    if sum("codemie-gitops login" in line for line in apply_lines) != 1:
        raise ExampleError("O-002 GitHub must acquire exactly one fresh login token")
    first_apply = next((index for index, line in enumerate(apply_lines) if "codemie-gitops apply" in line), -1)
    if first_apply <= indices[3]:
        raise ExampleError("O-002 GitHub apply command order is unsafe")
    credential_steps = [step for step in apply_steps if "env" in step]
    if len(credential_steps) != 1 or credential_steps[0] is not apply_steps[-1]:
        raise ExampleError("O-002 GitHub credentials must be scoped to the final step")
    credentials = _mapping_value(credential_steps[0]["env"], "GitHub credential environment")
    expected_credentials = {"CODEMIE_URL", "CODEMIE_AUTH_URL", "CODEMIE_CLIENT_ID", "CODEMIE_CLIENT_SECRET"}
    if set(credentials) != expected_credentials or any(
        not isinstance(value, str) or re.fullmatch(r"\$\{\{ secrets\.[A-Z0-9_]+ \}\}", value) is None
        for value in credentials.values()
    ):
        raise ExampleError("O-002 GitHub credential mapping is not closed")
    allowed_token_lines = {
        login_command,
        mask_command,
        'export CODEMIE_TOKEN="$token"',
        "unset CODEMIE_TOKEN token",
    }
    if any("token" in line.casefold() and line not in allowed_token_lines for line in apply_lines):
        raise ExampleError("O-002 GitHub token lifetime is not process-local")
    upload = next((step for step in build_steps if isinstance(step.get("uses"), str) and "actions/upload-artifact@" in step["uses"]), None)
    download = next((step for step in apply_steps if isinstance(step.get("uses"), str) and "actions/download-artifact@" in step["uses"]), None)
    if upload is None or download is None:
        raise ExampleError("O-002 GitHub artifact handoff is incomplete")
    upload_with = _mapping_value(upload.get("with"), "GitHub artifact upload")
    download_with = _mapping_value(download.get("with"), "GitHub artifact download")
    if upload_with.get("name") != "codemie-gitops-o002" or upload_with.get("path") != "o002-artifact/":
        raise ExampleError("O-002 GitHub artifact upload is not fixed")
    if download_with.get("name") != "codemie-gitops-o002" or download_with.get("path") != "o002-artifact":
        raise ExampleError("O-002 GitHub artifact download is not fixed")
    _assert_safe_shell(build_lines + apply_lines)


def validate_gitlab(content: str) -> None:
    document = _mapping_value(load_yaml_text(content), "GitLab document")
    if set(document) != {"stages", "workflow", "build_o002", "apply_o002"}:
        raise ExampleError("O-002 GitLab pipeline structure is not closed")
    build = _mapping_value(document["build_o002"], "GitLab build job")
    apply = _mapping_value(document["apply_o002"], "GitLab apply job")
    build_text = _script_text(build)
    apply_text = _script_text(apply)
    build_lines = _shell_lines(build_text)
    apply_lines = _shell_lines(apply_text)
    if "environment" in build or "variables" in build or re.search(r"(?:CODEMIE_|\$CI_JOB_JWT|\$CI_JOB_TOKEN)", str(build)):
        raise ExampleError("O-002 GitLab build/MR boundary must be secret-free")
    _validate_build_commands(build_lines, "GitLab build")
    if apply.get("resource_group") != "codemie-gitops-apply-codemie-apply":
        raise ExampleError("O-002 GitLab resource-group serialization is missing")
    tags = apply.get("tags")
    if not isinstance(tags, list) or "protected-runner" not in tags:
        raise ExampleError("O-002 GitLab apply must select a protected runner")
    environment = _mapping_value(apply.get("environment"), "GitLab environment")
    if environment.get("name") != "codemie-apply":
        raise ExampleError("O-002 GitLab apply must name the protected environment")
    rules = apply.get("rules")
    if not isinstance(rules, list) or len(rules) != 2:
        raise ExampleError("O-002 GitLab apply rules are incomplete")
    first = _mapping_value(rules[0], "GitLab protected rule")
    condition = first.get("if")
    if first.get("when") != "manual" or not isinstance(condition, str) or "CI_COMMIT_REF_PROTECTED" not in condition or "CI_DEFAULT_BRANCH" not in condition:
        raise ExampleError("O-002 GitLab apply must be protected-ref manual")
    needs = apply.get("needs")
    if not isinstance(needs, list) or len(needs) != 1 or _mapping_value(needs[0], "GitLab artifact need").get("artifacts") not in (True, "true"):
        raise ExampleError("O-002 GitLab apply must consume the build artifact")
    if any(term in apply_text for term in ("cargo build", "make test", "make lint", "check_o002_examples.py")):
        raise ExampleError("O-002 protected GitLab job must not rebuild the artifact")
    checksum_command = "( cd o002-artifact && sha256sum --check --strict codemie-gitops.sha256 )"
    indices = _ordered_indices(
        apply_lines,
        (
            'test "${CI_DEBUG_TRACE:-false}" != "true"',
            checksum_command,
            "cd examples/repository",
            'test -n "${CODEMIE_TOKEN:-}"',
            "export CODEMIE_TOKEN",
        ),
        "GitLab apply",
    )
    expected_apply_lines = [
        "set -euo pipefail",
        "set +x",
        'test "${CI_DEBUG_TRACE:-false}" != "true"',
        checksum_command,
        "cd examples/repository",
        'test -n "${CODEMIE_TOKEN:-}"',
        "export CODEMIE_TOKEN",
        "../../o002-artifact/codemie-gitops apply --file skills/example-skill.yaml --repo-root .",
        "../../o002-artifact/codemie-gitops apply --file assistants/example-assistant.yaml --repo-root .",
        "../../o002-artifact/codemie-gitops apply --file workflows/example-workflow.yaml --repo-root .",
        "unset CODEMIE_TOKEN",
    ]
    if apply_lines != expected_apply_lines:
        raise ExampleError("O-002 GitLab protected command surface is not closed")
    if "codemie-gitops login" in apply_text or "add-mask" in apply_text or re.search(r"CODEMIE_TOKEN\s*=", apply_text):
        raise ExampleError("O-002 GitLab must consume only its pre-supplied masked token")
    allowed_token_lines = {'test -n "${CODEMIE_TOKEN:-}"', "export CODEMIE_TOKEN", "unset CODEMIE_TOKEN"}
    if any("token" in line.casefold() and line not in allowed_token_lines for line in apply_lines):
        raise ExampleError("O-002 GitLab token lifetime is not process-local")
    first_apply = next((index for index, line in enumerate(apply_lines) if "codemie-gitops apply" in line), -1)
    if first_apply <= indices[-1]:
        raise ExampleError("O-002 GitLab apply command order is unsafe")
    artifacts = _mapping_value(build.get("artifacts"), "GitLab artifact")
    if (
        "reports" in artifacts
        or "dotenv" in str(artifacts).lower()
        or artifacts.get("paths") != ["o002-artifact/"]
    ):
        raise ExampleError("O-002 GitLab token material must not use dotenv reports")
    _assert_safe_shell(build_lines + apply_lines)


def validate_declarations(root: Path) -> None:
    config = _mapping_value(load_yaml(root / ".codemie/config.yaml"), "repository config")
    if set(config) != {"url", "auth_url", "project"} or not all(isinstance(value, str) and value for value in config.values()):
        raise ExampleError("O-002 repository config is not closed and non-secret")
    if any(marker in str(config) for marker in ENTERPRISE_MARKERS) or not str(config["url"]).endswith(".example.invalid"):
        raise ExampleError("O-002 repository config must use portable reserved endpoints")
    expected = {"Assistant", "Workflow", "Skill", "Datasource"}
    found: set[str] = set()
    for relative in DECLARATIONS:
        document = _mapping_value(load_yaml(root / relative), "declaration")
        kind = document.get("kind")
        if kind not in expected or kind in found:
            raise ExampleError("O-002 declarations must contain each supported kind once")
        found.add(kind)
        text = (root / relative).read_text(encoding="utf-8")
        if any(marker in text for marker in ENTERPRISE_MARKERS):
            raise ExampleError("O-002 declarations must be target-portable")
        lowered_keys: set[str] = set()

        def collect(value: Any) -> None:
            if isinstance(value, dict):
                for key, nested in value.items():
                    lowered_keys.add(str(key).lower())
                    collect(nested)
            elif isinstance(value, list):
                for nested in value:
                    collect(nested)

        collect(document)
        if lowered_keys.intersection({"token", "password", "client_secret", "access_token", "id", "uuid"}):
            raise ExampleError("O-002 declarations contain a credential or server-ID position")
    if found != expected:
        raise ExampleError("O-002 declaration kind set is incomplete")


def validate_runbooks(root: Path) -> None:
    root_readme = (root / "README.md").read_text(encoding="utf-8")
    example_readme = (root / "examples/README.md").read_text(encoding="utf-8")
    for name in ("GIT_REVERT_RECOVERY.md", "WORKFLOW_ADOPTION.md", "UNCERTAIN_WRITE.md"):
        if name not in root_readme or name not in example_readme:
            raise ExampleError("O-002 recovery links are incomplete")
    revert = (root / "ops/o002/GIT_REVERT_RECOVERY.md").read_text(encoding="utf-8").lower()
    adoption = (root / "ops/o002/WORKFLOW_ADOPTION.md").read_text(encoding="utf-8").lower()
    uncertain = (root / "ops/o002/UNCERTAIN_WRITE.md").read_text(encoding="utf-8").lower()
    if not all(term in revert for term in ("new git revert", "new serialized", "does not roll back", "never automatically delete")):
        raise ExampleError("O-002 Git recovery prohibitions are incomplete")
    if not all(term in adoption for term in ("exact uuid", "exactly once", "display name never selects", "adopt by name")):
        raise ExampleError("O-002 Workflow adoption invariants are incomplete")
    if not all(term in uncertain for term in ("hold the apply queue", "freeze every", "complete non-mutating inventory", "blind retry", "automatic delete", "remote rollback", "tie-break")):
        raise ExampleError("O-002 uncertain-write prohibitions are incomplete")


def find_binary(root: Path) -> Path:
    configured = os.environ.get("CODEMIE_GITOPS_BINARY")
    candidates = [Path(configured)] if configured else []
    candidates.extend(
        (
            root / "target/x86_64-unknown-linux-musl/release/codemie-gitops",
            root / "target/x86_64-unknown-linux-musl/debug/codemie-gitops",
            root / "target/release/codemie-gitops",
            root / "target/debug/codemie-gitops",
        )
    )
    for candidate in candidates:
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return candidate
    raise ExampleError("O-002 offline lint binary is unavailable; build it or set CODEMIE_GITOPS_BINARY")


def run_offline_lint(root: Path) -> None:
    binary = find_binary(root)
    clean_env = {"PATH": os.environ.get("PATH", "")}
    for relative in DECLARATIONS:
        result = subprocess.run(
            [str(binary), "lint", "--file", str(relative), "--repo-root", ".", "--output", "json"],
            cwd=REPOSITORY,
            env=clean_env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=300,
        )
        if result.returncode != 0:
            raise ExampleError("O-002 declaration failed offline lint")


def validate(root: Path = ROOT, *, offline_lint: bool = True) -> None:
    missing = [str(path) for path in REQUIRED_PATHS if not (root / path).is_file()]
    if missing:
        raise ExampleError("O-002 fixed example layout is incomplete")
    expected_examples = {path for path in REQUIRED_PATHS if path.parts[0] == "examples"}
    actual_examples = {
        path.relative_to(root)
        for path in (root / "examples").rglob("*")
        if path.is_file()
    }
    expected_runbooks = {path for path in REQUIRED_PATHS if path.parts[:2] == ("ops", "o002")}
    actual_runbooks = {
        path.relative_to(root)
        for path in (root / "ops/o002").rglob("*")
        if path.is_file()
    }
    if actual_examples != expected_examples or actual_runbooks != expected_runbooks:
        raise ExampleError("O-002 fixed example layout contains an unexpected file")
    validate_github((root / "examples/ci/github-actions.yml").read_text(encoding="utf-8"))
    validate_gitlab((root / "examples/ci/gitlab-ci.yml").read_text(encoding="utf-8"))
    validate_declarations(root / "examples/repository")
    validate_runbooks(root)
    if offline_lint:
        run_offline_lint(root)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--skip-offline-lint", action="store_true", help="structural tests only")
    args = parser.parse_args()
    try:
        validate(offline_lint=not args.skip_offline_lint)
    except (ExampleError, OSError, UnicodeDecodeError, subprocess.TimeoutExpired):
        print("O-002 example controls are invalid", file=sys.stderr)
        return 2
    print("O-002 local examples: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
