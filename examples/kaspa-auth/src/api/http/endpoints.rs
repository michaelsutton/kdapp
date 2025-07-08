pub struct Endpoint {
    pub method: &'static str,
    pub path: &'static str,
    pub description: &'static str,
}

pub fn get_api_endpoints() -> Vec<Endpoint> {
    vec![
        Endpoint { method: "GET", path: "/", description: "Server info" },
        Endpoint { method: "GET", path: "/health", description: "Health check" },
        Endpoint { method: "GET", path: "/web", description: "Web UI Dashboard" },
        Endpoint { method: "GET", path: "/funding-info", description: "Get funding address and economic parameters" },
        Endpoint { method: "GET", path: "/ws", description: "WebSocket connection for real-time updates" },
        Endpoint { method: "POST", path: "/auth/start", description: "Create authentication episode" },
        Endpoint { method: "POST", path: "/auth/register-episode", description: "Register blockchain episode with HTTP server" },
        Endpoint { method: "POST", path: "/auth/request-challenge", description: "Request challenge from blockchain" },
        Endpoint { method: "POST", path: "/auth/sign-challenge", description: "Sign challenge (helper endpoint)" },
        Endpoint { method: "POST", path: "/auth/verify", description: "Submit authentication response" },
        Endpoint { method: "GET", path: "/auth/status/{episode_id}", description: "Get episode status" },
        Endpoint { method: "GET", path: "/challenge/{episode_id}", description: "Get challenge for episode (legacy)" },
    ]
}
