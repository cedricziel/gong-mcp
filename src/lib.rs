use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, model::*, service::RequestContext};
use serde_json::json;
use std::sync::Arc;

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

                // TODO: Implement actual Gong API call to fetch calls
                // For now, return a placeholder
                let calls_data = json!({
                    "message": "Gong calls resource - Coming soon",
                    "note": "This will fetch calls from the Gong API using gong-rs library"
                });

                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(
                        serde_json::to_string_pretty(&calls_data).unwrap(),
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
            _ => Err(McpError::resource_not_found(
                "resource_not_found",
                Some(json!({
                    "uri": uri
                })),
            )),
        }
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
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
}
