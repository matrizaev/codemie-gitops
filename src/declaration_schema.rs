//! Private declaration DTOs generated from the authoritative JSON Schema.
//!
//! Runtime validation uses the unmodified schema before deserialization. The
//! build script removes only conditional keywords unsupported by the generator;
//! consequently these generated DTOs are a structural transport boundary, not
//! the semantic validator.

#![allow(
    dead_code,
    reason = "schema DTOs include fields not consumed by every command"
)]
#![allow(
    clippy::large_enum_variant,
    clippy::enum_variant_names,
    clippy::derivable_impls,
    clippy::possible_missing_else,
    reason = "typify-generated schema DTO layout and validation code are not hand-authored"
)]

include!(concat!(env!("OUT_DIR"), "/declaration_types.rs"));

/// Borrowed, resolved Skill fields exposed to the projection boundary.
///
/// The generated `SkillSpec` is an internally tagged union whose variant
/// fields are intentionally private. This view keeps projection typed without
/// round-tripping the declaration through an open JSON object.
pub(crate) struct ResolvedSkillSpec<'a> {
    pub(crate) categories: &'a [String],
    pub(crate) companion_files: &'a [CompanionFile],
    pub(crate) content: &'a str,
    pub(crate) description: &'a str,
    pub(crate) enabled_builtin_subagents: &'a [serde_json::Value],
    pub(crate) mcp_servers: &'a [McpServer],
    pub(crate) toolkits: &'a [Toolkit],
    pub(crate) visibility: &'a SkillSpecVariant0Visibility,
}

impl SkillSpec {
    /// Return the post-sidecar Skill form accepted by operation projection.
    ///
    /// `contentFrom` is an authoring boundary representation and must have
    /// been resolved by the parser before the declaration reaches apply.
    pub(crate) fn resolved(&self) -> Option<ResolvedSkillSpec<'_>> {
        match self {
            Self::Variant0 {
                categories,
                companion_files,
                content,
                description,
                enabled_builtin_subagents,
                mcp_servers,
                toolkits,
                visibility,
            } => Some(ResolvedSkillSpec {
                categories,
                companion_files,
                content,
                description,
                enabled_builtin_subagents,
                mcp_servers,
                toolkits,
                visibility,
            }),
            Self::Variant1 { .. } => None,
        }
    }
}
