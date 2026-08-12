from __future__ import annotations

import copy
import hashlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any
from urllib.error import HTTPError
from urllib.parse import parse_qs, urlsplit
from urllib.request import Request


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import v000_target as v000  # noqa: E402

from v000_target import (  # noqa: E402
    Budget,
    QualificationError,
    ReadOnlyTransport,
    StagedBinary,
    _scan,
    _validate_skill_item,
    begin_execution_session,
    execute,
    load_workspace_dotenv,
    obtain_token,
    parse_dotenv_data,
    require_qualification_for_apply,
    run_probes,
    validate_runtime_gate,
    validate_declaration_root,
    validate_smoke_manifest,
    verify_staged_binary,
)


TARGET = "https://authorized-target.example.invalid/"
PROJECT = "authorized-project"
ACTOR = "authorized-actor@example.invalid"
PREFIX = "v000-run-"
WORKFLOW_DISPLAY_NAME = "Smoke Workflow"
PRODUCT_USER_AGENT = "codemie-gitops/0.1.0"


def valid_manifest(*, now: datetime | None = None) -> dict[str, Any]:
    current = now or datetime.now(timezone.utc)
    stamp = lambda value: value.astimezone(timezone.utc).isoformat().replace("+00:00", "Z")
    return {
        "schemaVersion": 1,
        "runId": PREFIX + "001",
        "target": TARGET,
        "authorization": {
            "project": PROJECT,
            "actor": ACTOR,
            "allowedKinds": ["Assistant", "Workflow", "Skill"],
            "durableRecordOwner": "verification-owner",
            "exclusiveWriter": {
                "confirmed": True,
                "confirmer": "platform-owner",
                "startsAt": stamp(current - timedelta(minutes=5)),
                "endsAt": stamp(current + timedelta(hours=1)),
                "sequenceEndsAt": stamp(current + timedelta(minutes=30)),
                "runPrefix": PREFIX,
                "noOtherWriterAttested": True,
            },
        },
        "declarations": [
            {"kind": "Assistant", "path": "assistant.yaml", "naturalKey": {"project": PROJECT, "slug": PREFIX + "assistant"}},
            {"kind": "Workflow", "path": "workflow.yaml", "naturalKey": {"project": PROJECT, "slug": PREFIX + "workflow"}},
            {"kind": "Skill", "path": "skill.yaml", "naturalKey": {"project": PROJECT, "name": PREFIX + "skill"}},
        ],
    }


class FakeResponse(io.BytesIO):
    def __init__(self, body: bytes, *, status: int = 200, request_id: str = "request-safe-1") -> None:
        super().__init__(body)
        self._status = status
        self.headers = {"X-Request-ID": request_id}

    def getcode(self) -> int:
        return self._status

    def close(self) -> None:
        super().close()


class ContractOpener:
    def __init__(self, *, mutation: str | None = None, canary: str = "") -> None:
        self.requests: list[Any] = []
        self.mutation = mutation
        self.canary = canary

    def open(self, request: Any, timeout: float) -> FakeResponse:
        self.requests.append(request)
        if request.get_header("User-agent") != PRODUCT_USER_AGENT:
            raise HTTPError(request.full_url, 403, "user-agent-rejected", {}, None)
        parsed = urlsplit(request.full_url)
        query = parse_qs(parsed.query)
        if parsed.path == "/v1/user":
            body: dict[str, Any] = {
                "email": ACTOR,
                "user_id": "authenticated-user-id",
                "is_admin": False,
                "is_maintainer": False,
                "projects": [{"name": PROJECT, "is_project_admin": True}],
                "additive": self.canary,
            }
            if self.mutation == "missing-user-id":
                body.pop("user_id")
            elif self.mutation == "personal-owner":
                body["projects"][0]["is_project_admin"] = False
        elif parsed.path == f"/v1/projects/{PROJECT}":
            body = {
                "name": PROJECT,
                "project_type": "personal",
                "created_by": "authenticated-user-id",
                "members": [{"user_id": "authenticated-user-id", "is_project_admin": False}],
                "future": self.canary,
            }
        elif parsed.path == "/v1/workflows":
            marketplace = query.get("scope") == ["marketplace"]
            meta_config: str | None = None
            if self.mutation == "workflow-collision":
                meta_config = json.dumps(
                    {
                        "codemie.epam.com/gitops/workflow-identity": {
                            "version": 2,
                            "project": PROJECT,
                            "creator_user_id": "authenticated-user-id",
                            "slug": PREFIX + "workflow",
                        }
                    }
                )
            elif self.mutation == "workflow-invalid-marker":
                meta_config = "{duplicate-invalid"
            display_collision = (
                self.mutation == "workflow-display-collision"
                or (self.mutation == "workflow-display-collision-project" and not marketplace)
                or (self.mutation == "workflow-display-collision-marketplace" and marketplace)
            )
            item = {"id": "workflow-1", "project": PROJECT, "name": WORKFLOW_DISPLAY_NAME if display_collision else "Existing", "meta_config": meta_config, "created_by": {"id": "authenticated-user-id"}, "user_abilities": ["read", "write"], "future": self.canary}
            if self.mutation == "missing-meta":
                item.pop("meta_config")
            body = {
                "data": [item],
                "pagination": {"page": int(query["page"][0]), "per_page": 100, "total": 1, "pages": 1},
                "future": self.canary,
            }
        elif parsed.path == "/v1/skills":
            filters = json.loads(query["filters"][0])
            exact_scan = "search" in filters
            item = {
                "id": "skill-1",
                "name": PREFIX + "skill" if self.mutation == "skill-collision" else "existing-skill",
                "project": PROJECT,
                "created_by": {"id": "authenticated-user-id"},
                "user_abilities": ["read", "write"],
                "future": self.canary,
            }
            if self.mutation == "missing-created":
                item.pop("created_by")
            items = [item] if not exact_scan or self.mutation == "skill-collision" else []
            body = {
                "skills": items,
                "page": int(query["page"][0]),
                "perPage": 100,
                "total": len(items),
                "pages": 1 if items else 0,
                "future": self.canary,
            }
        elif parsed.path == "/v1/index":
            body = {"data": [], "pagination": {"page": int(query["page"][0]), "per_page": 100, "total": 0, "pages": 0}, "future": self.canary}
        elif parsed.path.startswith("/v1/assistants/slug/"):
            if self.mutation == "assistant-collision":
                return FakeResponse(
                    json.dumps(
                        {"id": "assistant-1", "user_abilities": ["read", "write"], "future": self.canary}
                    ).encode()
                )
            raise HTTPError(request.full_url, 404, self.canary, {}, None)
        else:
            raise AssertionError(parsed.path)
        return FakeResponse(json.dumps(body).encode(), request_id=self.canary or "request-safe-1")


