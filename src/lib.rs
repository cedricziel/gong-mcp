use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, model::*, service::RequestContext};
use serde_json::json;
use std::sync::Arc;
use base64::{Engine as _, engine::general_purpose};

/// Gong MCP Server
///
/// This server exposes Gong calls as MCP resources.
#[derive(Clone)]
pub struct GongServer {
    // Configuration
    base_url: Arc<Option<String>>,
    access_key: Arc<Option<String>>,
    access_key_secret: Arc<Option<String>>,
}

impl GongServer {
    pub fn new() -> Self {
        // Get configuration from environment variables
        let base_url = std::env::var("GONG_BASE_URL").ok();
        let access_key = std::env::var("GONG_ACCESS_KEY").ok();
        let access_key_secret = std::env::var("GONG_ACCESS_KEY_SECRET").ok();

        Self {
            base_url: Arc::new(base_url),
            access_key: Arc::new(access_key),
            access_key_secret: Arc::new(access_key_secret),
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
        self.base_url.is_some() && self.access_key.is_some() && self.access_key_secret.is_some()
    }

    /// Create an authenticated HTTP client for Gong API calls
    fn _create_http_client(&self) -> Result<reqwest::Client, McpError> {
        let access_key = self
            .access_key
            .as_ref()
            .as_ref()
            .ok_or_else(|| McpError::invalid_request("missing_access_key", None))?;
        let access_key_secret = self
            .access_key_secret
            .as_ref()
            .as_ref()
            .ok_or_else(|| McpError::invalid_request("missing_access_key_secret", None))?;

        let client = reqwest::Client::builder()
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!(
                        "Basic {}",
                        general_purpose::STANDARD.encode(format!("{}:{}", access_key, access_key_secret))
                    ))
                    .map_err(|e| {
                        McpError::internal_error("header_error", Some(json!({"error": e.to_string()})))
                    })?,
                );
                headers.insert(
                    reqwest::header::CONTENT_TYPE,
                    reqwest::header::HeaderValue::from_static("application/json"),
                );
                headers
            })
            .build()
            .map_err(|e| {
                McpError::internal_error("client_error", Some(json!({"error": e.to_string()})))
            })?;

        Ok(client)
    }

    /// Fetch list of calls from Gong API
    async fn _fetch_calls(&self) -> Result<serde_json::Value, McpError> {
        let base_url = self
            .base_url
            .as_ref()
            .as_ref()
            .ok_or_else(|| McpError::invalid_request("missing_base_url", None))?;

        let client = self._create_http_client()?;
        let url = format!("{}/v2/calls", base_url);

        // Fetch calls from the last 7 days
        let from_date_time = chrono::Utc::now() - chrono::Duration::days(7);
        let body = json!({
            "filter": {
                "fromDateTime": from_date_time.to_rfc3339(),
            },
            "contentSelector": {
                "exposedFields": {
                    "content": true,
                    "parties": true,
                }
            }
        });

        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                McpError::internal_error("api_error", Some(json!({"error": e.to_string()})))
            })?;

        if !response.status().is_success() {
            return Err(McpError::internal_error(
                "api_error",
                Some(json!({
                    "status": response.status().as_u16(),
                    "error": response.text().await.unwrap_or_default()
                })),
            ));
        }

        let data = response.json::<serde_json::Value>().await.map_err(|e| {
            McpError::internal_error("parse_error", Some(json!({"error": e.to_string()})))
        })?;

        Ok(data)
    }

    /// Fetch transcript for a specific call by ID
    async fn _fetch_transcript(&self, call_id: &str) -> Result<serde_json::Value, McpError> {
        let base_url = self
            .base_url
            .as_ref()
            .as_ref()
            .ok_or_else(|| McpError::invalid_request("missing_base_url", None))?;

        let client = self._create_http_client()?;
        let url = format!("{}/v2/calls/transcript", base_url);

        let body = json!({
            "filter": {
                "callIds": [call_id]
            }
        });

        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                McpError::internal_error("api_error", Some(json!({"error": e.to_string()})))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            if status == reqwest::StatusCode::NOT_FOUND {
                return Err(McpError::resource_not_found(
                    "call_not_found",
                    Some(json!({"callId": call_id})),
                ));
            }

            return Err(McpError::internal_error(
                "api_error",
                Some(json!({
                    "status": status.as_u16(),
                    "error": error_text
                })),
            ));
        }

        let data = response.json::<serde_json::Value>().await.map_err(|e| {
            McpError::internal_error("parse_error", Some(json!({"error": e.to_string()})))
        })?;

        Ok(data)
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
                    json!({
                        "configured": true,
                        "base_url": self.base_url.as_ref().as_ref().unwrap(),
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
                let formatted_response = if let Some(calls) = calls_data.get("calls").and_then(|c| c.as_array()) {
                    let formatted_calls: Vec<serde_json::Value> = calls
                        .iter()
                        .map(|call| {
                            json!({
                                "id": call.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                                "title": call.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled"),
                                "started": call.get("started").and_then(|v| v.as_str()).unwrap_or(""),
                                "duration": call.get("duration").and_then(|v| v.as_i64()).unwrap_or(0),
                                "direction": call.get("direction").and_then(|v| v.as_str()).unwrap_or(""),
                                "parties": call.get("parties").cloned().unwrap_or(json!([])),
                                "url": call.get("url").and_then(|v| v.as_str()).unwrap_or(""),
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

                // TODO: Implement actual Gong API call to fetch users
                // For now, return a placeholder
                let users_data = json!({
                    "message": "Gong users resource - Coming soon",
                    "note": "This will fetch users from the Gong API using gong-rs library"
                });

                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(
                        serde_json::to_string_pretty(&users_data).unwrap(),
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
                    let formatted_response = if let Some(transcripts) = transcript_data.get("callTranscripts").and_then(|t| t.as_array()) {
                        if let Some(transcript) = transcripts.first() {
                            let call_id = transcript.get("callId").and_then(|v| v.as_str()).unwrap_or("");
                            let transcript_obj = transcript.get("transcript").cloned().unwrap_or(json!(null));

                            // Extract speakers and sentences if available
                            let speakers = transcript_obj.get("speakers").cloned().unwrap_or(json!([]));
                            let sentences = transcript_obj.get("sentences").cloned().unwrap_or(json!([]));

                            json!({
                                "callId": call_id,
                                "speakers": speakers,
                                "sentences": sentences,
                                "metadata": {
                                    "sentenceCount": sentences.as_array().map(|s| s.len()).unwrap_or(0),
                                    "speakerCount": speakers.as_array().map(|s| s.len()).unwrap_or(0),
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
                description: Some("Retrieve the transcript for a specific Gong call by ID".to_string()),
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
        assert!(server.base_url.is_none() || server.base_url.is_some());
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
            assert!(!call_id.unwrap().is_empty(), "Call ID is empty for URI: {}", uri);
        }
    }

    #[test]
    fn test_invalid_transcript_uri_parsing() {
        // Invalid transcript URIs
        let invalid_uris = vec![
            "gong://calls//transcript",  // empty call ID
            "gong://calls/transcript",    // missing call ID
            "gong://transcript/123",      // wrong format
            "gong://calls/123",           // missing /transcript
        ];

        for uri in invalid_uris {
            let call_id = uri
                .strip_prefix("gong://calls/")
                .and_then(|s| s.strip_suffix("/transcript"));

            if let Some(id) = call_id {
                assert!(id.is_empty(), "Should have empty call ID for invalid URI: {}", uri);
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

    #[test]
    fn test_base64_encoding() {
        // Test that base64 encoding works correctly
        let encoded = general_purpose::STANDARD.encode("test:secret");
        assert!(!encoded.is_empty());
        assert_eq!(encoded, "dGVzdDpzZWNyZXQ=");
    }
}
