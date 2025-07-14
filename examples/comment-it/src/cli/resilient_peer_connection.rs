// src/cli/resilient_peer_connection.rs
use crate::cli::config::{CommentItConfig, OrganizerPeer};
use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::timeout;

#[derive(Debug)]
pub struct ResilientPeerConnection {
    config: CommentItConfig,
    http_peer: Client,
    peer_stats: std::collections::HashMap<String, PeerStats>,
}

#[derive(Debug, Clone)]
struct PeerStats {
    success_count: u32,
    failure_count: u32,
    last_success: Option<Instant>,
    last_failure: Option<Instant>,
    average_response_time: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct ApiRequest {
    pub method: HttpMethod,
    pub path: String,
    pub body: Option<Value>,
}

#[derive(Debug, Clone)]
pub enum HttpMethod {
    GET,
    POST,
}

#[derive(Debug)]
pub struct ApiResponse {
    pub success: bool,
    pub data: Option<Value>,
    pub error: Option<String>,
    pub peer_used: String,
    pub response_time: Duration,
}

impl ResilientPeerConnection {
    pub fn new(config: CommentItConfig) -> Self {
        let http_peer = Client::builder()
            .timeout(Duration::from_secs(config.resilience.request_timeout_seconds))
            .build()
            .unwrap();
        
        Self {
            config,
            http_peer,
            peer_stats: std::collections::HashMap::new(),
        }
    }
    
    /// Make a resilient API request with automatic fallback
    pub async fn request(&mut self, request: ApiRequest) -> Result<ApiResponse, Box<dyn std::error::Error>> {
        let enabled_peers: Vec<OrganizerPeer> = self.config.get_enabled_peers().into_iter().cloned().collect();
        
        if enabled_peers.is_empty() {
            return Err("No enabled organizer peers available".into());
        }
        
        println!("🔄 Attempting request to {} peers: {}", 
                enabled_peers.len(), 
                enabled_peers.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", "));
        
        let mut last_error = None;
        
        for (attempt, peer) in enabled_peers.iter().enumerate() {
            println!("🎯 Attempt {} - Trying peer '{}' at {}", 
                    attempt + 1, peer.name, peer.url);
            
            let start_time = Instant::now();
            
            // Try this peer with retries
            for retry in 0..self.config.resilience.max_retries_per_peer {
                if retry > 0 {
                    println!("   ↺ Retry {} for peer '{}'", retry + 1, peer.name);
                }
                
                match self.try_peer(peer, &request).await {
                    Ok(mut response) => {
                        let response_time = start_time.elapsed();
                        response.response_time = response_time;
                        response.peer_used = peer.name.clone();
                        
                        self.record_success(&peer.name, response_time);
                        
                        println!("✅ SUCCESS on peer '{}' ({}ms)", 
                                peer.name, response_time.as_millis());
                        
                        return Ok(response);
                    }
                    Err(e) => {
                        self.record_failure(&peer.name);
                        last_error = Some(e);
                        
                        if retry < self.config.resilience.max_retries_per_peer - 1 {
                            println!("   ❌ Retry failed for '{}': {}", peer.name, last_error.as_ref().unwrap());
                            tokio::time::sleep(Duration::from_millis(1000 * (retry + 1) as u64)).await;
                        }
                    }
                }
            }
            
            println!("❌ Peer '{}' failed after {} retries", 
                    peer.name, self.config.resilience.max_retries_per_peer);
            
            if !self.config.resilience.try_all_peers {
                break;
            }
        }
        
        Err(format!("All organizer peers failed. Last error: {}", 
                   last_error.map(|e| e.to_string()).unwrap_or_else(|| "Unknown error".to_string())).into())
    }
    
    /// Try a single peer once
    async fn try_peer(&self, peer: &OrganizerPeer, request: &ApiRequest) -> Result<ApiResponse, Box<dyn std::error::Error>> {
        let url = format!("{}{}", peer.url, request.path);
        
        let response = match request.method {
            HttpMethod::GET => {
                timeout(
                    Duration::from_secs(self.config.resilience.request_timeout_seconds),
                    self.http_peer.get(&url).send()
                ).await??
            }
            HttpMethod::POST => {
                let mut req = self.http_peer.post(&url);
                
                if let Some(body) = &request.body {
                    req = req.json(body);
                }
                
                timeout(
                    Duration::from_secs(self.config.resilience.request_timeout_seconds),
                    req.send()
                ).await??
            }
        };
        
        if response.status().is_success() {
            let data: Value = response.json().await?;
            Ok(ApiResponse {
                success: true,
                data: Some(data),
                error: None,
                peer_used: peer.name.clone(),
                response_time: Duration::default(), // Will be set by caller
            })
        } else {
            Err(format!("HTTP error: {}", response.status()).into())
        }
    }
    
