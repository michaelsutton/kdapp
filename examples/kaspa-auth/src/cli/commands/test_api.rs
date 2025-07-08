use clap::Args;
use reqwest::Client;
use serde_json::Value;
use std::error::Error;
use std::collections::HashMap;

#[derive(Args)]
pub struct TestApiCommand {
    #[arg(short, long, default_value = "http://localhost:8080")]
    pub server: String,
    
    #[arg(short, long)]
    pub verbose: bool,
    
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug)]
struct ApiEndpoint {
    method: &'static str,
    path: &'static str,
    description: &'static str,
    needs_data: bool,
    test_data: Option<serde_json::Value>,
}

impl TestApiCommand {
    pub async fn execute(self) -> Result<(), Box<dyn Error>> {
        let client = Client::new();
        let base_url = self.server.trim_end_matches('/');
        
        println!("🧪 Testing all API endpoints for: {}", base_url);
        println!("==================================================");
        println!();
        
        let endpoints = self.get_api_endpoints();
        let mut results = Vec::new();
        let mut episode_id: Option<u64> = None;
        
        for endpoint in endpoints {
            let result = self.test_endpoint(&client, base_url, &endpoint, episode_id).await;
            
            // Extract episode_id from successful POST /auth/start for later tests
            if endpoint.path == "/auth/start" && result.is_ok() {
                if let Ok(ref response) = result {
                    if let Some(id) = self.extract_episode_id(response) {
                        episode_id = Some(id);
                        println!("📝 Captured episode_id: {} for subsequent tests", id);
                        println!();
                    }
                }
            }
            
            results.push((endpoint, result));
        }
        
        // Summary
        println!("📊 SUMMARY");
        println!("==========");
        
        let mut success_count = 0;
        let mut total_count = 0;
        
        for (endpoint, result) in &results {
            total_count += 1;
            let status = match result {
                Ok(_) => {
                    success_count += 1;
                    "✅ PASS"
                },
                Err(_) => "❌ FAIL"
            };
            
            println!("{} {} {} - {}", 
                status, 
                endpoint.method, 
                endpoint.path, 
                endpoint.description
            );
            
            if let Err(e) = result {
                if self.verbose {
                    println!("    Error: {}", e);
                }
            }
        }
        
        println!();
        println!("📈 Results: {}/{} endpoints successful ({:.1}%)", 
            success_count, 
            total_count, 
            (success_count as f64 / total_count as f64) * 100.0
        );
        
        if success_count == total_count {
            println!("🎉 All endpoints working perfectly!");
        } else {
            println!("⚠️  Some endpoints failed - check server logs");
        }
        
        Ok(())
    }
    
