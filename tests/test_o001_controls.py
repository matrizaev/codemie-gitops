from __future__ import annotations

import copy
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
INVENTORY = ROOT / "scripts/o001_inventory.py"
CONTROLS = ROOT / "scripts/check_o001_controls.py"
sys.path.insert(0, str(ROOT / "scripts"))

from check_o001_controls import (  # noqa: E402
    ControlError,
    _load_json,
    validate_github_workflow,
    validate_gitlab_pipeline,
    validate_policy,
)
from o001_inventory import analyze_inventory  # noqa: E402


def run(
    command: list[str], *, env: dict[str, str] | None = None
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=ROOT,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )


def valid_activation_evidence() -> dict[str, Any]:
    return {
        "schemaVersion": 1,
        "environment": "production",
        "github": {
            "protectedEnvironmentEnabled": True,
            "requiredReviewersEnabled": True,
            "deploymentBranchPolicyEnabled": True,
            "sameEnvironmentRunA": "github/run-a",
            "sameEnvironmentRunB": "github/run-b",
            "sameEnvironmentOverlapObserved": False,
            "differentEnvironment": "staging",
            "differentEnvironmentRun": "github/run-c",
            "differentEnvironmentIndependentObserved": True,
            "artifactPromotionRun": "github/run-d",
            "artifactChecksumVerified": True,
            "verifiedByRole": "Release Engineering",
            "verifiedAt": "2026-08-11T10:00:00Z",
        },
        "gitlab": {
            "protectedEnvironmentEnabled": True,
            "requiredApprovalEnabled": True,
            "protectedVariablesEnabled": True,
            "deploymentBranchPolicyEnabled": True,
            "muslBuildRunnerReady": True,
            "protectedRunnerEnabled": True,
            "protectedRefEnabled": True,
            "pipelineVariableOverridesRestricted": True,
            "environmentScopedSecrets": ["CODEMIE_TOKEN", "CODEMIE_URL"],
            "secretEnvironmentScope": "production",
            "sameEnvironmentPipelineA": "gitlab/pipeline-a",
            "sameEnvironmentPipelineB": "gitlab/pipeline-b",
            "sameEnvironmentOverlapObserved": False,
            "differentEnvironment": "staging",
            "differentEnvironmentPipeline": "gitlab/pipeline-c",
            "differentEnvironmentIndependentObserved": True,
            "artifactPromotionPipeline": "gitlab/pipeline-d",
            "artifactChecksumVerified": True,
            "verifiedByRole": "Release Engineering",
            "verifiedAt": "2026-08-11T10:00:00Z",
        },
        "writerGovernance": {
            "freezeEvidence": "change/freeze-123",
            "inventoryEvidence": "change/inventory-123",
            "inventoryClean": True,
            "externalWritersFrozen": True,
            "approvedByRole": "CodeMie Platform Administrator",
        },
    }


def run_activation_evidence(evidence: dict[str, Any]) -> subprocess.CompletedProcess[str]:
    with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8") as handle:
        json.dump(evidence, handle)
        handle.flush()
        return run(
            [
                sys.executable,
                str(CONTROLS),
                "--activation-evidence",
                handle.name,
            ]
        )


