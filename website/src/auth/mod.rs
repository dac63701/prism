use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;

pub mod api_key;
pub mod jwt;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub role: String,
}

#[allow(dead_code)]
pub struct AdminUser(pub AuthUser);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    PgPool: axum::extract::FromRef<S>,
    Config: axum::extract::FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = jwt::extract_bearer_token(parts).ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authorization header"})),
            )
                .into_response()
        })?;

        let config = Config::from_ref(state);
        let claims = jwt::verify_access_token(&token, &config.jwt_secret).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid or expired token"})),
            )
                .into_response()
        })?;

        let pool = PgPool::from_ref(state);
        let user = crate::db::users::get_user_by_id(&pool, claims.sub)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Database error"})),
                )
                    .into_response()
            })?
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "User not found"})),
                )
                    .into_response()
            })?;

        if user.is_banned {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({"error": "Account is banned"})),
            )
                .into_response());
        }

        Ok(AuthUser {
            user_id: claims.sub,
            role: claims.role,
        })
    }
}

impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
    PgPool: axum::extract::FromRef<S>,
    Config: axum::extract::FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.role != "admin" {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({"error": "Admin access required"})),
            )
                .into_response());
        }
        Ok(AdminUser(user))
    }
}

#[allow(dead_code)]
pub struct ApiKeyOrJwtAuth(pub AuthUser);

impl<S> FromRequestParts<S> for ApiKeyOrJwtAuth
where
    S: Send + Sync,
    PgPool: axum::extract::FromRef<S>,
    Config: axum::extract::FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = PgPool::from_ref(state);

        if let Some(key) = jwt::extract_bearer_token(parts) {
            if key.starts_with("prism_") {
                match api_key::verify_api_key(&pool, &key).await {
                    Ok(user_id) => {
                        return Ok(ApiKeyOrJwtAuth(AuthUser {
                            user_id,
                            role: "user".into(),
                        }));
                    }
                    Err(_) => {
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            Json(json!({"error": "Invalid API key"})),
                        )
                            .into_response());
                    }
                }
            }

            let config = Config::from_ref(state);
            match jwt::verify_access_token(&key, &config.jwt_secret) {
                Ok(claims) => {
                    let user = crate::db::users::get_user_by_id(&pool, claims.sub)
                        .await
                        .map_err(|_| {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({"error": "Database error"})),
                            )
                                .into_response()
                        })?
                        .ok_or_else(|| {
                            (
                                StatusCode::UNAUTHORIZED,
                                Json(json!({"error": "User not found"})),
                            )
                                .into_response()
                        })?;

                    if user.is_banned {
                        return Err((
                            StatusCode::FORBIDDEN,
                            Json(json!({"error": "Account is banned"})),
                        )
                            .into_response());
                    }

                    return Ok(ApiKeyOrJwtAuth(AuthUser {
                        user_id: claims.sub,
                        role: claims.role,
                    }));
                }
                Err(_) => {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        Json(json!({"error": "Invalid or expired token"})),
                    )
                        .into_response());
                }
            }
        }

        Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Missing authorization header"})),
        )
            .into_response())
    }
}
