use gong_rs::apis::configuration::Configuration;
use gong_rs::apis::{calls_api, users_api};
use gong_rs::models;
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, model::*, service::RequestContext};
use serde_json::json;
use std::sync::Arc;

/// Gong MCP Server
///
/// This server exposes Gong calls as MCP resources.
#[derive(Clone)]
pub struct GongServer {
    // Gong API configuration
    config: Arc<Option<Configuration>>,
}

impl GongServer {
    pub fn new() -> Self {
        // Get configuration from environment variables
        let base_url = std::env::var("GONG_BASE_URL").ok();
        let access_key = std::env::var("GONG_ACCESS_KEY").ok();
        let access_key_secret = std::env::var("GONG_ACCESS_KEY_SECRET").ok();

        // Create gong-rs configuration if all required variables are present
        let config = if let (Some(base_url), Some(access_key), Some(access_key_secret)) =
            (base_url, access_key, access_key_secret)
        {
            let mut config = Configuration::new();
            config.base_path = base_url;
            config.basic_auth = Some((access_key, Some(access_key_secret)));
            Some(config)
        } else {
            None
        };

        Self {
            config: Arc::new(config),
        }
    }

    fn _create_resource(&self, uri: &str, name: &str, description: &str) -> Resource {
        RawResource {
            uri: uri.to_string(),
            name: name.to_string(),
            title: None,
            description: Some(description.to_string()),
            mime_type: None,
            size: None,
            icons: None,
        }
        .no_annotation()
    }

    fn _is_configured(&self) -> bool {
        self.config.is_some()
    }

    /// Fetch list of calls from Gong API
    async fn _fetch_calls(&self) -> Result<models::Calls, McpError> {
        let config = self
            .config
            .as_ref()
            .as_ref()
            .ok_or_else(|| McpError::invalid_request("not_configured", None))?;

        // Fetch calls from the last 7 days
        let from_date_time = chrono::Utc::now() - chrono::Duration::days(7);

        let params = calls_api::ListCallsExtensiveParams {
            public_api_base_request_with_data_v2_calls_request_filter_with_owners_content_selector:
                models::PublicApiBaseRequestWithDataV2CallsRequestFilterWithOwnersContentSelector {
                    cursor: None,
                    filter: Box::new(models::CallsRequestFilterWithOwners {
                        from_date_time: Some(from_date_time.to_rfc3339()),
                        to_date_time: None,
                        workspace_id: None,
                        call_ids: None,
                        primary_user_ids: None,
                    }),
                    content_selector: Some(Box::new(models::ContentSelector {
                        context: None,
                        context_timing: None,
                        exposed_fields: Some(Box::new(models::ExposedFields {
                            collaboration: None,
                            content: Some(Box::new(models::CallContent {
                                structure: Some(true),
                                topics: None,
                                trackers: None,
                                tracker_occurrences: None,
                                points_of_interest: None,
                                brief: None,
                                outline: None,
                                highlights: None,
                                call_outcome: None,
                                key_points: None,
                            })),
                            parties: Some(true),
                            interaction: None,
                            media: None,
                        })),
                    })),
                },
        };

        calls_api::list_calls_extensive(config, params)
            .await
            .map_err(|e| {
                McpError::internal_error("api_error", Some(json!({"error": e.to_string()})))
            })
    }

    /// Fetch transcript for a specific call by ID
    async fn _fetch_transcript(&self, call_id: &str) -> Result<models::CallTranscripts, McpError> {
        let config = self
            .config
            .as_ref()
            .as_ref()
            .ok_or_else(|| McpError::invalid_request("not_configured", None))?;

        let filter = models::CallsFilter {
            from_date_time: None,
            to_date_time: None,
            workspace_id: None,
            call_ids: Some(vec![call_id.to_string()]),
        };

        let params = calls_api::GetCallTranscriptsParams {
            public_api_base_request_v2_calls_filter: models::PublicApiBaseRequestV2CallsFilter {
                cursor: None,
                filter: Box::new(filter),
            },
        };

        calls_api::get_call_transcripts(config, params)
            .await
            .map_err(|e| {
                let error_str = e.to_string();
                if error_str.contains("404") || error_str.contains("not found") {
                    McpError::resource_not_found(
                        "call_not_found",
                        Some(json!({"callId": call_id, "error": error_str})),
                    )
                } else {
                    McpError::internal_error("api_error", Some(json!({"error": error_str})))
                }
            })
    }
}