def stage_test_binary(root: Path) -> tuple[Path, str]:
    binary = root / "codemie-gitops"
    binary.write_text(
        "#!/bin/sh\n"
        "if [ \"$1\" = lint ]; then\n"
        "  previous=\n"
        "  for argument in \"$@\"; do\n"
        "    if [ \"$previous\" = --repo-root ] && [ -f \"$argument/invalid.yaml\" ]; then exit 8; fi\n"
        "    previous=$argument\n"
        "  done\n"
        "  exit 0\n"
        "fi\n"
        "if [ \"$1\" = login ]; then printf '%s\\n' fresh-token; exit 0; fi\n"
        "exit 9\n",
        encoding="utf-8",
    )
    binary.chmod(0o555)
    return binary, hashlib.sha256(binary.read_bytes()).hexdigest()


def write_concrete_declarations(root: Path, manifest: dict[str, Any]) -> None:
    for item in manifest["declarations"]:
        path = root / item["path"]
        path.parent.mkdir(parents=True, exist_ok=True)
        key_name = "name" if item["kind"] == "Skill" else "slug"
        path.write_text(
            "apiVersion: codemie.epam.com/v1alpha1\n"
            f"kind: {item['kind']}\n"
            "metadata:\n"
            f"  project: {item['naturalKey']['project']}\n"
            f"  {key_name}: {item['naturalKey'][key_name]}\n"
            + (f"spec:\n  name: {WORKFLOW_DISPLAY_NAME}\n" if item["kind"] == "Workflow" else "spec: {}\n"),
            encoding="utf-8",
        )