class O001InventoryTests(unittest.TestCase):
    def test_clean_fixture_passes(self) -> None:
        result = run(
            [
                sys.executable,
                str(INVENTORY),
                "--input",
                "ops/o001/fixtures/inventory-clean.json",
            ]
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["status"], "pass")
        self.assertEqual(report["workflow"]["unmarkedRecords"], 1)

    def test_broken_fixture_reports_all_identity_failures(self) -> None:
        result = run(
            [
                sys.executable,
                str(INVENTORY),
                "--input",
                "ops/o001/fixtures/inventory-broken.json",
            ]
        )
        self.assertEqual(result.returncode, 1, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["status"], "fail")
        self.assertEqual(report["workflow"]["invalidMarkers"], 1)
        self.assertEqual(len(report["workflow"]["duplicateNaturalKeys"]), 1)
        self.assertEqual(report["skill"]["invalidRecords"], 1)
        self.assertEqual(len(report["skill"]["duplicateNaturalKeys"]), 1)
        self.assertEqual(report["datasource"]["invalidRecords"], 1)
        self.assertEqual(len(report["datasource"]["duplicateNaturalKeys"]), 1)

    def test_boolean_workflow_marker_version_is_invalid(self) -> None:
        snapshot = json.loads(
            (ROOT / "ops/o001/fixtures/inventory-clean.json").read_text(
                encoding="utf-8"
            )
        )
        snapshot["workflows"][0]["marker"]["version"] = True
        report = analyze_inventory(snapshot)
        self.assertEqual(report["status"], "fail")
        self.assertEqual(report["workflow"]["invalidMarkers"], 1)
        self.assertEqual(report["workflow"]["validMarkers"], 0)

    def test_malformed_snapshot_never_echoes_value(self) -> None:
        canary = "InventoryCanaryA9!"
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8") as snapshot:
            json.dump({"unexpected": canary}, snapshot)
            snapshot.flush()
            result = run(
                [sys.executable, str(INVENTORY), "--input", snapshot.name]
            )
        self.assertEqual(result.returncode, 2)
        self.assertNotIn(canary, result.stderr)
        self.assertEqual(result.stdout, "")

    def test_inventory_cli_rejects_duplicate_names_and_nonstandard_constants(
        self,
    ) -> None:
        documents = (
            '{"workflows":[{"markerPresent":true}],"workflows":[],'
            '"skills":[],"datasources":[]}',
            '{"workflows":[{"markerPresent":true,"marker":{'
            '"version":2,"version":1,"project":"p","slug":"s"}}],'
            '"skills":[],"datasources":[]}',
            '{"workflows":[],"skills":[NaN],"datasources":[]}',
            '{"workflows":[],"skills":[],"datasources":[Infinity]}',
        )
        for document in documents:
            with self.subTest(document=document[:32]):
                with tempfile.NamedTemporaryFile(
                    mode="w", encoding="utf-8"
                ) as snapshot:
                    snapshot.write(document)
                    snapshot.flush()
                    result = run(
                        [sys.executable, str(INVENTORY), "--input", snapshot.name]
                    )
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")
                self.assertEqual(result.stderr, "O-001 inventory input is invalid\n")


