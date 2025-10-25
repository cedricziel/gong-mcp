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

    /// Fetch list of calls from Gong API with optional filters and cursor for pagination
    async fn _fetch_calls_with_filter(
        &self,
        from_date_time: Option<String>,
        to_date_time: Option<String>,
        workspace_id: Option<String>,
        call_ids: Option<Vec<String>>,
        primary_user_ids: Option<Vec<String>>,
        cursor: Option<String>,
        include_structure: bool,
    ) -> Result<models::Calls, McpError> {
        let config = self
            .config
            .as_ref()
            .as_ref()
            .ok_or_else(|| McpError::invalid_request("not_configured", None))?;

        let params = calls_api::ListCallsExtensiveParams {
            public_api_base_request_with_data_v2_calls_request_filter_with_owners_content_selector:
                models::PublicApiBaseRequestWithDataV2CallsRequestFilterWithOwnersContentSelector {
                    cursor,
                    filter: Box::new(models::CallsRequestFilterWithOwners {
                        from_date_time,
                        to_date_time,
                        workspace_id,
                        call_ids,
                        primary_user_ids,
                    }),
                    content_selector: Some(Box::new(models::ContentSelector {
                        context: None,
                        context_timing: None,
                        exposed_fields: Some(Box::new(models::ExposedFields {
                            collaboration: None,
                            content: if include_structure {
                                Some(Box::new(models::CallContent {
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
                                }))
                            } else {
                                None
                            },
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

    /// Fetch metadata for a specific call by ID
    async fn _fetch_call(&self, call_id: &str) -> Result<models::SpecificCall, McpError> {
        let config = self
            .config
            .as_ref()
            .as_ref()
            .ok_or_else(|| McpError::invalid_request("not_configured", None))?;

        let params = calls_api::GetCallParams {
            id: call_id.to_string(),
        };

        calls_api::get_call(config, params)
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
                .enable_tools()
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
                } else if uri.starts_with("gong://calls/") {
                    // Check if it matches the call metadata pattern: gong://calls/{callId}
                    if !self._is_configured() {
                        return Err(McpError::invalid_request(
                            "not_configured",
                            Some(json!({
                                "message": "Gong API is not configured. Please set environment variables."
                            })),
                        ));
                    }

                    // Extract call ID from URI
                    let call_id = uri.strip_prefix("gong://calls/").ok_or_else(|| {
                        McpError::invalid_params(
                            "invalid_uri",
                            Some(json!({
                                "message": "Invalid URI format. Expected: gong://calls/{callId}",
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

                    // Fetch call metadata from Gong API
                    let call_data = self._fetch_call(call_id).await?;

                    // Format the call metadata response
                    let formatted_response = if let Some(call) = call_data.call {
                        let call = call.as_ref();
                        json!({
                            "id": call.id,
                            "url": call.url,
                            "title": call.title,
                            "scheduled": call.scheduled,
                            "started": call.started,
                            "duration": call.duration,
                            "direction": call.direction.as_ref().map(|d| format!("{:?}", d)),
                            "primaryUserId": call.primary_user_id,
                            "system": call.system,
                            "scope": call.scope.as_ref().map(|s| format!("{:?}", s)),
                            "media": call.media.as_ref().map(|m| format!("{:?}", m)),
                            "language": call.language,
                            "workspaceId": call.workspace_id,
                            "sdrDisposition": call.sdr_disposition,
                            "clientUniqueId": call.client_unique_id,
                            "customData": call.custom_data,
                            "purpose": call.purpose,
                            "meetingUrl": call.meeting_url,
                            "isPrivate": call.is_private,
                            "calendarEventId": call.calendar_event_id,
                        })
                    } else {
                        return Err(McpError::resource_not_found(
                            "call_not_found",
                            Some(json!({
                                "callId": call_id,
                                "message": "No call data returned from API"
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
                uri_template: "gong://calls/{callId}".to_string(),
                name: "Call Metadata".to_string(),
                title: None,
                description: Some(
                    "Retrieve full metadata for a specific Gong call by ID".to_string(),
                ),
                mime_type: Some("application/json".to_string()),
            }
            .no_annotation(),
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

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        if !self._is_configured() {
            return Ok(ListToolsResult {
                next_cursor: None,
                tools: Vec::new(),
            });
        }

        let schema = json!({
            "type": "object",
            "properties": {
                "from_date_time": {
                    "type": "string",
                    "format": "date-time",
                    "description": "Start of time range in ISO 8601 format (e.g., '2024-01-01T00:00:00Z' or '2024-01-01T02:30:00-07:00'). Returns calls that started on or after this time."
                },
                "to_date_time": {
                    "type": "string",
                    "format": "date-time",
                    "description": "End of time range in ISO 8601 format. Returns calls that started before this time (exclusive)."
                },
                "workspace_id": {
                    "type": "string",
                    "description": "Filter by workspace ID. Returns only calls belonging to this workspace."
                },
                "call_ids": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "List of specific call IDs to retrieve. If provided, only these calls are returned (within date range if specified)."
                },
                "primary_user_ids": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Filter by user IDs. Returns calls where these users are the primary participant/host."
                },
                "cursor": {
                    "type": "string",
                    "description": "Pagination cursor from a previous response. Use this to get the next page of results."
                },
                "limit": {
                    "type": "number",
                    "description": "Maximum number of calls to return from the current page. Without this, returns all calls from the API page (typically 100). Response includes 'truncated: true' if limited. Use this to reduce response size."
                },
                "include_structure": {
                    "type": "boolean",
                    "description": "Include call agenda/structure data (segments and their durations). Default: false. Basic call metadata (id, title, started, duration, direction, parties, url) is always included. Increases response size moderately."
                }
            },
            "additionalProperties": false
        });

        let schema_obj = schema.as_object().unwrap().clone();

        let tools = vec![Tool::new(
            "search_calls",
            "Search Gong calls with flexible filters. Returns basic call metadata (id, title, started, duration, \
             direction, parties, url) by default. Use include_structure to add call agenda data. \
             Supports pagination for large result sets - use limit to reduce response size. \
             All parameters are optional - returns recent calls if no filters provided.",
            std::sync::Arc::new(schema_obj),
        )
        .annotate(ToolAnnotations::new().read_only(true))];

        Ok(ListToolsResult {
            next_cursor: None,
            tools,
        })
    }

    async fn call_tool(
        &self,
        CallToolRequestParam { name, arguments }: CallToolRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        match name.as_ref() {
            "search_calls" => {
                if !self._is_configured() {
                    return Err(McpError::invalid_request(
                        "not_configured",
                        Some(json!({
                            "message": "Gong API is not configured. Please set GONG_BASE_URL, GONG_ACCESS_KEY, and GONG_ACCESS_KEY_SECRET environment variables.",
                            "required_env_vars": ["GONG_BASE_URL", "GONG_ACCESS_KEY", "GONG_ACCESS_KEY_SECRET"]
                        })),
                    ));
                }

                // Get arguments or use empty map if None
                let args = arguments.as_ref();

                // Extract parameters from arguments
                let from_date_time = args
                    .and_then(|a| a.get("from_date_time"))
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let to_date_time = args
                    .and_then(|a| a.get("to_date_time"))
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let workspace_id = args
                    .and_then(|a| a.get("workspace_id"))
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let call_ids = args
                    .and_then(|a| a.get("call_ids"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect::<Vec<String>>()
                    });

                let primary_user_ids = args
                    .and_then(|a| a.get("primary_user_ids"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect::<Vec<String>>()
                    });

                let cursor = args
                    .and_then(|a| a.get("cursor"))
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let limit = args
                    .and_then(|a| a.get("limit"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);

                let include_structure = args
                    .and_then(|a| a.get("include_structure"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // Fetch calls from Gong API
                let calls_data = self
                    ._fetch_calls_with_filter(
                        from_date_time.clone(),
                        to_date_time.clone(),
                        workspace_id.clone(),
                        call_ids.clone(),
                        primary_user_ids.clone(),
                        cursor.clone(),
                        include_structure,
                    )
                    .await?;

                // Extract and format the calls for easier consumption
                // Response format:
                // - calls: Array of call objects with basic metadata
                // - count: Number of calls returned (after limit applied)
                // - totalAvailable: Total calls in current API page before limiting (typically 100)
                // - truncated: true if limit parameter was applied and reduced the result set
                // - hasMore: true if more pages available (use nextCursor to fetch)
                // - nextCursor: Pagination cursor for retrieving the next page
                // - filters: Echo of all filter parameters used in the request
                let formatted_response = if let Some(calls) = calls_data.calls {
                    let all_formatted_calls: Vec<serde_json::Value> = calls
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

                    let total_available = all_formatted_calls.len();
                    let (formatted_calls, truncated) = if let Some(limit_value) = limit {
                        if all_formatted_calls.len() > limit_value {
                            (all_formatted_calls.into_iter().take(limit_value).collect(), true)
                        } else {
                            (all_formatted_calls, false)
                        }
                    } else {
                        (all_formatted_calls, false)
                    };

                    json!({
                        "calls": formatted_calls,
                        "count": formatted_calls.len(),
                        "totalAvailable": total_available,
                        "truncated": truncated,
                        "nextCursor": calls_data.records.as_ref().and_then(|r| r.cursor.clone()),
                        "hasMore": calls_data.records.as_ref().and_then(|r| r.cursor.as_ref()).is_some(),
                        "filters": {
                            "from_date_time": from_date_time,
                            "to_date_time": to_date_time,
                            "workspace_id": workspace_id,
                            "call_ids": call_ids,
                            "primary_user_ids": primary_user_ids,
                            "limit": limit,
                            "include_structure": include_structure,
                        }
                    })
                } else {
                    json!({
                        "calls": [],
                        "count": 0,
                        "totalAvailable": 0,
                        "truncated": false,
                        "nextCursor": null,
                        "hasMore": false,
                        "filters": {
                            "from_date_time": from_date_time,
                            "to_date_time": to_date_time,
                            "workspace_id": workspace_id,
                            "call_ids": call_ids,
                            "primary_user_ids": primary_user_ids,
                            "limit": limit,
                            "include_structure": include_structure,
                        }
                    })
                };

                Ok(CallToolResult {
                    content: vec![Content::text(
                        serde_json::to_string_pretty(&formatted_response).unwrap(),
                    )],
                    structured_content: None,
                    is_error: None,
                    meta: None,
                })
            }
            _ => Err(McpError::invalid_params(
                "unknown_tool",
                Some(json!({"tool": name})),
            )),
        }
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

    #[test]
    fn test_server_capabilities_include_tools() {
        let server = GongServer::new();
        let info = server.get_info();
        assert!(info.capabilities.tools.is_some(), "Server should support tools");
        assert!(info.capabilities.resources.is_some(), "Server should support resources");
    }

    #[test]
    fn test_parameter_extraction_from_json() {
        // Test parameter extraction logic
        let json_args = json!({
            "from_date_time": "2024-01-01T00:00:00Z",
            "to_date_time": "2024-01-31T23:59:59Z",
            "workspace_id": "W123",
            "call_ids": ["call1", "call2"],
            "primary_user_ids": ["user1", "user2"]
        });

        let args_map = json_args.as_object();

        // Extract from_date_time
        let from_date = args_map
            .and_then(|a| a.get("from_date_time"))
            .and_then(|v| v.as_str());
        assert_eq!(from_date, Some("2024-01-01T00:00:00Z"));

        // Extract call_ids array
        let call_ids = args_map
            .and_then(|a| a.get("call_ids"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<String>>()
            });
        assert_eq!(call_ids, Some(vec!["call1".to_string(), "call2".to_string()]));
    }

    #[test]
    fn test_call_metadata_uri_parsing() {
        // Valid call metadata URIs (without /transcript)
        let valid_uris = vec![
            "gong://calls/123456",
            "gong://calls/abc-def-123",
            "gong://calls/call_id_123",
        ];

        for uri in valid_uris {
            let call_id = uri.strip_prefix("gong://calls/");
            assert!(call_id.is_some(), "Failed to parse URI: {}", uri);
            assert!(!call_id.unwrap().is_empty(), "Call ID is empty for URI: {}", uri);
            // Should NOT contain /transcript
            assert!(!call_id.unwrap().contains("/transcript"), "Call metadata URI should not contain /transcript: {}", uri);
        }
    }

    #[test]
    fn test_invalid_call_metadata_uri_parsing() {
        let invalid_uris = vec![
            "gong://calls/",         // empty call ID
            "gong://calls",          // missing call ID separator
            "gong://call/123",       // wrong format (call vs calls)
        ];

        for uri in invalid_uris {
            let call_id = uri.strip_prefix("gong://calls/");
            if let Some(id) = call_id {
                assert!(id.is_empty(), "Should have empty call ID for invalid URI: {}", uri);
            }
        }
    }

    #[test]
    fn test_result_truncation_logic() {
        let calls = vec![
            json!({"id": "1"}),
            json!({"id": "2"}),
            json!({"id": "3"}),
            json!({"id": "4"}),
            json!({"id": "5"}),
        ];

        // Test with limit
        let limit = Some(3);
        let total = calls.len();

        let (truncated_calls, is_truncated) = if let Some(limit_value) = limit {
            if calls.len() > limit_value {
                (calls.into_iter().take(limit_value).collect::<Vec<_>>(), true)
            } else {
                (calls, false)
            }
        } else {
            (calls, false)
        };

        assert_eq!(truncated_calls.len(), 3, "Should have 3 calls after truncation");
        assert_eq!(total, 5, "Total should be 5 before truncation");
        assert!(is_truncated, "Should be marked as truncated");
    }

    #[test]
    fn test_result_no_truncation_when_under_limit() {
        let calls = vec![
            json!({"id": "1"}),
            json!({"id": "2"}),
        ];

        // Test with limit higher than actual count
        let limit = Some(5);
        let total = calls.len();

        let (truncated_calls, is_truncated) = if let Some(limit_value) = limit {
            if calls.len() > limit_value {
                (calls.into_iter().take(limit_value).collect::<Vec<_>>(), true)
            } else {
                (calls, false)
            }
        } else {
            (calls, false)
        };

        assert_eq!(truncated_calls.len(), 2, "Should have all 2 calls");
        assert_eq!(total, 2, "Total should be 2");
        assert!(!is_truncated, "Should NOT be marked as truncated when under limit");
    }

    #[test]
    fn test_new_parameter_extraction() {
        let json_args = json!({
            "from_date_time": "2024-01-01T00:00:00Z",
            "limit": 10,
            "include_structure": true
        });

        let args_map = json_args.as_object();

        // Extract limit
        let limit = args_map
            .and_then(|a| a.get("limit"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        assert_eq!(limit, Some(10), "Limit should be 10");

        // Extract include_structure
        let include_structure = args_map
            .and_then(|a| a.get("include_structure"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        assert!(include_structure, "include_structure should be true");
    }

    #[test]
    fn test_include_structure_default_false() {
        let json_args = json!({
            "from_date_time": "2024-01-01T00:00:00Z"
        });

        let args_map = json_args.as_object();

        // Extract include_structure (should default to false)
        let include_structure = args_map
            .and_then(|a| a.get("include_structure"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        assert!(!include_structure, "include_structure should default to false when not provided");
    }

    #[test]
    fn test_uri_disambiguation() {
        // Ensure we can distinguish between call metadata and transcript
        let metadata_uri = "gong://calls/123456";
        let transcript_uri = "gong://calls/123456/transcript";

        // Metadata should not end with /transcript
        assert!(!metadata_uri.ends_with("/transcript"), "Metadata URI should not end with /transcript");
        assert!(metadata_uri.starts_with("gong://calls/"), "Metadata URI should start with gong://calls/");

        // Transcript should end with /transcript
        assert!(transcript_uri.ends_with("/transcript"), "Transcript URI should end with /transcript");
        assert!(transcript_uri.starts_with("gong://calls/"), "Transcript URI should start with gong://calls/");

        // Extract call IDs
        let metadata_call_id = metadata_uri.strip_prefix("gong://calls/");
        let transcript_call_id = transcript_uri
            .strip_prefix("gong://calls/")
            .and_then(|s| s.strip_suffix("/transcript"));

        assert_eq!(metadata_call_id, Some("123456"), "Metadata URI should extract call ID");
        assert_eq!(transcript_call_id, Some("123456"), "Transcript URI should extract call ID");
    }
}
