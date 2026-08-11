#!/usr/bin/env python3
"""Bounded GET-only target qualification and closed V-003 pre-write gates."""

from __future__ import annotations

import argparse
import hashlib
import hmac
import importlib
import json
import math
import os
import re
import ssl
import stat
import subprocess
import sys
import time
import uuid
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path, PurePosixPath
from typing import Any, Callable
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urlencode, urljoin, urlsplit
from urllib.request import HTTPSHandler, HTTPRedirectHandler, ProxyHandler, Request, build_opener

try:
    fcntl = importlib.import_module("fcntl")
except ImportError:  # pragma: no cover - non-Linux platform gate
    fcntl = None

try:
    import yaml
except ImportError as error:  # pragma: no cover - deployment prerequisite
    raise SystemExit("V-000 harness requires PyYAML") from error


ROOT = Path(__file__).resolve().parents[1]
MANIFEST_VERSION = 2
RESPONSE_BODY_LIMIT = 8 * 1024 * 1024
REQUEST_TIMEOUT_SECONDS = 60.0
INVOCATION_DEADLINE_SECONDS = 300.0
MAX_PAGES = 1_000
MAX_ITEMS = 100_000
JSON_MAX_DEPTH = 64
ALLOWED_DOTENV_KEYS = frozenset(
    {
        "CODEMIE_URL",
        "CODEMIE_AUTH_URL",
        "CODEMIE_TOKEN",
        "CODEMIE_CLIENT_ID",
        "CODEMIE_CLIENT_SECRET",
        "CODEMIE_EMAIL",
        "CODEMIE_PASSWORD",
        "CODEMIE_TEST_PROJECT",
    }
)
SAFE_ID = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
DOTENV_NAME = re.compile(r"^CODEMIE_[A-Z0-9_]+$")
KINDS = ("Assistant", "Workflow", "Skill")
WORKFLOW_IDENTITY_KEY = "codemie.epam.com/gitops/workflow-identity"
FORBIDDEN_MANIFEST_KEYS = frozenset(
    {
        "datasource",
        "datasources",
        "selector",
        "selectors",
        "kindselector",
        "authorizationexception",
        "authorizationexceptions",
        "exception",
        "exceptions",
    }
)


class QualificationError(ValueError):
    """A fixed, safe qualification failure category."""

    def __init__(self, category: str):
        self.category = category
        super().__init__(category)


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req: Request, fp: Any, code: int, msg: str, headers: Any, newurl: str) -> None:
        return None


@dataclass(frozen=True)
class Budget:
    body_bytes: int = RESPONSE_BODY_LIMIT
    request_seconds: float = REQUEST_TIMEOUT_SECONDS
    invocation_seconds: float = INVOCATION_DEADLINE_SECONDS
    pages: int = MAX_PAGES
    items: int = MAX_ITEMS