    fn get_api_endpoints(&self) -> Vec<ApiEndpoint> {
        vec![
            ApiEndpoint {
                method: "GET",
                path: "/",
                description: "Server info",
                needs_data: false,
                test_data: None,
            },
            ApiEndpoint {
                method: "GET", 
                path: "/health",
                description: "Health check",
                needs_data: false,
                test_data: None,
            },
            ApiEndpoint {
                method: "GET",
                path: "/funding-info", 
                description: "Get funding address and economic parameters",
                needs_data: false,
                test_data: None,
            },
            ApiEndpoint {
                method: "POST",
                path: "/auth/start",
                description: "Create authentication episode", 
                needs_data: true,
                test_data: Some(serde_json::json!({
                    "public_key": "02DUMMY_TEST_PUBLIC_KEY_NOT_FOR_PRODUCTION_USE_ONLY_FOR_TESTING_PURPOSES"
                })),
            },
            ApiEndpoint {
                method: "POST",
                path: "/auth/register-episode",
                description: "Register blockchain episode with HTTP server",
                needs_data: true,
                test_data: Some(serde_json::json!({
                    "episode_id": 12345,
                    "public_key": "02DUMMY_TEST_PUBLIC_KEY_NOT_FOR_PRODUCTION_USE_ONLY_FOR_TESTING_PURPOSES"
                })),
            },
            ApiEndpoint {
                method: "POST", 
                path: "/auth/request-challenge",
                description: "Request challenge from blockchain",
                needs_data: true,
                test_data: Some(serde_json::json!({
                    "episode_id": "DYNAMIC_EPISODE_ID",
                    "public_key": "02DUMMY_TEST_PUBLIC_KEY_NOT_FOR_PRODUCTION_USE_ONLY_FOR_TESTING_PURPOSES"
                })),
            },
            ApiEndpoint {
                method: "POST",
                path: "/auth/sign-challenge", 
                description: "Sign challenge (helper endpoint)",
                needs_data: true,
                test_data: Some(serde_json::json!({
                    "challenge": "auth_1234567890",
                    "private_key": "DUMMY_TEST_KEY_NOT_FOR_PRODUCTION_USE_ONLY_FOR_TESTING_PURPOSES_DUMMY"
                })),
            },
            ApiEndpoint {
                method: "POST",
                path: "/auth/verify",
                description: "Submit authentication response", 
                needs_data: true,
                test_data: Some(serde_json::json!({
                    "episode_id": "DYNAMIC_EPISODE_ID",
                    "signature": "test_signature",
                    "nonce": "test_nonce"
                })),
            },
            ApiEndpoint {
                method: "GET",
                path: "/auth/status/DYNAMIC_EPISODE_ID",
                description: "Get episode status",
                needs_data: false,
                test_data: None,
            },
            ApiEndpoint {
                method: "GET", 
                path: "/challenge/DYNAMIC_EPISODE_ID",
                description: "Get challenge for episode (legacy)",
                needs_data: false,
                test_data: None,
            },
        ]
    }
    
    async fn test_endpoint(
        &self, 
        client: &Client, 
        base_url: &str, 
        endpoint: &ApiEndpoint,
        episode_id: Option<u64>
    ) -> Result<String, Box<dyn Error>> {
        // Replace dynamic placeholders
        let mut path = endpoint.path.to_string();
        let mut test_data = endpoint.test_data.clone();
        
        if let Some(id) = episode_id {
            path = path.replace("DYNAMIC_EPISODE_ID", &id.to_string());
            if let Some(ref mut data) = test_data {
                if let Some(obj) = data.as_object_mut() {
                    if obj.contains_key("episode_id") {
                        obj.insert("episode_id".to_string(), serde_json::Value::Number(id.into()));
                    }
                }
            }
        } else if path.contains("DYNAMIC_EPISODE_ID") {
            // Skip endpoints that need episode_id but we don't have one yet
            return Err("Skipped - no episode_id available yet".into());
        }
        
        let url = format!("{}{}", base_url, path);
        
        println!("🔍 Testing: {} {} - {}", endpoint.method, path, endpoint.description);
        
        let response = match endpoint.method {
            "GET" => {
                client.get(&url).send().await?
            },
            "POST" => {
                let mut request = client.post(&url).header("Content-Type", "application/json");
                if let Some(data) = test_data {
                    request = request.json(&data);
                }
                request.send().await?
            },
            _ => return Err("Unsupported HTTP method".into()),
        };
        
        let status = response.status();
        let response_text = response.text().await?;
        
        if self.verbose || !status.is_success() {
            println!("   Status: {}", status);
            if self.json {
                if let Ok(json) = serde_json::from_str::<Value>(&response_text) {
                    println!("   Response: {}", serde_json::to_string_pretty(&json)?);
                } else {
                    println!("   Response: {}", response_text);
                }
            } else {
                println!("   Response: {}", 
                    if response_text.len() > 100 { 
                        format!("{}...", &response_text[..100]) 
                    } else { 
                        response_text.clone() 
                    }
                );
            }
        }
        
        if status.is_success() {
            println!("   ✅ Success");
        } else {
            println!("   ❌ Failed");
        }
        println!();
        
        if status.is_success() {
            Ok(response_text)
        } else {
            Err(format!("HTTP {} - {}", status, response_text).into())
        }
    }
    
    fn extract_episode_id(&self, response: &str) -> Option<u64> {
        if let Ok(json) = serde_json::from_str::<Value>(response) {
            json.get("episode_id")?.as_u64()
        } else {
            None
        }
    }
}