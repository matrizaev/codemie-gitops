from __future__ import annotations

import sys
import hashlib
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from check_o002_examples import (  # noqa: E402
    ExampleError,
    load_yaml_text,
    validate_declarations,
    validate_github,
    validate_gitlab,
    validate_runbooks,
)


class O002ExampleTests(unittest.TestCase):
    def setUp(self) -> None:
        self.github = (ROOT / "examples/ci/github-actions.yml").read_text(encoding="utf-8")
        self.gitlab = (ROOT / "examples/ci/gitlab-ci.yml").read_text(encoding="utf-8")

    def test_checked_in_structure_declarations_and_runbooks_pass(self) -> None:
        validate_github(self.github)
        validate_gitlab(self.gitlab)
        validate_declarations(ROOT / "examples/repository")
        validate_runbooks(ROOT)

    def test_provider_yaml_duplicate_key_is_rejected_structurally(self) -> None:
        with self.assertRaises(ExampleError):
            load_yaml_text("jobs:\n  build: {}\n  build: {}\n")

    def test_github_mutations_fail_closed(self) -> None:
        mutations = {
            "secret in build": self.github.replace("          make lint", "          echo ${{ secrets.CODEMIE_TOKEN }}\n          make lint", 1),
            "build secret environment": self.github.replace("    runs-on: ubuntu-latest", "    runs-on: ubuntu-latest\n    env:\n      CODEMIE_TOKEN: ${{ secrets.CODEMIE_TOKEN }}", 1),
            "pull request target": self.github.replace("  pull_request:", "  pull_request_target:", 1),
            "unprotected apply": self.github.replace("    environment:\n      name: codemie-apply\n", "", 1),
            "protected rebuild": self.github.replace("          set -euo pipefail\n          set +x", "          set -euo pipefail\n          cargo build --release\n          set +x", 1),
            "build test omitted": self.github.replace("          make test\n", "", 1),
            "release omitted": self.github.replace("          cargo build --locked --release\n", "", 1),
            "full checker omitted": self.github.replace("          python3 scripts/check_o002_examples.py\n", "", 1),
            "checker weakened": self.github.replace("          python3 scripts/check_o002_examples.py\n", "          python3 scripts/check_o002_examples.py --skip-offline-lint\n", 1),
            "build checksum omitted": self.github.replace("          ( cd o002-artifact && sha256sum codemie-gitops > codemie-gitops.sha256 )\n", "", 1),
            "checksum omitted": self.github.replace("          ( cd o002-artifact && sha256sum --check --strict codemie-gitops.sha256 )\n", "", 1),
            "checksum after cd": self.github.replace("          ( cd o002-artifact && sha256sum --check --strict codemie-gitops.sha256 )\n          cd examples/repository", "          cd examples/repository\n          ( cd o002-artifact && sha256sum --check --strict codemie-gitops.sha256 )", 1),
            "mask after apply": self.github.replace('          echo "::add-mask::$token"\n', "", 1).replace("          unset CODEMIE_TOKEN token", '          echo "::add-mask::$token"\n          unset CODEMIE_TOKEN token', 1),
            "output before mask": self.github.replace('          echo "::add-mask::$token"', '          printenv\n          echo "::add-mask::$token"', 1),
            "command before mask": self.github.replace('          echo "::add-mask::$token"', '          true\n          echo "::add-mask::$token"', 1),
            "secret flag": self.github.replace("codemie-gitops login", "codemie-gitops login --client-secret value", 1),
            "tls bypass": self.github.replace("codemie-gitops login", "codemie-gitops login --insecure", 1),
            "shell trace": self.github.replace("          set +x", "          set -x", 1),
            "token output": self.github.replace("          export CODEMIE_TOKEN", "          echo $token\n          export CODEMIE_TOKEN", 1),
            "token file": self.github.replace("          export CODEMIE_TOKEN", '          printf "%s" "$token" > token.txt\n          export CODEMIE_TOKEN', 1),
            "token transfer": self.github.replace("          export CODEMIE_TOKEN", "          cp token.txt o002-artifact/token.txt\n          export CODEMIE_TOKEN", 1),
        }
        for label, content in mutations.items():
            self.assertNotEqual(content, self.github, label)
            with self.subTest(label=label), self.assertRaises(ExampleError):
                validate_github(content)

    def test_gitlab_mutations_fail_closed(self) -> None:
        mutations = {
            "unprotected ref": self.gitlab.replace("$CI_COMMIT_REF_PROTECTED == \"true\" && ", "", 1),
            "automatic apply": self.gitlab.replace("      when: manual", "      when: on_success", 1),
            "unprotected runner": self.gitlab.replace("    - protected-runner", "    - shared-runner", 1),
            "no mutex": self.gitlab.replace("  resource_group: codemie-gitops-apply-codemie-apply\n", "", 1),
            "protected rebuild": self.gitlab.replace("    - set +x", "    - cargo build --release\n    - set +x", 1),
            "build token variable": self.gitlab.replace("  script:\n", "  variables:\n    CODEMIE_TOKEN: $MASKED_TOKEN\n  script:\n", 1),
            "build test omitted": self.gitlab.replace("    - make test\n", "", 1),
            "release omitted": self.gitlab.replace("    - cargo build --locked --release\n", "", 1),
            "full checker omitted": self.gitlab.replace("    - python3 scripts/check_o002_examples.py\n", "", 1),
            "checker weakened": self.gitlab.replace("    - python3 scripts/check_o002_examples.py\n", "    - python3 scripts/check_o002_examples.py --skip-offline-lint\n", 1),
            "build checksum omitted": self.gitlab.replace("    - ( cd o002-artifact && sha256sum codemie-gitops > codemie-gitops.sha256 )\n", "", 1),
            "apply checksum omitted": self.gitlab.replace("    - ( cd o002-artifact && sha256sum --check --strict codemie-gitops.sha256 )\n", "", 1),
            "dotenv": self.gitlab.replace("  artifacts:\n", "  artifacts:\n    reports:\n      dotenv: token.env\n", 1),
            "unsafe trace": self.gitlab.replace("    - set +x", "    - set -x", 1),
            "fresh login without runtime mask": self.gitlab.replace("    - export CODEMIE_TOKEN", '    - CODEMIE_TOKEN="$(../../o002-artifact/codemie-gitops login)"'),
            "missing pre-supplied token": self.gitlab.replace("    - export CODEMIE_TOKEN\n", "", 1),
            "simulated masking": self.gitlab.replace("    - export CODEMIE_TOKEN", '    - echo "::add-mask::$CODEMIE_TOKEN"\n    - export CODEMIE_TOKEN', 1),
            "literal assignment": self.gitlab.replace("    - export CODEMIE_TOKEN", "    - CODEMIE_TOKEN=literal-secret\n    - export CODEMIE_TOKEN", 1),
            "environment dump": self.gitlab.replace("    - export CODEMIE_TOKEN", "    - printenv\n    - export CODEMIE_TOKEN", 1),
            "token file": self.gitlab.replace("    - export CODEMIE_TOKEN", '    - printf "%s" "$CODEMIE_TOKEN" > token.txt\n    - export CODEMIE_TOKEN', 1),
            "token transfer": self.gitlab.replace("    - export CODEMIE_TOKEN", "    - cp token.txt o002-artifact/token.txt\n    - export CODEMIE_TOKEN", 1),
        }
        for label, content in mutations.items():
            self.assertNotEqual(content, self.gitlab, label)
            with self.subTest(label=label), self.assertRaises(ExampleError):
                validate_gitlab(content)

    def test_artifact_local_checksum_handoff_accepts_exact_bytes_and_rejects_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifact = root / "o002-artifact"
            artifact.mkdir()
            binary = artifact / "codemie-gitops"
            binary.write_bytes(b"reviewed-release-bytes")
            digest = hashlib.sha256(binary.read_bytes()).hexdigest()
            (artifact / "codemie-gitops.sha256").write_text(
                f"{digest}  codemie-gitops\n",
                encoding="utf-8",
            )
            passed = subprocess.run(
                ["sha256sum", "--check", "--strict", "codemie-gitops.sha256"],
                cwd=artifact,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )
            self.assertEqual(passed.returncode, 0)
            binary.write_bytes(b"replacement-bytes")
            failed = subprocess.run(
                ["sha256sum", "--check", "--strict", "codemie-gitops.sha256"],
                cwd=artifact,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )
            self.assertNotEqual(failed.returncode, 0)

    def test_missing_recovery_prohibitions_fail_closed(self) -> None:
        mutations = {
            "ops/o002/GIT_REVERT_RECOVERY.md": ("does not roll back", "does not change"),
            "ops/o002/WORKFLOW_ADOPTION.md": ("exactly once", "as needed"),
            "ops/o002/UNCERTAIN_WRITE.md": ("Blind retry", "Retry"),
        }
        for relative, (before, after) in mutations.items():
            with self.subTest(relative=relative), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                for source_relative in (
                    "README.md",
                    "examples/README.md",
                    "ops/o002/GIT_REVERT_RECOVERY.md",
                    "ops/o002/WORKFLOW_ADOPTION.md",
                    "ops/o002/UNCERTAIN_WRITE.md",
                ):
                    destination = root / source_relative
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    destination.write_text((ROOT / source_relative).read_text(encoding="utf-8"), encoding="utf-8")
                target = root / relative
                target.write_text(target.read_text(encoding="utf-8").replace(before, after, 1), encoding="utf-8")
                with self.assertRaises(ExampleError):
                    validate_runbooks(root)


if __name__ == "__main__":
    unittest.main()
