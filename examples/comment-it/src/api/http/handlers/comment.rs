// src/api/http/handlers/comment.rs
use axum::{
    extract::{Json, State},
    response::Json as ResponseJson,
    http::StatusCode,
};
use log::{info, error};
use crate::api::http::{
    state::PeerState,
    types::{SubmitCommentRequest, SubmitCommentResponse, GetCommentsRequest, GetCommentsResponse},
};

pub async fn submit_comment(
    State(state): State<PeerState>,
    Json(request): Json<SubmitCommentRequest>,
) -> Result<ResponseJson<SubmitCommentResponse>, StatusCode> {
    info!("🔥 COMMENT SUBMIT: episode_id={}, text_length={}", request.episode_id, request.text.len());
    
    // Basic validation
    if request.text.trim().is_empty() {
        error!("Comment text is empty");
        return Err(StatusCode::BAD_REQUEST);
    }
    
    if request.text.len() > 1000 {
        error!("Comment text too long: {} characters", request.text.len());
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Verify session token exists and is valid
    let episode_state = {
        let episodes = state.blockchain_episodes.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        episodes.get(&request.episode_id).cloned()
    };
    
    let episode = match episode_state {
        Some(episode) => episode,
        None => {
            error!("Episode {} not found", request.episode_id);
            return Err(StatusCode::NOT_FOUND);
        }
    };
    
    // Verify authentication
    if !episode.is_authenticated {
        error!("Episode {} is not authenticated", request.episode_id);
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    // Verify session token matches
    if let Some(ref session_token) = episode.session_token {
        if *session_token != request.session_token {
            error!("Session token mismatch for episode {}", request.episode_id);
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        error!("No session token for episode {}", request.episode_id);
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    // TODO: Create comment transaction and submit to blockchain
    // For now, return success response
    let response = SubmitCommentResponse {
        episode_id: request.episode_id,
        comment_id: 1, // TODO: Generate proper comment ID
        transaction_id: Some("pending_comment_tx".to_string()),
        status: "comment_submitted".to_string(),
    };
    
    info!("✅ COMMENT SUBMITTED: episode_id={}", request.episode_id);
    Ok(ResponseJson(response))
}

pub async fn get_comments(
    State(state): State<PeerState>,
    Json(request): Json<GetCommentsRequest>,
) -> Result<ResponseJson<GetCommentsResponse>, StatusCode> {
    info!("📚 GET COMMENTS: episode_id={}", request.episode_id);
    
    // Check if episode exists
    let episode_exists = {
        let episodes = state.blockchain_episodes.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        episodes.contains_key(&request.episode_id)
    };
    
    if !episode_exists {
        error!("Episode {} not found", request.episode_id);
        return Err(StatusCode::NOT_FOUND);
    }
    
    // TODO: Retrieve comments from blockchain/episode state
    // For now, return empty comments array
    let response = GetCommentsResponse {
        episode_id: request.episode_id,
        comments: vec![], // TODO: Load actual comments
        status: "comments_retrieved".to_string(),
    };
    
    info!("✅ COMMENTS RETRIEVED: episode_id={}, count={}", request.episode_id, response.comments.len());
    Ok(ResponseJson(response))
}