    /// Record successful peer interaction
    fn record_success(&mut self, peer_name: &str, response_time: Duration) {
        let stats = self.peer_stats.entry(peer_name.to_string()).or_insert_with(|| PeerStats {
            success_count: 0,
            failure_count: 0,
            last_success: None,
            last_failure: None,
            average_response_time: None,
        });
        
        stats.success_count += 1;
        stats.last_success = Some(Instant::now());
        
        // Update average response time
        stats.average_response_time = Some(match stats.average_response_time {
            Some(avg) => Duration::from_millis(
                (avg.as_millis() as u64 + response_time.as_millis() as u64) / 2
            ),
            None => response_time,
        });
    }
    
    /// Record failed peer interaction
    fn record_failure(&mut self, peer_name: &str) {
        let stats = self.peer_stats.entry(peer_name.to_string()).or_insert_with(|| PeerStats {
            success_count: 0,
            failure_count: 0,
            last_success: None,
            last_failure: None,
            average_response_time: None,
        });
        
        stats.failure_count += 1;
        stats.last_failure = Some(Instant::now());
    }
    
    /// Get peer statistics for monitoring
    pub fn get_peer_stats(&self) -> &std::collections::HashMap<String, PeerStats> {
        &self.peer_stats
    }
    
    /// Update peer reputation based on performance
    pub fn update_peer_reputations(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (peer_name, stats) in &self.peer_stats {
            let total_requests = stats.success_count + stats.failure_count;
            if total_requests < 5 {
                continue; // Need more data
            }
            
            let success_rate = (stats.success_count as f64 / total_requests as f64) * 100.0;
            let base_reputation = success_rate as u8;
            
            // Bonus for fast response times
            let time_bonus = if let Some(avg_time) = stats.average_response_time {
                if avg_time < Duration::from_millis(500) {
                    10
                } else if avg_time < Duration::from_secs(2) {
                    5
                } else {
                    0
                }
            } else {
                0
            };
            
            let new_reputation = (base_reputation + time_bonus).min(100);
            self.config.update_peer_reputation(peer_name, new_reputation);
        }
        
        // Save updated config
        self.config.save()?;
        Ok(())
    }
    
    /// Convenience methods for common API calls
    pub async fn start_auth(&mut self, public_key: &str) -> Result<ApiResponse, Box<dyn std::error::Error>> {
        let request = ApiRequest {
            method: HttpMethod::POST,
            path: "/auth/start".to_string(),
            body: Some(serde_json::json!({
                "public_key": public_key
            })),
        };
        
        self.request(request).await
    }
    
    pub async fn get_challenge(&mut self, episode_id: u64) -> Result<ApiResponse, Box<dyn std::error::Error>> {
        let request = ApiRequest {
            method: HttpMethod::GET,
            path: format!("/auth/challenge/{}", episode_id),
            body: None,
        };
        
        self.request(request).await
    }
    
    pub async fn verify_auth(&mut self, episode_id: u64, signature: &str, nonce: &str) -> Result<ApiResponse, Box<dyn std::error::Error>> {
        let request = ApiRequest {
            method: HttpMethod::POST,
            path: "/auth/verify".to_string(),
            body: Some(serde_json::json!({
                "episode_id": episode_id,
                "signature": signature,
                "nonce": nonce
            })),
        };
        
        self.request(request).await
    }
    
    pub async fn revoke_session(&mut self, episode_id: u64, session_token: &str) -> Result<ApiResponse, Box<dyn std::error::Error>> {
        let request = ApiRequest {
            method: HttpMethod::POST,
            path: "/auth/revoke-session".to_string(),
            body: Some(serde_json::json!({
                "episode_id": episode_id,
                "session_token": session_token
            })),
        };
        
        self.request(request).await
    }
    
    pub async fn get_wallet_status(&mut self) -> Result<ApiResponse, Box<dyn std::error::Error>> {
        let request = ApiRequest {
            method: HttpMethod::GET,
            path: "/wallet/status".to_string(),
            body: None,
        };
        
        self.request(request).await
    }
}

impl PeerStats {
    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            0.0
        } else {
            (self.success_count as f64 / total as f64) * 100.0
        }
    }
    
    pub fn is_healthy(&self) -> bool {
        self.success_rate() > 50.0 && self.failure_count < 10
    }
}