class O001ControlTests(unittest.TestCase):
    def test_checked_in_controls_and_local_drills_pass(self) -> None:
        result = run([sys.executable, str(CONTROLS)])
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["status"], "local-pass")
        self.assertEqual(report["activationStatus"], "required")
        self.assertIn("exact-artifact-promotion", report["checks"])

    def test_commented_yaml_fragments_cannot_spoof_controls(self) -> None:
        github = (ROOT / ".github/workflows/codemie-gitops-apply.yml").read_text(
            encoding="utf-8"
        )
        spoofed_github = github.replace("  queue: max", "  # queue: max", 1)
        spoofed_github += "\n# queue: max\n"
        with self.assertRaises(ControlError):
            validate_github_workflow(spoofed_github)

        gitlab = (ROOT / ".gitlab-ci.yml").read_text(encoding="utf-8")
        spoofed_gitlab = gitlab.replace(
            "  resource_group: codemie-gitops-apply-production",
            "  # resource_group: codemie-gitops-apply-production",
            1,
        )
        spoofed_gitlab += "\n# resource_group: codemie-gitops-apply-production\n"
        with self.assertRaises(ControlError):
            validate_gitlab_pipeline(spoofed_gitlab)

    def test_gitlab_dynamic_environment_alias_is_rejected(self) -> None:
        gitlab = (ROOT / ".gitlab-ci.yml").read_text(encoding="utf-8")
        dynamic = gitlab.replace("    name: production", '    name: "$TARGET_ENVIRONMENT"', 1)
        with self.assertRaises(ControlError):
            validate_gitlab_pipeline(dynamic)

    def test_duplicate_provider_mapping_keys_are_rejected(self) -> None:
        github = (ROOT / ".github/workflows/codemie-gitops-apply.yml").read_text(
            encoding="utf-8"
        )
        gitlab = (ROOT / ".gitlab-ci.yml").read_text(encoding="utf-8")
        cases = (
            (
                validate_github_workflow,
                github
                + "\nconcurrency:\n  group: attacker-controlled\n  queue: max\n",
            ),
            (
                validate_github_workflow,
                github
                + '\n"concurrency":\n  group: attacker-controlled\n  queue: max\n',
            ),
            (
                validate_github_workflow,
                github
                + '\n"concurrency" :\n  group: attacker-controlled\n  queue: max\n',
            ),
            (
                validate_github_workflow,
                github
                + "\n? concurrency\n: {group: attacker-controlled, queue: max}\n",
            ),
            (
                validate_github_workflow,
                github
                + "\n{concurrency: {group: attacker-controlled, queue: max}}\n",
            ),
            (
                validate_github_workflow,
                github + "\n<<: *attacker-controlled\n",
            ),
            (
                validate_github_workflow,
                github.replace("  queue: max", "  queue: max\n  queue: min", 1),
            ),
            (
                validate_github_workflow,
                github.replace(
                    '        run: ./o001-artifact/codemie-gitops apply --file "$DECLARATION_PATH" --repo-root "$GITHUB_WORKSPACE"',
                    '        run: ./o001-artifact/codemie-gitops apply --file "$DECLARATION_PATH" --repo-root "$GITHUB_WORKSPACE"\n'
                    "        run: echo bypass",
                    1,
                ),
            ),
            (
                validate_github_workflow,
                github.replace(
                    '        run: ./o001-artifact/codemie-gitops apply --file "$DECLARATION_PATH" --repo-root "$GITHUB_WORKSPACE"',
                    '        run: ./o001-artifact/codemie-gitops apply --file "$DECLARATION_PATH" --repo-root "$GITHUB_WORKSPACE"\n'
                    '        "run": echo bypass',
                    1,
                ),
            ),
            (
                validate_gitlab_pipeline,
                gitlab
                + "\ncodemie_gitops_production_apply:\n  script:\n    - echo bypass\n",
            ),
            (
                validate_gitlab_pipeline,
                gitlab
                + '\n"codemie_gitops_production_apply":\n  script:\n    - echo bypass\n',
            ),
            (
                validate_gitlab_pipeline,
                gitlab
                + '\n"codemie_gitops_production_apply" :\n'
                "  script:\n    - echo bypass\n",
            ),
            (
                validate_gitlab_pipeline,
                gitlab.replace(
                    "  resource_group: codemie-gitops-apply-production",
                    "  resource_group: codemie-gitops-apply-production\n"
                    "  resource_group: bypass",
                    1,
                ),
            ),
            (
                validate_gitlab_pipeline,
                gitlab.replace(
                    "      when: manual",
                    "      when: manual\n      when: on_success",
                    1,
                ),
            ),
            (
                validate_gitlab_pipeline,
                gitlab.replace(
                    "    - if:",
                    "    - 'if':",
                    1,
                ),
            ),
            (
                validate_gitlab_pipeline,
                gitlab.replace(
                    "    - if:",
                    "    - !override if:",
                    1,
                ),
            ),
            (
                validate_gitlab_pipeline,
                gitlab.replace(
                    "    - if:",
                    "    - {if: true, when: on_success}",
                    1,
                ),
            ),
        )
        for validator, content in cases:
            with self.subTest(validator=validator.__name__):
                with self.assertRaises(ControlError):
                    validator(content)

    def test_policy_schema_and_gate_versions_are_strict_integers(self) -> None:
        policy = json.loads(
            (ROOT / "ops/o001/control-policy.json").read_text(encoding="utf-8")
        )
        for field, value in (("schemaVersion", True), ("gate", False)):
            with self.subTest(field=field):
                invalid = copy.deepcopy(policy)
                if field == "schemaVersion":
                    invalid["schemaVersion"] = value
                else:
                    invalid["inventory"]["gates"]["invalidWorkflowMarkers"] = value
                with self.assertRaises(ControlError):
                    validate_policy(invalid)

    def test_json_controls_reject_duplicate_names_and_nonstandard_constants(
        self,
    ) -> None:
        policy = (ROOT / "ops/o001/control-policy.json").read_text(encoding="utf-8")
        duplicate_policy = policy.replace(
            '"schemaVersion": 1',
            '"schemaVersion": 2, "schemaVersion": 1',
            1,
        )
        invalid_documents = (
            duplicate_policy,
            '{"schemaVersion": NaN}',
            '{"schemaVersion": Infinity}',
        )
        for document in invalid_documents:
            with self.subTest(document=document[:24]):
                with tempfile.NamedTemporaryFile(
                    mode="w", encoding="utf-8"
                ) as artifact:
                    artifact.write(document)
                    artifact.flush()
                    with self.assertRaises(ControlError):
                        _load_json(Path(artifact.name))

        evidence = json.dumps(valid_activation_evidence())
        duplicate_activation = evidence.replace(
            '"protectedEnvironmentEnabled": true',
            '"protectedEnvironmentEnabled": false, '
            '"protectedEnvironmentEnabled": true',
            1,
        )
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8") as artifact:
            artifact.write(duplicate_activation)
            artifact.flush()
            result = run(
                [
                    sys.executable,
                    str(CONTROLS),
                    "--activation-evidence",
                    artifact.name,
                ]
            )
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, "")
        self.assertEqual(result.stderr, "O-001 control check failed\n")

    def test_ci_preflight_requires_inventory_freeze_and_evidence(self) -> None:
        env = os.environ.copy()
        env.update(
            {
                "TARGET_ENVIRONMENT": "production",
                "O001_MODE": "apply",
                "CONTROL_EVIDENCE": "change/CR-123",
                "INVENTORY_APPROVED": "true",
                "WRITER_FREEZE_APPROVED": "true",
            }
        )
        result = run([sys.executable, str(CONTROLS), "--ci-preflight"], env=env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["status"], "preflight-pass")

        env["WRITER_FREEZE_APPROVED"] = "false"
        rejected = run(
            [sys.executable, str(CONTROLS), "--ci-preflight"], env=env
        )
        self.assertEqual(rejected.returncode, 2)
        self.assertEqual(rejected.stdout, "")

    def test_activation_evidence_gate_is_reproducible(self) -> None:
        result = run_activation_evidence(valid_activation_evidence())
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            json.loads(result.stdout)["activation"]["status"], "activation-pass"
        )

    def test_activation_evidence_rejects_incomplete_provider_controls(self) -> None:
        cases = (
            (None, "schemaVersion", True),
            ("github", "deploymentBranchPolicyEnabled", False),
            ("github", "sameEnvironmentRunB", "github/run-a"),
            ("github", "differentEnvironment", "production"),
            ("github", "verifiedAt", "nonsense"),
            ("gitlab", "requiredApprovalEnabled", False),
            ("gitlab", "muslBuildRunnerReady", False),
            ("gitlab", "protectedRunnerEnabled", False),
            ("gitlab", "protectedRefEnabled", False),
            ("gitlab", "pipelineVariableOverridesRestricted", False),
            ("gitlab", "environmentScopedSecrets", ["CODEMIE_TOKEN"]),
            ("gitlab", "secretEnvironmentScope", "staging"),
            ("gitlab", "sameEnvironmentPipelineB", "gitlab/pipeline-a"),
            ("gitlab", "differentEnvironment", "production"),
        )
        for section, field, value in cases:
            with self.subTest(section=section, field=field):
                evidence = valid_activation_evidence()
                target = evidence if section is None else evidence[section]
                target[field] = value
                result = run_activation_evidence(evidence)
                self.assertEqual(result.returncode, 1)
                self.assertEqual(result.stdout, "")

    def test_freeze_and_inventory_evidence_must_be_distinct(self) -> None:
        evidence = valid_activation_evidence()
        evidence["writerGovernance"]["inventoryEvidence"] = evidence[
            "writerGovernance"
        ]["freezeEvidence"]
        result = run_activation_evidence(evidence)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, "")

    def test_incomplete_activation_template_cannot_pass(self) -> None:
        result = run(
            [
                sys.executable,
                str(CONTROLS),
                "--activation-evidence",
                "ops/o001/activation-evidence.example.json",
            ]
        )
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, "")


if __name__ == "__main__":
    unittest.main()