class V000ManifestAndGateTests(unittest.TestCase):
    def test_valid_closed_manifest_passes(self) -> None:
        document = valid_manifest()
        self.assertIs(validate_smoke_manifest(document), document)
        incomplete = json.loads((ROOT / "ops/v000/enterprise-smoke.example.json").read_text(encoding="utf-8"))
        with self.assertRaises(QualificationError):
            validate_smoke_manifest(incomplete)

    def test_datasource_and_missing_kind_mutations_fail_before_transport(self) -> None:
        mutations = []
        datasource_member = valid_manifest()
        datasource_member["datasources"] = []
        mutations.append(datasource_member)
        datasource_kind = valid_manifest()
        datasource_kind["declarations"][0]["kind"] = "Datasource"
        mutations.append(datasource_kind)
        datasource_path = valid_manifest()
        datasource_path["declarations"][0]["path"] = "run.local/datasource.yaml"
        mutations.append(datasource_path)
        outside_path = valid_manifest()
        outside_path["declarations"][0]["path"] = "../outside.yaml"
        mutations.append(outside_path)
        selector = valid_manifest()
        selector["authorization"]["kindSelector"] = "Assistant"
        mutations.append(selector)
        missing = valid_manifest()
        missing["declarations"] = missing["declarations"][:-1]
        mutations.append(missing)
        mixed_project = valid_manifest()
        mixed_project["declarations"][1]["naturalKey"]["project"] = "other-project"
        mutations.append(mixed_project)
        for document in mutations:
            calls = 0

            def factory(*_: Any) -> Any:
                nonlocal calls
                calls += 1
                raise AssertionError("transport must not construct")

            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                manifest_path = root / "manifest.json"
                manifest_path.write_text(json.dumps(document), encoding="utf-8")
                binary = root / "codemie-gitops"
                binary.write_bytes(b"fixed staged binary")
                binary.chmod(0o555)
                digest = hashlib.sha256(binary.read_bytes()).hexdigest()
                with self.subTest(document=document), self.assertRaises(QualificationError):
                    execute(
                        binary=binary,
                        expected_sha256=digest,
                        target=TARGET,
                        project=PROJECT,
                        manifest_path=manifest_path,
                        evidence_path=root / "evidence.json",
                        environment={"CODEMIE_TOKEN": "secret", "CODEMIE_TEST_PROJECT": PROJECT},
                        transport_factory=factory,
                    )
            self.assertEqual(calls, 0)

    def test_lint_isolated_to_declaration_root_and_ignores_unrelated_yaml(self) -> None:
        calls = 0

        def factory(*_: Any) -> Any:
            nonlocal calls
            calls += 1
            raise AssertionError("transport must not construct")

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            declaration_root = root / "run-v000.local"
            declaration_root.mkdir()
            manifest = valid_manifest()
            write_concrete_declarations(declaration_root, manifest)
            (root / "invalid.yaml").write_text("not: [valid", encoding="utf-8")
            manifest_path = root / "manifest.json"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            binary, digest = stage_test_binary(root)
            with self.assertRaises(QualificationError) as caught:
                execute(
                    binary=binary, expected_sha256=digest, target=TARGET, project=PROJECT,
                    manifest_path=manifest_path, evidence_path=root / "evidence.json",
                    environment={"CODEMIE_TOKEN": "secret", "CODEMIE_TEST_PROJECT": "wrong-project"},
                    credential_workspace=root, declaration_root=declaration_root,
                    transport_factory=factory,
                )
            self.assertEqual(caught.exception.category, "project-binding-invalid")
        self.assertEqual(calls, 0)

    def test_concrete_project_can_resolve_from_isolated_repository_config(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = valid_manifest()
            write_concrete_declarations(root, manifest)
            (root / ".codemie").mkdir()
            (root / ".codemie/config.yaml").write_text(f"project: {PROJECT}\n", encoding="utf-8")
            for item in manifest["declarations"]:
                path = root / item["path"]
                path.write_text(path.read_text(encoding="utf-8").replace(f"  project: {PROJECT}\n", "", 1), encoding="utf-8")
            manifest_path = root / "manifest.json"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            binary, digest = stage_test_binary(root)
            opener = ContractOpener()
            evidence = execute(
                binary=binary, expected_sha256=digest, target=TARGET, project=PROJECT,
                manifest_path=manifest_path, evidence_path=root / "evidence.json",
                environment={"CODEMIE_TOKEN": "secret", "CODEMIE_TEST_PROJECT": PROJECT},
                credential_workspace=root, declaration_root=root,
                transport_factory=lambda target, token: ReadOnlyTransport(target, token, opener=opener),
            )
            self.assertEqual(evidence["bindings"]["project"], "pass")

    def test_concrete_wrong_project_symlink_and_missing_workflow_name_fail_before_transport(self) -> None:
        for mutation in ("wrong-project", "symlink", "missing-workflow-name"):
            calls = 0

            def factory(*_: Any) -> Any:
                nonlocal calls
                calls += 1
                raise AssertionError("transport must not construct")

            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                manifest = valid_manifest()
                write_concrete_declarations(root, manifest)
                assistant = root / manifest["declarations"][0]["path"]
                if mutation == "wrong-project":
                    assistant.write_text(assistant.read_text(encoding="utf-8").replace(PROJECT, "other-project", 1), encoding="utf-8")
                else:
                    if mutation == "symlink":
                        target = root / "real-assistant.yaml"
                        assistant.replace(target)
                        assistant.symlink_to(target)
                    else:
                        workflow_item = next(item for item in manifest["declarations"] if item["kind"] == "Workflow")
                        workflow = root / workflow_item["path"]
                        workflow.write_text(
                            workflow.read_text(encoding="utf-8").replace(
                                f"spec:\n  name: {WORKFLOW_DISPLAY_NAME}\n",
                                "spec: {}\n",
                                1,
                            ),
                            encoding="utf-8",
                        )
                manifest_path = root / "manifest.json"
                manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
                binary, digest = stage_test_binary(root)
                with self.subTest(mutation=mutation), self.assertRaises(QualificationError):
                    execute(
                        binary=binary, expected_sha256=digest, target=TARGET, project=PROJECT,
                        manifest_path=manifest_path, evidence_path=root / "evidence.json",
                        environment={"CODEMIE_TOKEN": "secret", "CODEMIE_TEST_PROJECT": PROJECT},
                        credential_workspace=root, declaration_root=root, transport_factory=factory,
                    )
            self.assertEqual(calls, 0)

    def test_declaration_root_symlink_is_rejected_before_canonicalization(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory)
            real = parent / "run-real.local"
            real.mkdir()
            alias = parent / "run-alias.local"
            alias.symlink_to(real, target_is_directory=True)
            with self.assertRaises(QualificationError):
                validate_declaration_root(alias, parent)

    def test_runtime_actor_project_role_and_window_gates(self) -> None:
        manifest = valid_manifest()
        user = {"email": ACTOR, "user_id": "authenticated-user-id", "is_admin": False, "is_maintainer": False, "projects": [{"name": PROJECT, "is_project_admin": True}]}
        passed = validate_runtime_gate(
            manifest, target=TARGET, project=PROJECT, user=user,
        )
        self.assertEqual(set(passed.values()), {"pass"})
        cases = [
            {"user": {**user, "email": "different@example.invalid"}},
            {"project": "other-project"},
            {"user": {**user, "projects": [{"name": "other-project", "is_project_admin": True}]}},
        ]
        base = {
            "manifest": manifest, "target": TARGET, "project": PROJECT, "user": user,
        }
        for changes in cases:
            arguments = {**base, **changes}
            with self.subTest(changes=changes), self.assertRaises(QualificationError):
                validate_runtime_gate(**arguments)
        admin_without_membership = {"email": ACTOR, "user_id": "authenticated-user-id", "is_admin": True, "projects": []}
        with self.assertRaises(QualificationError):
            validate_runtime_gate(**{**base, "user": admin_without_membership})
        expired = valid_manifest(now=datetime.now(timezone.utc) - timedelta(hours=2))
        with self.assertRaises(QualificationError):
            validate_smoke_manifest(expired)


class V000CredentialAndBinaryTests(unittest.TestCase):
    def test_dotenv_parser_is_non_evaluating_and_closed(self) -> None:
        literal = "$(touch /tmp/must-not-run)"
        parsed = parse_dotenv_data(f"CODEMIE_URL={TARGET}\nCODEMIE_PASSWORD='{literal}'\nCODEMIE_TEST_PROJECT={PROJECT}\n")
        self.assertEqual(parsed["CODEMIE_PASSWORD"], literal)
        invalid = (
            "export CODEMIE_URL=x\n",
            "UNKNOWN=x\n",
            "CODEMIE_URL=x\nCODEMIE_URL=y\n",
            "CODEMIE_URL =x\n",
            "CODEMIE_URL= x\n",
            "CODEMIE_URL='unterminated\n",
        )
        for content in invalid:
            with self.subTest(content=content), self.assertRaises(QualificationError):
                parse_dotenv_data(content)

    def test_dotenv_file_requires_ignored_untracked_owner_only_regular_file(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "--quiet"], cwd=root, check=True)
            (root / ".gitignore").write_text(".env\n", encoding="utf-8")
            dotenv = root / ".env"
            dotenv.write_text(f"CODEMIE_URL={TARGET}\nCODEMIE_TEST_PROJECT={PROJECT}\n", encoding="utf-8")
            dotenv.chmod(0o600)
            values = load_workspace_dotenv(root, dotenv)
            self.assertEqual(set(values), {"CODEMIE_URL", "CODEMIE_TEST_PROJECT"})
            dotenv.chmod(0o644)
            with self.assertRaises(QualificationError):
                load_workspace_dotenv(root, dotenv)

    def test_checksum_mismatch_and_writable_binary_fail_before_authentication(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "codemie-gitops"
            binary.write_bytes(b"one")
            binary.chmod(0o555)
            digest = hashlib.sha256(b"one").hexdigest()
            self.assertEqual(verify_staged_binary(binary, digest), digest)
            binary.chmod(0o755)
            binary.write_bytes(b"two")
            binary.chmod(0o555)
            with self.assertRaises(QualificationError):
                verify_staged_binary(binary, digest)
            binary.chmod(0o755)
            with self.assertRaises(QualificationError):
                verify_staged_binary(binary, hashlib.sha256(b"two").hexdigest())

    def test_checksum_mutation_stops_before_transport_or_login(self) -> None:
        calls = 0

        def factory(*_: Any) -> Any:
            nonlocal calls
            calls += 1
            raise AssertionError("transport must not construct")

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = valid_manifest()
            write_concrete_declarations(root, manifest)
            manifest_path = root / "manifest.json"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            binary, digest = stage_test_binary(root)
            binary.chmod(0o755)
            binary.write_text("#!/bin/sh\nexit 99\n", encoding="utf-8")
            binary.chmod(0o555)
            with self.assertRaises(QualificationError) as caught:
                execute(
                    binary=binary, expected_sha256=digest, target=TARGET, project=PROJECT,
                    manifest_path=manifest_path, evidence_path=root / "evidence.json",
                    environment={"CODEMIE_TEST_PROJECT": PROJECT},
                    credential_workspace=root, declaration_root=root,
                    transport_factory=factory,
                )
            self.assertEqual(caught.exception.category, "checksum-mismatch")
        self.assertEqual(calls, 0)

    def test_relative_binary_is_anchored_once_and_not_resolved_through_path(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, digest = stage_test_binary(root)
            shadow = root / "shadow"
            shadow.mkdir()
            marker = root / "secret-canary"
            (shadow / "codemie-gitops").write_text(
                "#!/bin/sh\nprintf '%s' \"$SECRET_CANARY\" > \"$MARKER\"\nexit 77\n",
                encoding="utf-8",
            )
            (shadow / "codemie-gitops").chmod(0o555)
            declaration_root = root / "declarations"
            declaration_root.mkdir()
            previous = Path.cwd()
            os.chdir(root)
            try:
                for authored in (Path("codemie-gitops"), Path("./codemie-gitops")):
                    with self.subTest(authored=authored), StagedBinary.open(authored, digest) as binary:
                        result = binary.run(
                            ["lint"],
                            cwd=declaration_root,
                            env={"PATH": str(shadow), "SECRET_CANARY": "must-not-leak", "MARKER": str(marker)},
                            stdin=subprocess.DEVNULL,
                            stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL,
                            timeout=10,
                            check=False,
                        )
                        self.assertEqual(result.returncode, 0)
                        self.assertFalse(marker.exists())
                        token = obtain_token(
                            binary,
                            TARGET,
                            {
                                "PATH": str(shadow),
                                "CODEMIE_CLIENT_SECRET": "must-not-leak",
                                "SECRET_CANARY": "must-not-leak",
                                "MARKER": str(marker),
                            },
                            10,
                        )
                        self.assertEqual(token, "fresh-token")
                        self.assertFalse(marker.exists())
            finally:
                os.chdir(previous)
            self.assertTrue(source.exists())

    def test_sealed_copy_survives_path_replacement_and_source_inode_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "codemie-gitops"
            source.write_text("#!/bin/sh\nprintf original\n", encoding="utf-8")
            source.chmod(0o555)
            digest = hashlib.sha256(source.read_bytes()).hexdigest()
            with StagedBinary.open(source, digest) as binary:
                opened_source = root / "opened-source"
                source.replace(opened_source)
                source.write_text("#!/bin/sh\nprintf replacement\n", encoding="utf-8")
                source.chmod(0o555)
                opened_source.chmod(0o755)
                opened_source.write_text("#!/bin/sh\nprintf mutated-in-place\n", encoding="utf-8")
                opened_source.chmod(0o555)
                result = binary.run(
                    [],
                    stdin=subprocess.DEVNULL,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    timeout=10,
                    check=False,
                )
                self.assertEqual(result.returncode, 0)
                self.assertEqual(result.stdout, b"original")
            self.assertTrue(binary.closed)

    def test_qualification_proof_requires_same_open_binary_session_and_valid_digest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary_path, digest = stage_test_binary(root)
            opener = ContractOpener()
            with StagedBinary.open(binary_path, digest) as binary:
                session = begin_execution_session("token-canary")
                transport = ReadOnlyTransport(TARGET, session, opener=opener)
                with self.assertRaises(QualificationError) as caught:
                    run_probes(
                        transport,
                        valid_manifest(),
                        PROJECT,
                        binary,
                        begin_execution_session("token-canary"),
                        WORKFLOW_DISPLAY_NAME,
                    )
                self.assertEqual(caught.exception.category, "session-binding-invalid")
                self.assertEqual(opener.requests, [])
                result = run_probes(
                    transport,
                    valid_manifest(),
                    PROJECT,
                    binary,
                    session,
                    WORKFLOW_DISPLAY_NAME,
                )
                require_qualification_for_apply(
                    result.proof,
                    binary=binary,
                    session=session,
                    expected_digest=digest,
                )
                for other_session, other_digest in (
                    (begin_execution_session("token-canary"), digest),
                    (session, "b" * 64),
                    (session, "not-a-sha"),
                ):
                    with self.subTest(other_digest=other_digest), self.assertRaises(QualificationError):
                        require_qualification_for_apply(
                            result.proof,
                            binary=binary,
                            session=other_session,
                            expected_digest=other_digest,
                        )
                with StagedBinary.open(binary_path, digest) as other_binary, self.assertRaises(QualificationError):
                    require_qualification_for_apply(
                        result.proof,
                        binary=other_binary,
                        session=session,
                        expected_digest=digest,
                    )
            with self.assertRaises(QualificationError):
                require_qualification_for_apply(
                    result.proof,
                    binary=binary,
                    session=session,
                    expected_digest=digest,
                )

    def test_missing_linux_sealing_support_fails_with_fixed_category(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            binary_path, digest = stage_test_binary(Path(directory))
            with mock.patch.object(v000, "fcntl", None), self.assertRaises(QualificationError) as caught:
                StagedBinary.open(binary_path, digest)
            self.assertEqual(caught.exception.category, "binary-platform-unsupported")


class V000TransportTests(unittest.TestCase):
    def test_nested_base_path_is_preserved_for_probe_routes(self) -> None:
        class NestedBaseServer:
            def __init__(self) -> None:
                self.requests: list[Any] = []

            def open(self, request: Any, timeout: float) -> FakeResponse:
                self.requests.append(request)
                return FakeResponse(b'{"ok":true}')

        fake_server = NestedBaseServer()
        transport = ReadOnlyTransport(
            "https://authorized-target.example.invalid/code-assistant-api/",
            "secret",
            opener=fake_server,
        )
        self.assertEqual(transport.get_json("/v1/user", probe="nested-base"), {"ok": True})
        self.assertEqual(
            fake_server.requests[0].full_url,
            "https://authorized-target.example.invalid/code-assistant-api/v1/user",
        )
        self.assertEqual(fake_server.requests[0].get_header("Authorization"), "Bearer secret")
        self.assertEqual(fake_server.requests[0].get_header("User-agent"), PRODUCT_USER_AGENT)

    def test_ambiguous_or_escaping_target_base_paths_fail_before_request(self) -> None:
        class ZeroRequestServer:
            def __init__(self) -> None:
                self.requests: list[Any] = []

            def open(self, request: Any, timeout: float) -> FakeResponse:
                self.requests.append(request)
                raise AssertionError("invalid target must not issue a request")

        invalid_targets = (
            "https://authorized-target.example.invalid/code-assistant-api/../admin/",
            "https://authorized-target.example.invalid/code-assistant-api/./v1/",
            "https://authorized-target.example.invalid/code-assistant-api//v1/",
            "https://authorized-target.example.invalid/code-assistant-api%2fv1/",
            "https://authorized-target.example.invalid/code-assistant-api%2Fv1/",
            "https://authorized-target.example.invalid/code-assistant-api%5cv1/",
            "https://authorized-target.example.invalid/code-assistant-api\\v1/",
            "https://authorized-target.example.invalid/code assistant-api/",
            "https://user@authorized-target.example.invalid/code-assistant-api/",
            "https://authorized-target.example.invalid/code-assistant-api/?query=1",
            "https://authorized-target.example.invalid/code-assistant-api/#fragment",
            "https://authorized-target.example.invalid/code-assistant-api/\x7f",
        )
        for target in invalid_targets:
            fake_server = ZeroRequestServer()
            with self.subTest(target=target), self.assertRaises(QualificationError) as caught:
                ReadOnlyTransport(target, "secret", opener=fake_server)
            self.assertEqual(caught.exception.category, "target-invalid")
            self.assertEqual(fake_server.requests, [])

        fake_server = ZeroRequestServer()
        transport = ReadOnlyTransport(TARGET, "secret", opener=fake_server)
        for route in ("/../v1/user", "/./v1/user", "/v1/%2e%2e/user", "/v1%2fadmin", "/v1\\user"):
            with self.subTest(route=route), self.assertRaises(QualificationError) as caught:
                transport.get_json(route, probe="invalid-route")
            self.assertEqual(caught.exception.category, "route-invalid")
        self.assertEqual(fake_server.requests, [])

    def test_same_origin_and_cross_origin_redirects_are_not_followed(self) -> None:
        class RedirectServer:
            def __init__(self, location: str) -> None:
                self.location = location
                self.requests: list[Any] = []

            def open(self, request: Any, timeout: float) -> FakeResponse:
                self.requests.append(request)
                raise HTTPError(request.full_url, 302, "redirect-canary", {"Location": self.location}, None)

        for location in ("/capture", "https://other.example.invalid/capture"):
            fake_server = RedirectServer(location)
            transport = ReadOnlyTransport(TARGET, "redirect-secret", opener=fake_server)
            with self.assertRaises(QualificationError) as caught:
                transport.get_json("/redirect", probe="redirect")
            self.assertEqual(caught.exception.category, "redirect-rejected")
            self.assertEqual(len(fake_server.requests), 1)
            self.assertNotIn("capture", urlsplit(fake_server.requests[0].full_url).path)
            self.assertEqual(fake_server.requests[0].get_header("Authorization"), "Bearer redirect-secret")
            self.assertEqual(fake_server.requests[0].get_header("User-agent"), PRODUCT_USER_AGENT)

    def test_fake_server_rejects_python_urllib_and_accepts_exact_product_user_agent(self) -> None:
        fake_server = ContractOpener()
        python_request = Request(
            TARGET + "v1/user",
            headers={"User-Agent": f"Python-urllib/{sys.version_info.major}.{sys.version_info.minor}"},
            method="GET",
        )
        with self.assertRaises(HTTPError) as caught:
            fake_server.open(python_request, timeout=1.0)
        self.assertEqual(caught.exception.code, 403)

        transport = ReadOnlyTransport(TARGET, "secret", opener=fake_server)
        transport.get_json("/v1/user", probe="user-agent")
        self.assertEqual(v000._product_user_agent(), PRODUCT_USER_AGENT)
        self.assertEqual(fake_server.requests[-1].get_header("User-agent"), PRODUCT_USER_AGENT)

    def test_transport_surface_and_observed_methods_are_get_only(self) -> None:
        opener = ContractOpener()
        transport = ReadOnlyTransport(TARGET, "secret", opener=opener)
        transport.get_json("/v1/user", probe="user")
        self.assertFalse(hasattr(transport, "post"))
        self.assertFalse(hasattr(transport, "put"))
        self.assertFalse(hasattr(transport, "delete"))
        self.assertEqual({request.get_method() for request in opener.requests}, {"GET"})
        self.assertEqual(
            {request.get_header("User-agent") for request in opener.requests},
            {PRODUCT_USER_AGENT},
        )

    def test_body_timeout_and_json_faults_fail_with_fixed_categories(self) -> None:
        class OneResponseOpener:
            def __init__(self, response: bytes | Exception) -> None:
                self.response = response

            def open(self, request: Any, timeout: float) -> FakeResponse:
                if isinstance(self.response, Exception):
                    raise self.response
                return FakeResponse(self.response)

        cases = (
            (b'{"body":"body-canary-that-is-too-long"}', Budget(body_bytes=8), "body-limit-exceeded"),
            (b'{"a":1,"a":2}', Budget(), "json-incompatible"),
            (TimeoutError("exception-canary"), Budget(), "connectivity-failed"),
        )
        for response, budget, category in cases:
            transport = ReadOnlyTransport(TARGET, "token-canary", budget=budget, opener=OneResponseOpener(response))
            with self.subTest(category=category), self.assertRaises(QualificationError) as caught:
                transport.get_json("/v1/user", probe="fault")
            self.assertEqual(caught.exception.category, category)
            self.assertNotIn("canary", str(caught.exception))

    def test_page_and_item_budgets_and_page_zero_origin_fail_closed(self) -> None:
        class SkillPages:
            def __init__(self, *, returned_page: int | None = None, total: int = 2) -> None:
                self.returned_page = returned_page
                self.total = total

            def open(self, request: Any, timeout: float) -> FakeResponse:
                page = int(parse_qs(urlsplit(request.full_url).query)["page"][0])
                body = {
                    "skills": [{"id": f"id-{page}", "name": "skill", "project": PROJECT, "created_by": None, "user_abilities": ["read"]}],
                    "page": self.returned_page if self.returned_page is not None else page,
                    "perPage": 100, "total": self.total, "pages": (self.total + 99) // 100,
                }
                return FakeResponse(json.dumps(body).encode())

        for opener, budget, category in (
            (SkillPages(total=101), Budget(pages=1), "pagination-limit-exceeded"),
            (SkillPages(total=2), Budget(items=1), "pagination-limit-exceeded"),
            (SkillPages(returned_page=1, total=1), Budget(), "pagination-incompatible"),
        ):
            transport = ReadOnlyTransport(TARGET, "secret", budget=budget, opener=opener)
            with self.subTest(category=category), self.assertRaises(QualificationError) as caught:
                _scan(transport, route="/v1/skills", query_for_page=lambda page: {"page": str(page)}, list_key="skills", pagination_style="flat", probe="skill-page-0", validate_item=_validate_skill_item)
            self.assertEqual(caught.exception.category, category)


class V000EvidenceTests(unittest.TestCase):
    def test_unrelated_skill_and_unmarked_workflow_pass_full_sanitized_probe(self) -> None:
        canaries = ("token-canary", "body-canary", "exception-canary")
        opener = ContractOpener(canary="body-canary")
        manifest = valid_manifest()
        with tempfile.TemporaryDirectory() as directory:
            binary_path, digest = stage_test_binary(Path(directory))
            with StagedBinary.open(binary_path, digest) as binary:
                session = begin_execution_session(canaries[0])
                transport = ReadOnlyTransport(TARGET, session, opener=opener)
                result = run_probes(
                    transport,
                    manifest,
                    PROJECT,
                    binary,
                    session,
                    WORKFLOW_DISPLAY_NAME,
                )
                require_qualification_for_apply(
                    result.proof,
                    binary=binary,
                    session=session,
                    expected_digest=digest,
                )
                evidence = result.evidence
        rendered = json.dumps(evidence)
        self.assertEqual(evidence["stagedBinarySha256"], digest)
        self.assertGreater(evidence["observations"]["workflowPage0Items"], 0)
        self.assertGreater(evidence["observations"]["skillPage0Items"], 0)
        self.assertEqual({request.get_method() for request in opener.requests}, {"GET"})
        self.assertEqual(
            {request.get_header("User-agent") for request in opener.requests},
            {PRODUCT_USER_AGENT},
        )
        self.assertTrue(any("/v1/index" in request.full_url for request in opener.requests))
        skill_filters = [
            json.loads(parse_qs(urlsplit(request.full_url).query)["filters"][0])
            for request in opener.requests
            if urlsplit(request.full_url).path == "/v1/skills"
        ]
        self.assertEqual(len(skill_filters), 2)
        self.assertNotIn("search", skill_filters[0])
        self.assertEqual(skill_filters[1]["search"], PREFIX + "skill")
        self.assertEqual(
            evidence["naturalKeyAbsence"],
            {"Assistant": "pass", "Workflow": "pass", "Skill": "pass"},
        )
        for canary in canaries:
            self.assertNotIn(canary, rendered)
        self.assertNotIn(TARGET, rendered)
        self.assertNotIn(ACTOR, rendered)
        self.assertNotIn("session", rendered.lower())
        self.assertNotIn("digest", json.dumps(evidence["bindings"]).lower())

    def test_each_smoke_natural_key_collision_fails_after_get_only_scans(self) -> None:
        for mutation in (
            "assistant-collision",
            "workflow-collision",
            "workflow-display-collision-project",
            "workflow-display-collision-marketplace",
            "skill-collision",
        ):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                binary_path, digest = stage_test_binary(Path(directory))
                opener = ContractOpener(mutation=mutation, canary="body-canary")
                session = begin_execution_session("token-canary")
                transport = ReadOnlyTransport(TARGET, session, opener=opener)
                with StagedBinary.open(binary_path, digest) as binary, self.assertRaises(QualificationError) as caught:
                    run_probes(
                        transport,
                        valid_manifest(),
                        PROJECT,
                        binary,
                        session,
                        WORKFLOW_DISPLAY_NAME,
                    )
                self.assertEqual(caught.exception.category, "identity-collision")
                self.assertTrue(opener.requests)
                self.assertEqual({request.get_method() for request in opener.requests}, {"GET"})
                self.assertFalse(any(request.get_method() in {"POST", "PUT", "PATCH", "DELETE"} for request in opener.requests))
                self.assertNotIn(PREFIX, str(caught.exception))

    def test_invalid_target_workflow_marker_fails_closed_without_write(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            binary_path, digest = stage_test_binary(Path(directory))
            opener = ContractOpener(mutation="workflow-invalid-marker")
            session = begin_execution_session("token-canary")
            transport = ReadOnlyTransport(TARGET, session, opener=opener)
            with StagedBinary.open(binary_path, digest) as binary, self.assertRaises(QualificationError) as caught:
                run_probes(
                    transport,
                    valid_manifest(),
                    PROJECT,
                    binary,
                    session,
                    WORKFLOW_DISPLAY_NAME,
                )
            self.assertEqual(caught.exception.category, "identity-marker-invalid")
            self.assertEqual({request.get_method() for request in opener.requests}, {"GET"})

    def test_missing_consumed_member_fails_without_canary_leak(self) -> None:
        for mutation in ("missing-user-id", "missing-meta", "missing-created"):
            with tempfile.TemporaryDirectory() as directory:
                binary_path, digest = stage_test_binary(Path(directory))
                with StagedBinary.open(binary_path, digest) as binary, self.subTest(mutation=mutation), self.assertRaises(QualificationError) as caught:
                    session = begin_execution_session("token-canary")
                    transport = ReadOnlyTransport(
                        TARGET,
                        session,
                        opener=ContractOpener(mutation=mutation, canary="body-canary"),
                    )
                    run_probes(
                        transport,
                        valid_manifest(),
                        PROJECT,
                        binary,
                        session,
                        WORKFLOW_DISPLAY_NAME,
                    )
            self.assertEqual(caught.exception.category, "response-incompatible")
            self.assertNotIn("canary", str(caught.exception))

    def test_execute_persists_only_sanitized_sha_bound_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary, digest = stage_test_binary(root)
            manifest = valid_manifest()
            write_concrete_declarations(root, manifest)
            manifest_path = root / "manifest.json"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            evidence_path = root / "evidence.json"
            opener = ContractOpener(canary="body-canary")

            def factory(target: str, session: v000.ExecutionSession) -> ReadOnlyTransport:
                return ReadOnlyTransport(target, session, opener=opener)

            result = execute(
                binary=binary, expected_sha256=digest, target=TARGET, project=PROJECT,
                manifest_path=manifest_path, evidence_path=evidence_path,
                environment={"CODEMIE_TOKEN": "token-canary", "CODEMIE_TEST_PROJECT": PROJECT},
                credential_workspace=root,
                declaration_root=root,
                transport_factory=factory,
            )
            persisted = evidence_path.read_text(encoding="utf-8")
            self.assertEqual(json.loads(persisted), result)
            self.assertEqual(result["stagedBinarySha256"], digest)
            for canary in ("token-canary", "body-canary", ACTOR, TARGET, PREFIX):
                self.assertNotIn(canary, persisted)
            self.assertEqual(evidence_path.stat().st_mode & 0o777, 0o600)


if __name__ == "__main__":
    unittest.main()
