use serde_json::Value;

fn contract() -> Value {
    serde_json::from_str(include_str!("../specs/codemie-openapi.json"))
        .expect("approved OpenAPI baseline must remain valid JSON")
}

#[test]
fn every_openapi_backed_production_operation_exists() {
    let contract = contract();
    let operations = [
        ("get", "/v1/user"),
        ("get", "/v1/assistants/slug/{assistant_slug}"),
        ("post", "/v1/assistants"),
        ("put", "/v1/assistants/{assistant_id}"),
        ("get", "/v1/workflows"),
        ("get", "/v1/workflows/id/{workflow_id}"),
        ("post", "/v1/workflows"),
        ("put", "/v1/workflows/{workflow_id}"),
        ("get", "/v1/skills"),
        ("get", "/v1/skills/{skill_id}"),
        ("get", "/v1/skills/{skill_id}/companion-files/content"),
        ("post", "/v1/skills"),
        ("put", "/v1/skills/{skill_id}"),
        ("get", "/v1/index"),
        ("post", "/v1/application/{app_name}/index"),
        ("put", "/v1/application/{app_name}/index/{repo_name}"),
        ("post", "/v1/index/knowledge_base/file"),
        ("put", "/v1/index/knowledge_base/file"),
    ];

    for (method, path) in operations {
        assert!(
            contract["paths"][path][method].is_object(),
            "approved OpenAPI baseline is missing {method} {path}"
        );
    }
}

#[test]
fn used_operations_declare_success_responses_and_mutation_bodies() {
    let contract = contract();
    let mutations = [
        ("post", "/v1/assistants"),
        ("put", "/v1/assistants/{assistant_id}"),
        ("post", "/v1/workflows"),
        ("put", "/v1/workflows/{workflow_id}"),
        ("post", "/v1/skills"),
        ("put", "/v1/skills/{skill_id}"),
        ("post", "/v1/application/{app_name}/index"),
        ("put", "/v1/application/{app_name}/index/{repo_name}"),
        ("post", "/v1/index/knowledge_base/file"),
        ("put", "/v1/index/knowledge_base/file"),
    ];

    for (method, path) in mutations {
        let operation = &contract["paths"][path][method];
        assert!(
            operation["requestBody"]["content"].is_object(),
            "{method} {path} must declare a request body"
        );
        let responses = operation["responses"]
            .as_object()
            .expect("operation responses must be an object");
        assert!(
            responses.keys().any(|status| status.starts_with('2')),
            "{method} {path} must declare a successful response"
        );
    }
}

#[test]
fn contract_metadata_matches_checked_in_openapi() {
    let metadata: Value = serde_json::from_str(include_str!(
        "../specs/rust-architecture-remediation/contract-metadata.json"
    ))
    .expect("contract metadata must be valid JSON");
    let contract = contract();

    assert_eq!(metadata["openapi"], contract["openapi"]);
    assert_eq!(metadata["serverVersion"], contract["info"]["version"]);
    assert_eq!(
        metadata["sha256"],
        "559600a70febad5963d03b454677c84605f5dab8053efc9d54ff5b6d1273df48"
    );
}

fn schema_with_components(schema: &Value, contract: &Value) -> Value {
    let mut schema = schema.clone();
    schema
        .as_object_mut()
        .expect("operation schema must be an object")
        .insert("components".to_owned(), contract["components"].clone());
    schema
}

fn assert_fixture(schema: &Value, fixture: &Value, label: &str) {
    let validator = jsonschema::validator_for(schema)
        .unwrap_or_else(|error| panic!("{label} schema must compile: {error}"));
    if let Err(error) = validator.validate(fixture) {
        panic!("{label} fixture violates approved OpenAPI schema: {error}");
    }
}

#[test]
fn entity_json_request_and_success_response_fixtures_match_openapi() {
    let contract = contract();
    let long_content = "x".repeat(120);
    let skill_response = serde_json::json!({
        "id": "skill-id",
        "name": "demo-skill",
        "description": "A useful demo skill",
        "content": long_content,
        "project": "demo",
        "visibility": "private",
        "categories": [],
        "createdDate": "2026-08-13T00:00:00Z"
    });
    let cases = [
        (
            "post",
            "/v1/assistants",
            "200",
            serde_json::json!({"name": "Assistant"}),
            serde_json::json!({"message": "created", "assistantId": "assistant-id"}),
        ),
        (
            "put",
            "/v1/assistants/{assistant_id}",
            "200",
            serde_json::json!({"name": "Assistant"}),
            serde_json::json!({"message": "updated"}),
        ),
        (
            "post",
            "/v1/workflows",
            "200",
            serde_json::json!({"name": "Workflow", "description": "", "project": "demo"}),
            serde_json::json!({"message": "created", "data": {"id": "workflow-id"}}),
        ),
        (
            "put",
            "/v1/workflows/{workflow_id}",
            "200",
            serde_json::json!({"name": "Workflow", "description": "", "project": "demo"}),
            serde_json::json!({"message": "updated", "data": {"id": "workflow-id"}}),
        ),
        (
            "post",
            "/v1/skills",
            "201",
            serde_json::json!({
                "name": "demo-skill",
                "description": "A useful demo skill",
                "content": "x".repeat(120),
                "project": "demo"
            }),
            skill_response.clone(),
        ),
        (
            "put",
            "/v1/skills/{skill_id}",
            "200",
            serde_json::json!({"description": "An updated skill"}),
            skill_response,
        ),
        (
            "post",
            "/v1/application/{app_name}/index",
            "201",
            serde_json::json!({
                "name": "demo-repo",
                "description": "Repository",
                "link": "https://github.com/example/repository",
                "branch": "main",
                "indexType": "code"
            }),
            serde_json::json!({"message": "created"}),
        ),
        (
            "put",
            "/v1/application/{app_name}/index/{repo_name}",
            "201",
            serde_json::json!({"description": "Updated repository"}),
            serde_json::json!({"message": "updated"}),
        ),
    ];

    for (method, path, status, request_fixture, response_fixture) in cases {
        let operation = &contract["paths"][path][method];
        let request_schema = schema_with_components(
            &operation["requestBody"]["content"]["application/json"]["schema"],
            &contract,
        );
        assert_fixture(
            &request_schema,
            &request_fixture,
            &format!("{method} {path} request"),
        );

        let response_schema = schema_with_components(
            &operation["responses"][status]["content"]["application/json"]["schema"],
            &contract,
        );
        assert_fixture(
            &response_schema,
            &response_fixture,
            &format!("{method} {path} {status} response"),
        );
    }
}