def _strict_json(content: bytes | str, category: str = "json-incompatible") -> Any:
    def pairs(values: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in values:
            if key in result:
                raise QualificationError(category)
            result[key] = value
        return result

    def constant(_: str) -> None:
        raise QualificationError(category)

    try:
        value = json.loads(content, object_pairs_hook=pairs, parse_constant=constant)
    except (json.JSONDecodeError, UnicodeDecodeError, QualificationError) as error:
        raise QualificationError(category) from error

    def depth(item: Any, level: int = 0) -> None:
        if level > JSON_MAX_DEPTH:
            raise QualificationError(category)
        if isinstance(item, dict):
            for nested in item.values():
                depth(nested, level + 1)
        elif isinstance(item, list):
            for nested in item:
                depth(nested, level + 1)

    depth(value)
    return value


def _exact_keys(value: Any, expected: set[str], category: str = "manifest-invalid") -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        raise QualificationError(category)
    return value


def _string(value: Any, category: str = "response-incompatible") -> str:
    if not isinstance(value, str) or not value:
        raise QualificationError(category)
    return value


def _bool(value: Any, category: str = "response-incompatible") -> bool:
    if type(value) is not bool:
        raise QualificationError(category)
    return value


def _uint(value: Any, category: str = "response-incompatible") -> int:
    if type(value) is not int or value < 0:
        raise QualificationError(category)
    return value


def _strings(value: Any) -> list[str]:
    if not isinstance(value, list):
        raise QualificationError("response-incompatible")
    return [_string(item) for item in value]


def _reject_forbidden_manifest_surface(value: Any, path: tuple[str, ...] = ()) -> None:
    if isinstance(value, dict):
        for key, nested in value.items():
            if not isinstance(key, str) or key.replace("_", "").lower() in FORBIDDEN_MANIFEST_KEYS:
                raise QualificationError("smoke-scope-invalid")
            _reject_forbidden_manifest_surface(nested, path + (key,))
    elif isinstance(value, list):
        for nested in value:
            _reject_forbidden_manifest_surface(nested, path)
    elif isinstance(value, str) and value.casefold() == "datasource":
        raise QualificationError("smoke-scope-invalid")


class StrictYamlLoader(yaml.SafeLoader):
    pass


def _yaml_mapping(loader: StrictYamlLoader, node: yaml.MappingNode, deep: bool = False) -> dict[Any, Any]:
    result: dict[Any, Any] = {}
    for key, value in loader.construct_pairs(node, deep=deep):
        if key in result:
            raise QualificationError("declaration-invalid")
        result[key] = value
    return result


StrictYamlLoader.add_constructor(yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, _yaml_mapping)


def _load_declaration_yaml(content: bytes) -> dict[str, Any]:
    try:
        documents = list(yaml.load_all(content.decode("utf-8"), Loader=StrictYamlLoader))
    except (UnicodeDecodeError, yaml.YAMLError, QualificationError) as error:
        raise QualificationError("declaration-invalid") from error
    if len(documents) != 1 or not isinstance(documents[0], dict):
        raise QualificationError("declaration-invalid")
    return documents[0]


def _safe_workspace_file(workspace: Path, relative: str) -> tuple[Path, bytes]:
    pure = PurePosixPath(relative)
    if pure.is_absolute() or ".." in pure.parts or not pure.parts:
        raise QualificationError("declaration-path-invalid")
    try:
        workspace_metadata = workspace.lstat()
        if stat.S_ISLNK(workspace_metadata.st_mode) or not stat.S_ISDIR(workspace_metadata.st_mode):
            raise QualificationError("declaration-path-invalid")
        root = workspace.resolve(strict=True)
        current = root
        for part in pure.parts:
            current = current / part
            metadata = current.lstat()
            if stat.S_ISLNK(metadata.st_mode):
                raise QualificationError("declaration-path-invalid")
        resolved = current.resolve(strict=True)
        resolved.relative_to(root)
        metadata = resolved.stat()
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > 1024 * 1024:
            raise QualificationError("declaration-path-invalid")
        descriptor = os.open(resolved, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
        try:
            opened = os.fstat(descriptor)
            if (opened.st_dev, opened.st_ino) != (metadata.st_dev, metadata.st_ino):
                raise QualificationError("declaration-path-invalid")
            chunks: list[bytes] = []
            total = 0
            while chunk := os.read(descriptor, 65536):
                total += len(chunk)
                if total > 1024 * 1024:
                    raise QualificationError("declaration-path-invalid")
                chunks.append(chunk)
        finally:
            os.close(descriptor)
    except (OSError, ValueError, QualificationError) as error:
        raise QualificationError("declaration-path-invalid") from error
    return resolved, b"".join(chunks)


def validate_concrete_declarations(
    manifest: dict[str, Any],
    *,
    workspace: Path,
    binary: "StagedBinary",
    deadline: float,
) -> str:
    lint_environment = {"PATH": os.environ.get("PATH", "")}
    config_project: str | None = None
    workflow_display_name: str | None = None
    config_path = workspace / ".codemie/config.yaml"
    if os.path.lexists(config_path):
        _, config_content = _safe_workspace_file(workspace, ".codemie/config.yaml")
        config = _load_declaration_yaml(config_content)
        candidate = config.get("project")
        if candidate is not None and not isinstance(candidate, str):
            raise QualificationError("declaration-invalid")
        config_project = candidate
    for item in manifest["declarations"]:
        declaration_path, content = _safe_workspace_file(workspace, item["path"])
        declaration = _load_declaration_yaml(content)
        kind = item["kind"]
        if declaration.get("apiVersion") != "codemie.epam.com/v1alpha1" or declaration.get("kind") != kind:
            raise QualificationError("declaration-binding-invalid")
        metadata = declaration.get("metadata")
        effective_project = metadata.get("project", config_project) if isinstance(metadata, dict) else None
        if not isinstance(metadata, dict) or effective_project != item["naturalKey"]["project"]:
            raise QualificationError("project-binding-invalid")
        key_name = "name" if kind == "Skill" else "slug"
        if metadata.get(key_name) != item["naturalKey"][key_name]:
            raise QualificationError("declaration-binding-invalid")
        if kind == "Workflow":
            spec = declaration.get("spec")
            if not isinstance(spec, dict):
                raise QualificationError("declaration-invalid")
            workflow_display_name = _string(spec.get("name"), "declaration-invalid")
        try:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise QualificationError("deadline-exceeded")
            result = binary.run(
                ["lint", "--file", str(declaration_path), "--repo-root", str(workspace), "--output", "json"],
                cwd=workspace,
                env=lint_environment,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=remaining,
                check=False,
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            raise QualificationError("declaration-lint-failed") from error
        if result.returncode != 0:
            raise QualificationError("declaration-lint-failed")
    if workflow_display_name is None:
        raise QualificationError("declaration-invalid")
    return workflow_display_name


def _timestamp(value: Any) -> datetime:
    text = _string(value, "manifest-invalid")
    if not text.endswith("Z"):
        raise QualificationError("manifest-invalid")
    try:
        parsed = datetime.fromisoformat(text[:-1] + "+00:00")
    except ValueError as error:
        raise QualificationError("manifest-invalid") from error
    if parsed.tzinfo is None:
        raise QualificationError("manifest-invalid")
    return parsed.astimezone(timezone.utc)


def load_smoke_manifest(path: Path) -> dict[str, Any]:
    try:
        if path.is_symlink() or path.stat().st_size > 1024 * 1024:
            raise QualificationError("manifest-invalid")
        content = path.read_bytes()
    except (OSError, QualificationError) as error:
        raise QualificationError("manifest-invalid") from error
    document = _strict_json(content, "manifest-invalid")
    return validate_smoke_manifest(document)


def validate_smoke_manifest(document: Any, *, now: datetime | None = None) -> dict[str, Any]:
    _reject_forbidden_manifest_surface(document)
    root = _exact_keys(document, {"schemaVersion", "runId", "target", "authorization", "declarations"})
    if root["schemaVersion"] != 1:
        raise QualificationError("manifest-invalid")
    run_id = _string(root["runId"], "manifest-invalid")
    if SAFE_ID.fullmatch(run_id) is None:
        raise QualificationError("manifest-invalid")
    validate_target_url(_string(root["target"], "manifest-invalid"))
    authorization = _exact_keys(
        root["authorization"],
        {"project", "actor", "allowedKinds", "durableRecordOwner", "exclusiveWriter"},
    )
    project = _string(authorization["project"], "manifest-invalid")
    _string(authorization["actor"], "manifest-invalid")
    _string(authorization["durableRecordOwner"], "manifest-invalid")
    if authorization["allowedKinds"] != list(KINDS):
        raise QualificationError("smoke-scope-invalid")
    writer = _exact_keys(
        authorization["exclusiveWriter"],
        {"confirmed", "confirmer", "startsAt", "endsAt", "sequenceEndsAt", "runPrefix", "noOtherWriterAttested"},
    )
    if not _bool(writer["confirmed"], "manifest-invalid") or not _bool(writer["noOtherWriterAttested"], "manifest-invalid"):
        raise QualificationError("writer-window-invalid")
    _string(writer["confirmer"], "manifest-invalid")
    prefix = _string(writer["runPrefix"], "manifest-invalid")
    if SAFE_ID.fullmatch(prefix) is None or not run_id.startswith(prefix):
        raise QualificationError("writer-window-invalid")
    starts = _timestamp(writer["startsAt"])
    ends = _timestamp(writer["endsAt"])
    sequence_ends = _timestamp(writer["sequenceEndsAt"])
    current = (now or datetime.now(timezone.utc)).astimezone(timezone.utc)
    if not starts <= current < sequence_ends <= ends:
        raise QualificationError("writer-window-invalid")
    declarations = root["declarations"]
    if not isinstance(declarations, list) or len(declarations) != 3:
        raise QualificationError("smoke-scope-invalid")
    found: set[str] = set()
    for declaration in declarations:
        item = _exact_keys(declaration, {"kind", "path", "naturalKey"})
        kind = item["kind"]
        if kind not in KINDS or kind in found:
            raise QualificationError("smoke-scope-invalid")
        found.add(kind)
        relative = PurePosixPath(_string(item["path"], "manifest-invalid"))
        if relative.is_absolute() or ".." in relative.parts or relative.suffix not in {".yaml", ".yml"}:
            raise QualificationError("manifest-invalid")
        if any("datasource" in part.casefold() for part in relative.parts):
            raise QualificationError("smoke-scope-invalid")
        key_name = "name" if kind == "Skill" else "slug"
        natural = _exact_keys(item["naturalKey"], {"project", key_name})
        if _string(natural["project"], "manifest-invalid") != project:
            raise QualificationError("project-binding-invalid")
        natural_value = _string(natural[key_name], "manifest-invalid")
        if not natural_value.startswith(prefix):
            raise QualificationError("writer-window-invalid")
    if found != set(KINDS):
        raise QualificationError("smoke-scope-invalid")
    return root


def validate_target_url(value: str, *, allow_loopback_http: bool = False) -> tuple[str, str, int]:
    if any(ord(char) < 32 or 127 <= ord(char) <= 159 for char in value):
        raise QualificationError("target-invalid")
    try:
        parsed = urlsplit(value)
        port = parsed.port
    except ValueError as error:
        raise QualificationError("target-invalid") from error
    if parsed.username is not None or parsed.password is not None or parsed.fragment or parsed.query:
        raise QualificationError("target-invalid")
    if not parsed.hostname or parsed.path not in ("", "/"):
        raise QualificationError("target-invalid")
    if parsed.scheme != "https":
        if not allow_loopback_http or parsed.scheme != "http" or parsed.hostname not in {"127.0.0.1", "::1", "localhost"}:
            raise QualificationError("target-invalid")
    return parsed.scheme, parsed.hostname.casefold(), port or (443 if parsed.scheme == "https" else 80)


def _open_binary_without_symlinks(path: Path) -> int:
    """Open an authored path component-by-component without PATH lookup."""

    parts = path.parts
    if not parts or ".." in parts:
        raise QualificationError("binary-invalid")
    nofollow = getattr(os, "O_NOFOLLOW", 0)
    if not nofollow or not Path("/proc/self/fd").is_dir():
        raise QualificationError("binary-platform-unsupported")
    directory_flags = os.O_RDONLY | os.O_DIRECTORY | nofollow
    try:
        descriptor = os.open("/" if path.is_absolute() else ".", directory_flags)
    except OSError as error:
        raise QualificationError("binary-invalid") from error
    components = parts[1:] if path.is_absolute() else parts
    try:
        for component in components[:-1]:
            next_descriptor = os.open(component, directory_flags, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = next_descriptor
        opened = os.open(components[-1], os.O_RDONLY | nofollow, dir_fd=descriptor)
    except (OSError, IndexError) as error:
        raise QualificationError("binary-invalid") from error
    finally:
        os.close(descriptor)
    return opened


class StagedBinary:
    """An immutable sealed copy of the one opened and hashed executable."""

    __slots__ = ("_descriptor", "_digest", "_identity")

    def __init__(self, descriptor: int, digest: str) -> None:
        self._descriptor = descriptor
        self._digest = digest
        self._identity = object()

    @classmethod
    def open(cls, path: Path, expected: str, *, deadline: float | None = None) -> "StagedBinary":
        if SHA256.fullmatch(expected) is None:
            raise QualificationError("checksum-invalid")
        required_os = ("memfd_create", "MFD_CLOEXEC", "MFD_ALLOW_SEALING")
        required_fcntl = ("F_ADD_SEALS", "F_GET_SEALS", "F_SEAL_WRITE", "F_SEAL_GROW", "F_SEAL_SHRINK", "F_SEAL_SEAL")
        if (
            any(not hasattr(os, name) for name in required_os)
            or fcntl is None
            or any(not hasattr(fcntl, name) for name in required_fcntl)
        ):
            raise QualificationError("binary-platform-unsupported")
        source = _open_binary_without_symlinks(path)
        descriptor = -1
        try:
            metadata = os.fstat(source)
            if (
                not stat.S_ISREG(metadata.st_mode)
                or metadata.st_mode & 0o222
                or not metadata.st_mode & 0o111
            ):
                raise QualificationError("binary-invalid")
            descriptor = os.memfd_create(
                "codemie-gitops-v000",
                os.MFD_CLOEXEC | os.MFD_ALLOW_SEALING,
            )
            digest = hashlib.sha256()
            while chunk := os.read(source, 1024 * 1024):
                if deadline is not None and time.monotonic() >= deadline:
                    raise QualificationError("deadline-exceeded")
                digest.update(chunk)
                written = 0
                while written < len(chunk):
                    count = os.write(descriptor, chunk[written:])
                    if count <= 0:
                        raise OSError("short staged-binary copy")
                    written += count
            actual = digest.hexdigest()
            if not hmac.compare_digest(actual, expected):
                raise QualificationError("checksum-mismatch")
            os.fchmod(descriptor, 0o555)
            seals = (
                fcntl.F_SEAL_WRITE
                | fcntl.F_SEAL_GROW
                | fcntl.F_SEAL_SHRINK
                | fcntl.F_SEAL_SEAL
            )
            fcntl.fcntl(descriptor, fcntl.F_ADD_SEALS, seals)
            if fcntl.fcntl(descriptor, fcntl.F_GET_SEALS) != seals:
                raise QualificationError("binary-invalid")
            os.lseek(descriptor, 0, os.SEEK_SET)
            return cls(descriptor, actual)
        except QualificationError:
            if descriptor >= 0:
                os.close(descriptor)
            raise
        except OSError as error:
            if descriptor >= 0:
                os.close(descriptor)
            raise QualificationError("binary-invalid") from error
        finally:
            os.close(source)

    @property
    def digest(self) -> str:
        return self._digest

    @property
    def closed(self) -> bool:
        return self._descriptor < 0

    def run(self, arguments: list[str], **options: Any) -> subprocess.CompletedProcess[Any]:
        if self.closed:
            raise QualificationError("binary-invalid")
        executable = f"/proc/self/fd/{self._descriptor}"
        return subprocess.run(
            [executable, *arguments],
            pass_fds=(self._descriptor,),
            **options,
        )

    def close(self) -> None:
        if not self.closed:
            os.close(self._descriptor)
            self._descriptor = -1

    def __enter__(self) -> "StagedBinary":
        return self

    def __exit__(self, *_: object) -> None:
        self.close()


def verify_staged_binary(path: Path, expected: str, *, deadline: float | None = None) -> str:
    """Compatibility helper for isolated verification; execution uses the capability."""

    with StagedBinary.open(path, expected, deadline=deadline) as binary:
        return binary.digest


def parse_dotenv_data(content: str) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in content.splitlines():
        if not line or line.startswith("#"):
            continue
        if line.startswith("export ") or "\x00" in line or "\r" in line or "=" not in line:
            raise QualificationError("dotenv-invalid")
        name, value = line.split("=", 1)
        if DOTENV_NAME.fullmatch(name) is None or name not in ALLOWED_DOTENV_KEYS or name in result:
            raise QualificationError("dotenv-invalid")
        if not value or value != value.strip() or "\n" in value:
            raise QualificationError("dotenv-invalid")
        if value[:1] in {"'", '"'}:
            if len(value) < 2 or value[-1] != value[0] or value[0] in value[1:-1]:
                raise QualificationError("dotenv-invalid")
            value = value[1:-1]
        if not value or any(ord(char) < 32 or ord(char) == 127 for char in value):
            raise QualificationError("dotenv-invalid")
        result[name] = value
    return result


def load_workspace_dotenv(workspace: Path, supplied: Path, *, deadline: float | None = None) -> dict[str, str]:
    expected = workspace / ".env"
    if supplied != expected:
        raise QualificationError("dotenv-invalid")
    try:
        metadata = os.lstat(supplied)
        if not stat.S_ISREG(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode):
            raise QualificationError("dotenv-invalid")
        if metadata.st_uid != os.getuid() or stat.S_IMODE(metadata.st_mode) != 0o600:
            raise QualificationError("dotenv-invalid")
        descriptor = os.open(supplied, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
        try:
            opened = os.fstat(descriptor)
            if (opened.st_dev, opened.st_ino) != (metadata.st_dev, metadata.st_ino):
                raise QualificationError("dotenv-invalid")
            chunks: list[bytes] = []
            total = 0
            while chunk := os.read(descriptor, 65536):
                total += len(chunk)
                if total > 1024 * 1024:
                    raise QualificationError("dotenv-invalid")
                chunks.append(chunk)
        finally:
            os.close(descriptor)
        def git(arguments: list[str], *, stdout: Any) -> subprocess.CompletedProcess[bytes]:
            timeout = (deadline - time.monotonic()) if deadline is not None else INVOCATION_DEADLINE_SECONDS
            if timeout <= 0:
                raise QualificationError("deadline-exceeded")
            return subprocess.run(
                ["git", *arguments], cwd=workspace, stdout=stdout,
                stderr=subprocess.DEVNULL, check=False, timeout=timeout,
            )

        checks = (
            git(["check-ignore", "--quiet", "--", ".env"], stdout=subprocess.DEVNULL).returncode == 0,
            git(["ls-files", "--error-unmatch", "--", ".env"], stdout=subprocess.DEVNULL).returncode != 0,
            git(["log", "--all", "--format=%H", "--", ".env"], stdout=subprocess.PIPE).stdout == b"",
        )
        if not all(checks):
            raise QualificationError("dotenv-invalid")
        content = b"".join(chunks).decode("utf-8")
    except QualificationError:
        raise
    except (OSError, UnicodeDecodeError, subprocess.TimeoutExpired) as error:
        raise QualificationError("dotenv-invalid") from error
    return parse_dotenv_data(content)


def obtain_token(binary: StagedBinary, target: str, environment: dict[str, str], deadline_seconds: float) -> str:
    supplied = environment.get("CODEMIE_TOKEN")
    if supplied:
        return supplied
    clean = dict(environment)
    try:
        result = binary.run(
            ["login", "--url", target],
            env=clean,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=deadline_seconds,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise QualificationError("authentication-failed") from error
    if result.returncode != 0 or result.stderr or len(result.stdout) > 16 * 1024:
        raise QualificationError("authentication-failed")
    try:
        token = result.stdout.decode("utf-8")
    except UnicodeDecodeError as error:
        raise QualificationError("authentication-failed") from error
    if not token.endswith("\n") or "\n" in token[:-1] or "\r" in token:
        raise QualificationError("authentication-failed")
    token = token[:-1]
    if not token or any(ord(char) < 33 or ord(char) == 127 for char in token):
        raise QualificationError("authentication-failed")
    return token


class ReadOnlyTransport:
    """Closed GET-only transport. It has no method-bearing public primitive."""

    def __init__(
        self,
        target: str,
        token: str | ExecutionSession,
        *,
        budget: Budget = Budget(),
        allow_loopback_http: bool = False,
        opener: Any | None = None,
    ) -> None:
        self._origin = validate_target_url(target, allow_loopback_http=allow_loopback_http)
        self._base = target.rstrip("/") + "/"
        if isinstance(token, ExecutionSession):
            self._token = token._token
            self._session_identity: object | None = token._identity
        else:
            self._token = token
            self._session_identity = None
        self._budget = budget
        self._deadline = time.monotonic() + budget.invocation_seconds
        context = ssl.create_default_context()
        self._opener = opener or build_opener(ProxyHandler({}), HTTPSHandler(context=context), NoRedirect())
        self.requests: list[dict[str, str]] = []

    def _remaining(self) -> float:
        remaining = self._deadline - time.monotonic()
        if remaining <= 0:
            raise QualificationError("deadline-exceeded")
        return min(self._budget.request_seconds, remaining)

    def get_json(self, route: str, *, query: dict[str, str] | None = None, probe: str, allow_not_found: bool = False) -> Any:
        if not route.startswith("/") or "//" in route or "?" in route or "#" in route:
            raise QualificationError("route-invalid")
        url = urljoin(self._base, route.lstrip("/"))
        if query:
            url += "?" + urlencode(query)
        parsed = urlsplit(url)
        origin = (parsed.scheme, (parsed.hostname or "").casefold(), parsed.port or (443 if parsed.scheme == "https" else 80))
        if origin != self._origin:
            raise QualificationError("origin-mismatch")
        request = Request(url, headers={"Authorization": f"Bearer {self._token}", "Accept": "application/json"}, method="GET")
        try:
            response = self._opener.open(request, timeout=self._remaining())
        except HTTPError as error:
            if allow_not_found and error.code == 404:
                return None
            if 300 <= error.code < 400:
                raise QualificationError("redirect-rejected") from None
            raise QualificationError("http-failed") from None
        except (URLError, TimeoutError, OSError):
            raise QualificationError("connectivity-failed") from None
        try:
            status = response.getcode()
            if status is None or not 200 <= status < 300:
                raise QualificationError("http-failed")
            # Server-controlled headers are never evidence. A locally generated
            # opaque ID safely correlates this fixed probe category.
            request_id = str(uuid.uuid4())
            body = bytearray()
            while True:
                self._remaining()
                chunk = response.read(min(65536, self._budget.body_bytes + 1 - len(body)))
                if not chunk:
                    break
                body.extend(chunk)
                if len(body) > self._budget.body_bytes:
                    raise QualificationError("body-limit-exceeded")
            decoded = _strict_json(bytes(body))
            self.requests.append({"probe": probe, "requestId": request_id})
            return decoded
        except QualificationError:
            raise
        except (OSError, TimeoutError):
            raise QualificationError("connectivity-failed") from None
        finally:
            response.close()


def decode_user(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise QualificationError("response-incompatible")
    email = _string(value.get("email"))
    is_admin = _bool(value.get("is_admin"))
    is_maintainer = _bool(value.get("is_maintainer"))
    raw_projects = value.get("projects")
    if not isinstance(raw_projects, list):
        raise QualificationError("response-incompatible")
    projects: list[dict[str, Any]] = []
    for raw in raw_projects:
        if not isinstance(raw, dict):
            raise QualificationError("response-incompatible")
        projects.append({"name": _string(raw.get("name")), "is_project_admin": _bool(raw.get("is_project_admin"))})
    return {"email": email, "is_admin": is_admin, "is_maintainer": is_maintainer, "projects": projects}


def _validate_workflow_item(item: Any) -> str:
    if not isinstance(item, dict):
        raise QualificationError("response-incompatible")
    identifier = _string(item.get("id"))
    _string(item.get("project")); _string(item.get("name")); _strings(item.get("user_abilities"))
    if "meta_config" not in item or (item["meta_config"] is not None and not isinstance(item["meta_config"], str)):
        raise QualificationError("response-incompatible")
    return identifier


def _workflow_marker_matches(
    item: Any,
    *,
    project: str,
    slug: str,
) -> tuple[str, str]:
    identifier = _validate_workflow_item(item)
    if item["project"] != project:
        return identifier, "other-project"
    meta_config = item["meta_config"]
    if meta_config is None:
        return identifier, "unmarked"
    value = _strict_json(meta_config, "identity-marker-invalid")
    if not isinstance(value, dict):
        raise QualificationError("identity-marker-invalid")
    if WORKFLOW_IDENTITY_KEY not in value:
        return identifier, "unmarked"
    identity = value[WORKFLOW_IDENTITY_KEY]
    if not isinstance(identity, dict) or set(identity) != {"version", "project", "slug"}:
        raise QualificationError("identity-marker-invalid")
    marker_project = identity["project"]
    marker_slug = identity["slug"]
    if (
        type(identity["version"]) is not int
        or identity["version"] != 1
        or not isinstance(marker_project, str)
        or not marker_project
        or not isinstance(marker_slug, str)
        or not marker_slug
        or marker_project != item["project"]
    ):
        raise QualificationError("identity-marker-invalid")
    return identifier, "exact" if marker_project == project and marker_slug == slug else "other-valid"


def _validate_skill_item(item: Any) -> str:
    if not isinstance(item, dict):
        raise QualificationError("response-incompatible")
    identifier = _string(item.get("id"))
    _string(item.get("name")); _string(item.get("project")); _strings(item.get("user_abilities"))
    if "created_by" not in item or (item["created_by"] is not None and not isinstance(item["created_by"], dict)):
        raise QualificationError("response-incompatible")
    return identifier


def _validate_datasource_item(item: Any) -> str:
    if not isinstance(item, dict):
        raise QualificationError("response-incompatible")
    identifier = _string(item.get("id"))
    _string(item.get("repo_name")); _string(item.get("project_name")); _string(item.get("index_type")); _strings(item.get("user_abilities"))
    return identifier


def _scan(
    transport: ReadOnlyTransport,
    *,
    route: str,
    query_for_page: Callable[[int], dict[str, str]],
    list_key: str,
    pagination_style: str,
    probe: str,
    validate_item: Callable[[Any], str],
) -> tuple[int, int, int]:
    expected: tuple[int, int, int] | None = None
    seen: set[str] = set()
    count = 0
    page = 0
    page_zero_items = 0
    while True:
        if page >= transport._budget.pages:
            raise QualificationError("page-limit-exceeded")
        value = transport.get_json(route, query=query_for_page(page), probe=probe)
        if not isinstance(value, dict):
            raise QualificationError("response-incompatible")
        items = value.get(list_key)
        if not isinstance(items, list):
            raise QualificationError("response-incompatible")
        if pagination_style == "nested":
            pagination = value.get("pagination")
            if not isinstance(pagination, dict):
                raise QualificationError("response-incompatible")
            returned = _uint(pagination.get("page")); per_page = _uint(pagination.get("per_page")); total = _uint(pagination.get("total")); pages = _uint(pagination.get("pages"))
        else:
            returned = _uint(value.get("page")); per_page = _uint(value.get("perPage")); total = _uint(value.get("total")); pages = _uint(value.get("pages"))
        if returned != page or per_page != 100 or pages != math.ceil(total / 100) or ((pages == 0) != (total == 0)):
            raise QualificationError("pagination-incompatible")
        current = (pages, total, per_page)
        if expected is None:
            expected = current
            page_zero_items = len(items)
            if pages > transport._budget.pages or total > transport._budget.items:
                raise QualificationError("pagination-limit-exceeded")
        elif current != expected:
            raise QualificationError("pagination-unstable")
        for item in items:
            identifier = validate_item(item)
            if identifier in seen:
                raise QualificationError("pagination-unstable")
            seen.add(identifier)
            count += 1
            if count > transport._budget.items:
                raise QualificationError("item-limit-exceeded")
        if pages == 0 or page + 1 >= pages:
            break
        page += 1
    if expected is None or count != expected[1]:
        raise QualificationError("pagination-unstable")
    return page_zero_items, page + 1, count


def validate_runtime_gate(
    manifest: dict[str, Any],
    *,
    target: str,
    project: str,
    user: dict[str, Any],
    now: datetime | None = None,
) -> dict[str, str]:
    authorization = manifest["authorization"]
    declaration_projects = {item["naturalKey"]["project"] for item in manifest["declarations"]}
    if manifest["target"] != target or authorization["project"] != project or declaration_projects != {project}:
        raise QualificationError("project-binding-invalid")
    if user["email"] != authorization["actor"]:
        raise QualificationError("actor-binding-invalid")
    matching = [entry for entry in user["projects"] if entry["name"] == project]
    if len(matching) != 1:
        raise QualificationError("project-binding-invalid")
    role = user["is_admin"] or user["is_maintainer"] or matching[0]["is_project_admin"]
    if not role:
        raise QualificationError("role-binding-invalid")
    validate_smoke_manifest(manifest, now=now)
    return {"actor": "pass", "project": "pass", "role": "pass", "writerWindow": "pass"}


_CAPABILITY_SEAL = object()


class ExecutionSession:
    """Process-local bearer-session identity; its token is never evidence."""

    __slots__ = ("_identity", "_token")

    def __init__(self, seal: object, token: str) -> None:
        if seal is not _CAPABILITY_SEAL or not token:
            raise QualificationError("session-binding-invalid")
        self._identity = object()
        self._token = token


def begin_execution_session(token: str) -> ExecutionSession:
    return ExecutionSession(_CAPABILITY_SEAL, token)


class QualificationProof:
    """Unpersistable proof bound to one open staged binary and auth session."""

    __slots__ = ("_binary_identity", "_digest", "_seal", "_session_identity")

    def __init__(
        self,
        seal: object,
        binary: StagedBinary,
        session: ExecutionSession,
    ) -> None:
        if seal is not _CAPABILITY_SEAL or binary.closed or SHA256.fullmatch(binary.digest) is None:
            raise QualificationError("qualification-proof-invalid")
        self._seal = seal
        self._binary_identity = binary._identity
        self._session_identity = session._identity
        self._digest = binary.digest


@dataclass(frozen=True)
class QualificationResult:
    evidence: dict[str, Any]
    proof: QualificationProof


def require_qualification_for_apply(
    proof: QualificationProof,
    *,
    binary: StagedBinary,
    session: ExecutionSession,
    expected_digest: str,
) -> None:
    """Seal a future V-003 call to the live in-process qualification inputs."""

    if SHA256.fullmatch(expected_digest) is None:
        raise QualificationError("checksum-invalid")
    if (
        proof._seal is not _CAPABILITY_SEAL
        or binary.closed
        or proof._binary_identity is not binary._identity
        or proof._session_identity is not session._identity
        or not hmac.compare_digest(proof._digest, expected_digest)
        or not hmac.compare_digest(binary.digest, expected_digest)
    ):
        raise QualificationError("qualification-proof-invalid")


def run_probes(
    transport: ReadOnlyTransport,
    manifest: dict[str, Any],
    project: str,
    binary: StagedBinary,
    session: ExecutionSession,
    workflow_display_name: str,
) -> QualificationResult:
    if transport._session_identity is not session._identity:
        raise QualificationError("session-binding-invalid")
    user = decode_user(transport.get_json("/v1/user", probe="capability"))
    workflow_declaration = next(item for item in manifest["declarations"] if item["kind"] == "Workflow")
    workflow_slug = workflow_declaration["naturalKey"]["slug"]
    workflow_exact_ids: set[str] = set()
    workflow_display_collision_ids: set[str] = set()

    def validate_workflow(item: Any) -> str:
        identifier, classification = _workflow_marker_matches(item, project=project, slug=workflow_slug)
        if classification == "exact":
            workflow_exact_ids.add(identifier)
        elif classification == "unmarked" and item["name"] == workflow_display_name:
            workflow_display_collision_ids.add(identifier)
        return identifier

    workflow_observations = []
    for scope in (None, "marketplace"):
        workflow_observations.append(
            _scan(
                transport,
                route="/v1/workflows",
                query_for_page=lambda page, scope=scope: {"minimal_response": "false", "page": str(page), "per_page": "100", **({"scope": scope} if scope else {})},
                list_key="data", pagination_style="nested", probe="workflow-identity-page-0" if scope is None else "workflow-marketplace-identity-page-0", validate_item=validate_workflow,
            )
        )
    skill_declaration = next(item for item in manifest["declarations"] if item["kind"] == "Skill")
    skill_name = skill_declaration["naturalKey"]["name"]
    general_skill_filter = json.dumps(
        {"project": project, "scope": "project_with_marketplace"},
        separators=(",", ":"),
        sort_keys=True,
    )
    skill_compatibility = _scan(
        transport,
        route="/v1/skills",
        query_for_page=lambda page: {"filters": general_skill_filter, "page": str(page), "per_page": "100"},
        list_key="skills",
        pagination_style="flat",
        probe="skill-compatibility-page-0",
        validate_item=_validate_skill_item,
    )
    exact_skill_filter = json.dumps(
        {"project": project, "scope": "project_with_marketplace", "search": skill_name},
        separators=(",", ":"),
        sort_keys=True,
    )
    skill_exact_ids: set[str] = set()

    def validate_exact_skill(item: Any) -> str:
        identifier = _validate_skill_item(item)
        if item["project"] == project and item["name"] == skill_name:
            skill_exact_ids.add(identifier)
        return identifier

    _scan(
        transport,
        route="/v1/skills",
        query_for_page=lambda page: {"filters": exact_skill_filter, "page": str(page), "per_page": "100"},
        list_key="skills",
        pagination_style="flat",
        probe="skill-identity-page-0",
        validate_item=validate_exact_skill,
    )
    datasource_filter = json.dumps({"project": project}, separators=(",", ":"), sort_keys=True)
    datasource = _scan(transport, route="/v1/index", query_for_page=lambda page: {"full_response": "true", "page": str(page), "per_page": "100", "filters": datasource_filter}, list_key="data", pagination_style="nested", probe="datasource-read-page-0", validate_item=_validate_datasource_item)
    assistant = next(item for item in manifest["declarations"] if item["kind"] == "Assistant")
    slug = assistant["naturalKey"]["slug"]
    existing = transport.get_json(f"/v1/assistants/slug/{quote(slug, safe='')}", query={"project": project}, probe="assistant-identity-read", allow_not_found=True)
    if existing is not None:
        if not isinstance(existing, dict):
            raise QualificationError("response-incompatible")
        _string(existing.get("id")); _strings(existing.get("user_abilities"))
    if sum(item[0] for item in workflow_observations) == 0 or skill_compatibility[0] == 0:
        raise QualificationError("page-zero-observation-missing")
    if existing is not None or workflow_exact_ids or workflow_display_collision_ids or skill_exact_ids:
        raise QualificationError("identity-collision")
    bindings = validate_runtime_gate(
        manifest,
        target=transport._base.rstrip("/") + "/",
        project=project,
        user=user,
    )
    evidence = {
        "schemaVersion": 1,
        "manifestVersion": MANIFEST_VERSION,
        "status": "pass",
        "stagedBinarySha256": binary.digest,
        "bindings": bindings,
        "naturalKeyAbsence": {kind: "pass" for kind in KINDS},
        "observations": {
            "workflowPage0Items": sum(item[0] for item in workflow_observations),
            "skillPage0Items": skill_compatibility[0],
            "datasourcePage0Items": datasource[0],
        },
        "requests": transport.requests,
    }
    return QualificationResult(evidence, QualificationProof(_CAPABILITY_SEAL, binary, session))


def write_evidence(path: Path, evidence: dict[str, Any]) -> None:
    data = (json.dumps(evidence, separators=(",", ":"), sort_keys=True) + "\n").encode("utf-8")
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0), 0o600)
        try:
            written = 0
            while written < len(data):
                count = os.write(descriptor, data[written:])
                if count <= 0:
                    raise OSError("short evidence write")
                written += count
            os.fsync(descriptor)
        finally:
            os.close(descriptor)
    except OSError as error:
        raise QualificationError("evidence-write-failed") from error


def execute(
    *,
    binary: Path,
    expected_sha256: str,
    target: str,
    project: str,
    manifest_path: Path,
    evidence_path: Path,
    environment: dict[str, str],
    dotenv: Path | None = None,
    credential_workspace: Path = ROOT,
    declaration_root: Path | None = None,
    transport_factory: Callable[[str, ExecutionSession], ReadOnlyTransport] = ReadOnlyTransport,
) -> dict[str, Any]:
    deadline = time.monotonic() + INVOCATION_DEADLINE_SECONDS
    declarations = declaration_root or credential_workspace
    manifest = load_smoke_manifest(manifest_path)
    validate_target_url(target)
    if manifest["target"] != target or manifest["authorization"]["project"] != project:
        raise QualificationError("project-binding-invalid")
    with StagedBinary.open(binary, expected_sha256, deadline=deadline) as staged:
        workflow_display_name = validate_concrete_declarations(
            manifest,
            workspace=declarations,
            binary=staged,
            deadline=deadline,
        )
        values = dict(environment)
        if dotenv is not None:
            values.update(load_workspace_dotenv(credential_workspace, dotenv, deadline=deadline))
        configured_target = values.get("CODEMIE_URL")
        if configured_target is not None and configured_target != target:
            raise QualificationError("project-binding-invalid")
        values["CODEMIE_URL"] = target
        if values.get("CODEMIE_TEST_PROJECT") != project:
            raise QualificationError("project-binding-invalid")
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise QualificationError("deadline-exceeded")
        token = obtain_token(staged, target, values, remaining)
        values.pop("CODEMIE_CLIENT_SECRET", None)
        values.pop("CODEMIE_PASSWORD", None)
        session = begin_execution_session(token)
        try:
            transport = transport_factory(target, session)
            transport._deadline = deadline
            qualification = run_probes(
                transport,
                manifest,
                project,
                staged,
                session,
                workflow_display_name,
            )
            write_evidence(evidence_path, qualification.evidence)
            return qualification.evidence
        finally:
            token = ""
            values.pop("CODEMIE_TOKEN", None)


def validate_declaration_root(candidate: Path, allowed_parent: Path) -> Path:
    absolute = candidate if candidate.is_absolute() else Path.cwd() / candidate
    try:
        parent = allowed_parent.resolve(strict=True)
        relative = absolute.relative_to(allowed_parent)
        if len(relative.parts) != 1 or not relative.name.startswith("run-") or not relative.name.endswith(".local"):
            raise QualificationError("declaration-path-invalid")
        metadata = absolute.lstat()
        if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISDIR(metadata.st_mode):
            raise QualificationError("declaration-path-invalid")
        resolved = absolute.resolve(strict=True)
        resolved.relative_to(parent)
        if resolved.parent != parent:
            raise QualificationError("declaration-path-invalid")
    except (OSError, ValueError, QualificationError) as error:
        raise QualificationError("declaration-path-invalid") from error
    return resolved


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--preflight", action="store_true")
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--url", required=True)
    parser.add_argument("--project", required=True)
    parser.add_argument("--smoke-manifest", type=Path, required=True)
    parser.add_argument("--declaration-root", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--dotenv", type=Path)
    args = parser.parse_args()
    if not args.preflight:
        parser.error("--preflight is required; no modifying mode exists")
    evidence_path: Path | None = None
    try:
        evidence_candidate = args.evidence if args.evidence.is_absolute() else Path.cwd() / args.evidence
        evidence_path = evidence_candidate.parent.resolve(strict=True) / evidence_candidate.name
        relative = evidence_path.relative_to(ROOT / "ops/v000")
        if len(relative.parts) != 1 or not relative.name.endswith(".local.json"):
            raise QualificationError("evidence-path-invalid")
        declaration_root = validate_declaration_root(args.declaration_root, ROOT / "ops/v000")
        dotenv_path = None
        if args.dotenv is not None:
            if args.dotenv not in (Path(".env"), ROOT / ".env"):
                raise QualificationError("dotenv-invalid")
            dotenv_path = ROOT / ".env"
        execute(
            binary=args.binary,
            expected_sha256=args.sha256,
            target=args.url,
            project=args.project,
            manifest_path=args.smoke_manifest,
            evidence_path=evidence_path,
            environment=dict(os.environ),
            dotenv=dotenv_path,
            credential_workspace=ROOT,
            declaration_root=declaration_root,
        )
    except QualificationError as error:
        if evidence_path is not None and not evidence_path.exists():
            failure = {
                "schemaVersion": 1,
                "manifestVersion": MANIFEST_VERSION,
                "status": "fail",
                "category": error.category,
            }
            try:
                write_evidence(evidence_path, failure)
            except QualificationError:
                pass
        print(f"V-000 qualification failed: {error.category}", file=sys.stderr)
        return 2
    print("V-000 qualification: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