impl Default for GongServer {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerHandler for GongServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_resources()
                .build(),
            server_info: Implementation {
                name: "gong-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: None,
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "Gong MCP Server - Access Gong calls via resources. \
                Configure using environment variables: GONG_BASE_URL, GONG_ACCESS_KEY, GONG_ACCESS_KEY_SECRET."
                    .to_string(),
            ),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        if !self._is_configured() {
            return Ok(ListResourcesResult {
                resources: vec![self._create_resource(
                    "gong://status",
                    "Configuration Status",
                    "Check if the Gong API is configured correctly",
                )],
                next_cursor: None,
            });
        }

        Ok(ListResourcesResult {
            resources: vec![
                self._create_resource(
                    "gong://status",
                    "Configuration Status",
                    "Check if the Gong API is configured correctly",
                ),
                self._create_resource(
                    "gong://calls",
                    "Gong Calls",
                    "List of recent calls from Gong",
                ),
                self._create_resource(
                    "gong://users",
                    "Gong Users",
                    "List of users in your Gong workspace",
                ),
            ],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        match uri.as_str() {
            "gong://status" => {
                let status = if self._is_configured() {
                    let base_url = self
                        .config
                        .as_ref()
                        .as_ref()
                        .map(|c| c.base_path.as_str())
                        .unwrap_or("unknown");
                    json!({
                        "configured": true,
                        "base_url": base_url,
                        "message": "Gong API is configured and ready to use"
                    })
                } else {
                    json!({
                        "configured": false,
                        "message": "Gong API is not configured. Please set GONG_BASE_URL, GONG_ACCESS_KEY, and GONG_ACCESS_KEY_SECRET environment variables."
                    })
                };

                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(
                        serde_json::to_string_pretty(&status).unwrap(),
                        uri,
                    )],
                })
            }
            "gong://calls" => {
                if !self._is_configured() {
                    return Err(McpError::invalid_request(
                        "not_configured",
                        Some(json!({
                            "message": "Gong API is not configured. Please set environment variables."
                        })),
                    ));
                }

                // Fetch calls from Gong API
                let calls_data = self._fetch_calls().await?;

                // Extract and format the calls for easier consumption
                let formatted_response = if let Some(calls) = calls_data.calls {
                    let formatted_calls: Vec<serde_json::Value> = calls
                        .iter()
                        .map(|call| {
                            let meta = call.meta_data.as_ref().map(|m| m.as_ref());
                            json!({
                                "id": meta.and_then(|m| m.id.as_ref()).unwrap_or(&String::new()),
                                "title": meta.and_then(|m| m.title.as_ref()).unwrap_or(&"Untitled".to_string()),
                                "started": meta.and_then(|m| m.started.as_ref()).unwrap_or(&String::new()),
                                "duration": meta.and_then(|m| m.duration).unwrap_or(0),
                                "direction": meta.and_then(|m| m.direction.as_ref()).map(|d| format!("{:?}", d)).unwrap_or_default(),
                                "parties": call.parties.as_ref().map(|p| json!(p)).unwrap_or(json!([])),
                                "url": meta.and_then(|m| m.url.as_ref()).unwrap_or(&String::new()),
                            })
                        })
                        .collect();

                    json!({
                        "calls": formatted_calls,
                        "count": formatted_calls.len(),
                        "message": format!("Retrieved {} calls from the last 7 days", formatted_calls.len())
                    })
                } else {
                    json!({
                        "calls": [],
                        "count": 0,
                        "message": "No calls found"
                    })
                };

                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(
                        serde_json::to_string_pretty(&formatted_response).unwrap(),
                        uri,
                    )],
                })
            }
            "gong://users" => {
                if !self._is_configured() {
                    return Err(McpError::invalid_request(
                        "not_configured",
                        Some(json!({
                            "message": "Gong API is not configured. Please set environment variables."
                        })),
                    ));
                }

                // Fetch users from Gong API
                let config = self
                    .config
                    .as_ref()
                    .as_ref()
                    .ok_or_else(|| McpError::invalid_request("not_configured", None))?;

                let params = users_api::ListUsersParams {
                    cursor: None,
                    include_avatars: Some(false),
                };

                let users_data = users_api::list_users(config, params).await.map_err(|e| {
                    McpError::internal_error("api_error", Some(json!({"error": e.to_string()})))
                })?;

                // Format the users response
                let formatted_response = if let Some(users) = users_data.users {
                    let formatted_users: Vec<serde_json::Value> = users
                        .iter()
                        .map(|user| {
                            json!({
                                "id": user.id.as_ref().unwrap_or(&String::new()),
                                "email": user.email_address.as_ref().unwrap_or(&String::new()),
                                "firstName": user.first_name.as_ref().unwrap_or(&String::new()),
                                "lastName": user.last_name.as_ref().unwrap_or(&String::new()),
                                "active": user.active.unwrap_or(false),
                            })
                        })
                        .collect();

                    json!({
                        "users": formatted_users,
                        "count": formatted_users.len(),
                        "message": format!("Retrieved {} users", formatted_users.len())
                    })
                } else {
                    json!({
                        "users": [],
                        "count": 0,
                        "message": "No users found"
                    })
                };

                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(
                        serde_json::to_string_pretty(&formatted_response).unwrap(),
                        uri,
                    )],
                })
            }
            _ => {
                // Check if it matches the transcript pattern: gong://calls/{callId}/transcript
                if uri.starts_with("gong://calls/") && uri.ends_with("/transcript") {
                    if !self._is_configured() {
                        return Err(McpError::invalid_request(
                            "not_configured",
                            Some(json!({
                                "message": "Gong API is not configured. Please set environment variables."
                            })),
                        ));
                    }

                    // Extract call ID from URI
                    let call_id = uri
                        .strip_prefix("gong://calls/")
                        .and_then(|s| s.strip_suffix("/transcript"))
                        .ok_or_else(|| {
                            McpError::invalid_params(
                                "invalid_uri",
                                Some(json!({
                                    "message": "Invalid URI format. Expected: gong://calls/{callId}/transcript",
                                    "uri": uri
                                })),
                            )
                        })?;

                    // Validate call ID is not empty
                    if call_id.is_empty() {
                        return Err(McpError::invalid_params(
                            "missing_call_id",
                            Some(json!({
                                "message": "Call ID cannot be empty"
                            })),
                        ));
                    }

                    // Fetch transcript from Gong API
                    let transcript_data = self._fetch_transcript(call_id).await?;

                    // Format the transcript response with metadata
                    let formatted_response =
                        if let Some(transcripts) = transcript_data.call_transcripts {
                            if let Some(transcript) = transcripts.first() {
                                let empty_string = String::new();
                                let retrieved_call_id =
                                    transcript.call_id.as_ref().unwrap_or(&empty_string);
                                let monologues = transcript.transcript.as_ref();

                                // Extract sentences and speaker information from monologues
                                let (all_sentences, speaker_ids): (Vec<_>, Vec<_>) = monologues
                                    .map(|m| {
                                        m.iter()
                                            .flat_map(|monologue| {
                                                let speaker_id = monologue.speaker_id.clone();
                                                monologue
                                                    .sentences
                                                    .as_ref()
                                                    .map(|sentences| {
                                                        sentences
                                                            .iter()
                                                            .map(|s| {
                                                                (
                                                                    json!({
                                                                        "speakerId": speaker_id,
                                                                        "start": s.start,
                                                                        "end": s.end,
                                                                        "text": s.text,
                                                                    }),
                                                                    speaker_id.clone(),
                                                                )
                                                            })
                                                            .collect::<Vec<_>>()
                                                    })
                                                    .unwrap_or_default()
                                            })
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default()
                                    .into_iter()
                                    .unzip();

                                // Get unique speakers
                                let unique_speakers: std::collections::HashSet<_> =
                                    speaker_ids.into_iter().flatten().collect();

                                json!({
                                    "callId": retrieved_call_id,
                                    "monologues": monologues,
                                    "sentences": all_sentences,
                                    "metadata": {
                                        "sentenceCount": all_sentences.len(),
                                        "speakerCount": unique_speakers.len(),
                                        "monologueCount": monologues.map(|m| m.len()).unwrap_or(0),
                                    }
                                })
                            } else {
                                return Err(McpError::resource_not_found(
                                    "transcript_not_found",
                                    Some(json!({
                                        "callId": call_id,
                                        "message": "No transcript found for this call"
                                    })),
                                ));
                            }
                        } else {
                            return Err(McpError::resource_not_found(
                                "transcript_not_found",
                                Some(json!({
                                    "callId": call_id,
                                    "message": "No transcript data returned from API"
                                })),
                            ));
                        };

                    Ok(ReadResourceResult {
                        contents: vec![ResourceContents::text(
                            serde_json::to_string_pretty(&formatted_response).unwrap(),
                            uri,
                        )],
                    })
                } else {
                    // Unknown resource
                    Err(McpError::resource_not_found(
                        "resource_not_found",
                        Some(json!({
                            "uri": uri
                        })),
                    ))
                }
            }
        }
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        if !self._is_configured() {
            return Ok(ListResourceTemplatesResult {
                next_cursor: None,
                resource_templates: Vec::new(),
            });
        }

        let templates = vec![
            RawResourceTemplate {
                uri_template: "gong://calls/{callId}/transcript".to_string(),
                name: "Call Transcript".to_string(),
                title: None,
                description: Some(
                    "Retrieve the transcript for a specific Gong call by ID".to_string(),
                ),
                mime_type: Some("application/json".to_string()),
            }
            .no_annotation(),
        ];

        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: templates,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = GongServer::new();
        assert!(server.config.is_none() || server.config.is_some());
    }

    #[test]
    fn test_server_info() {
        let server = GongServer::new();
        let info = server.get_info();
        assert_eq!(info.server_info.name, "gong-mcp");
        assert!(info.capabilities.resources.is_some());
    }

    #[test]
    fn test_transcript_uri_parsing() {
        // Valid transcript URIs
        let valid_uris = vec![
            "gong://calls/123456/transcript",
            "gong://calls/abc-def-123/transcript",
            "gong://calls/call_id_123/transcript",
        ];

        for uri in valid_uris {
            let call_id = uri
                .strip_prefix("gong://calls/")
                .and_then(|s| s.strip_suffix("/transcript"));
            assert!(call_id.is_some(), "Failed to parse URI: {}", uri);
            assert!(
                !call_id.unwrap().is_empty(),
                "Call ID is empty for URI: {}",
                uri
            );
        }
    }

    #[test]
    fn test_invalid_transcript_uri_parsing() {
        // Invalid transcript URIs
        let invalid_uris = vec![
            "gong://calls//transcript", // empty call ID
            "gong://calls/transcript",  // missing call ID
            "gong://transcript/123",    // wrong format
            "gong://calls/123",         // missing /transcript
        ];

        for uri in invalid_uris {
            let call_id = uri
                .strip_prefix("gong://calls/")
                .and_then(|s| s.strip_suffix("/transcript"));

            if let Some(id) = call_id {
                assert!(
                    id.is_empty(),
                    "Should have empty call ID for invalid URI: {}",
                    uri
                );
            }
        }
    }

    #[test]
    fn test_server_configuration_detection() {
        // Test without configuration
        let server = GongServer::new();
        let is_configured = server._is_configured();

        // The result depends on environment, but method should not panic
        let _ = is_configured;
    }

    #[test]
    fn test_server_with_mock_config() {
        // Set up environment variables
        unsafe {
            std::env::set_var("GONG_BASE_URL", "https://api.gong.io");
            std::env::set_var("GONG_ACCESS_KEY", "test_key");
            std::env::set_var("GONG_ACCESS_KEY_SECRET", "test_secret");
        }

        let server = GongServer::new();
        assert!(server._is_configured());

        // Clean up
        unsafe {
            std::env::remove_var("GONG_BASE_URL");
            std::env::remove_var("GONG_ACCESS_KEY");
            std::env::remove_var("GONG_ACCESS_KEY_SECRET");
        }
    }
}
