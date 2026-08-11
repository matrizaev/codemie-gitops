#!/usr/bin/env python3
"""Analyze an ephemeral, normalized identity inventory for O-001.

The input deliberately excludes server IDs and payloads. Output contains only
schema-valid natural keys and aggregate counts. Malformed values are counted,
never echoed.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Any

MAX_INPUT_BYTES = 8 * 1024 * 1024
MAX_RECORDS_PER_KIND = 100_000
SKILL_NAME = re.compile(r"^[a-z0-9][a-z0-9-]{1,62}[a-z0-9]$")
DATASOURCE_NAME = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_-]*$")


class InventoryInputError(ValueError):
    """The normalized inventory envelope is invalid."""


def _contains_forbidden_character(value: str) -> bool:
    return any(
        ord(character) <= 0x1F
        or 0x7F <= ord(character) <= 0x9F
        or 0x202A <= ord(character) <= 0x202E
        or 0x2066 <= ord(character) <= 0x2069
        for character in value
    )


def _safe_identifier(
    value: Any,
    *,
    minimum: int,
    maximum: int,
    pattern: re.Pattern[str] | None = None,
) -> str | None:
    if not isinstance(value, str):
        return None
    if not minimum <= len(value) <= maximum or _contains_forbidden_character(value):
        return None
    if pattern is not None and pattern.fullmatch(value) is None:
        return None
    return value


def _records(snapshot: dict[str, Any], name: str) -> list[Any]:
    records = snapshot.get(name)
    if not isinstance(records, list):
        raise InventoryInputError("inventory kind collections must be arrays")
    if len(records) > MAX_RECORDS_PER_KIND:
        raise InventoryInputError("inventory kind exceeds the record budget")
    return records


def _duplicate_groups(
    keys: list[tuple[str, str]], *, key_name: str
) -> list[dict[str, Any]]:
    return [
        {"project": project, key_name: key, "count": count}
        for (project, key), count in sorted(Counter(keys).items())
        if count > 1
    ]


def analyze_inventory(snapshot: Any) -> dict[str, Any]:
    if not isinstance(snapshot, dict) or set(snapshot) != {
        "workflows",
        "skills",
        "datasources",
    }:
        raise InventoryInputError("inventory must contain exactly three kind arrays")

    workflow_keys: list[tuple[str, str]] = []
    invalid_workflow_markers = 0
    unmarked_workflows = 0
    for record in _records(snapshot, "workflows"):
        if not isinstance(record, dict) or "markerPresent" not in record:
            invalid_workflow_markers += 1
            continue
        if record.get("markerPresent") is False and set(record) == {"markerPresent"}:
            unmarked_workflows += 1
            continue
        if record.get("markerPresent") is not True or set(record) != {
            "markerPresent",
            "marker",
        }:
            invalid_workflow_markers += 1
            continue
        marker = record.get("marker")
        if not isinstance(marker, dict) or set(marker) != {
            "version",
            "project",
            "slug",
        }:
            invalid_workflow_markers += 1
            continue
        project = _safe_identifier(marker.get("project"), minimum=1, maximum=100)
        slug = _safe_identifier(marker.get("slug"), minimum=1, maximum=100)
        version = marker.get("version")
        if (
            type(version) is not int
            or version != 1
            or project is None
            or slug is None
        ):
            invalid_workflow_markers += 1
            continue
        workflow_keys.append((project, slug))

    skill_keys: list[tuple[str, str]] = []
    invalid_skills = 0
    for record in _records(snapshot, "skills"):
        if not isinstance(record, dict) or set(record) != {"project", "name"}:
            invalid_skills += 1
            continue
        project = _safe_identifier(record.get("project"), minimum=1, maximum=100)
        name = _safe_identifier(
            record.get("name"), minimum=3, maximum=64, pattern=SKILL_NAME
        )
        if project is None or name is None:
            invalid_skills += 1
            continue
        skill_keys.append((project, name))

    datasource_keys: list[tuple[str, str]] = []
    invalid_datasources = 0
    for record in _records(snapshot, "datasources"):
        if not isinstance(record, dict) or set(record) != {"project", "repo_name"}:
            invalid_datasources += 1
            continue
        project = _safe_identifier(record.get("project"), minimum=1, maximum=100)
        repo_name = _safe_identifier(
            record.get("repo_name"),
            minimum=4,
            maximum=50,
            pattern=DATASOURCE_NAME,
        )
        if project is None or repo_name is None:
            invalid_datasources += 1
            continue
        datasource_keys.append((project, repo_name))

    workflow_duplicates = _duplicate_groups(workflow_keys, key_name="slug")
    skill_duplicates = _duplicate_groups(skill_keys, key_name="name")
    datasource_duplicates = _duplicate_groups(
        datasource_keys, key_name="repo_name"
    )
    failed = any(
        (
            invalid_workflow_markers,
            invalid_skills,
            invalid_datasources,
            workflow_duplicates,
            skill_duplicates,
            datasource_duplicates,
        )
    )
    return {
        "status": "fail" if failed else "pass",
        "workflow": {
            "validMarkers": len(workflow_keys),
            "unmarkedRecords": unmarked_workflows,
            "invalidMarkers": invalid_workflow_markers,
            "duplicateNaturalKeys": workflow_duplicates,
        },
        "skill": {
            "validRecords": len(skill_keys),
            "invalidRecords": invalid_skills,
            "duplicateNaturalKeys": skill_duplicates,
        },
        "datasource": {
            "validRecords": len(datasource_keys),
            "invalidRecords": invalid_datasources,
            "duplicateNaturalKeys": datasource_duplicates,
        },
    }


def _read_input(path: str) -> Any:
    def reject_duplicate_names(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for name, value in pairs:
            if name in result:
                raise InventoryInputError("inventory input is not valid UTF-8 JSON")
            result[name] = value
        return result

    def reject_nonstandard_constant(_: str) -> None:
        raise InventoryInputError("inventory input is not valid UTF-8 JSON")

    if path == "-":
        raw = sys.stdin.buffer.read(MAX_INPUT_BYTES + 1)
    else:
        with Path(path).open("rb") as handle:
            raw = handle.read(MAX_INPUT_BYTES + 1)
    if len(raw) > MAX_INPUT_BYTES:
        raise InventoryInputError("inventory input exceeds the byte budget")
    try:
        return json.loads(
            raw,
            object_pairs_hook=reject_duplicate_names,
            parse_constant=reject_nonstandard_constant,
        )
    except (UnicodeDecodeError, json.JSONDecodeError, InventoryInputError) as error:
        raise InventoryInputError("inventory input is not valid UTF-8 JSON") from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--input",
        default="-",
        help="normalized JSON inventory path, or - for stdin",
    )
    args = parser.parse_args()
    try:
        report = analyze_inventory(_read_input(args.input))
    except (InventoryInputError, OSError):
        print("O-001 inventory input is invalid", file=sys.stderr)
        return 2
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))
    return 0 if report["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
