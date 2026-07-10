#![allow(dead_code)]

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;

pub async fn get_tags_for_clip(pool: &PgPool, clip_id: Uuid) -> Result<Vec<String>, AppError> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT tag FROM clip_tags WHERE clip_id = $1 ORDER BY tag")
            .bind(clip_id)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

pub async fn add_tag(pool: &PgPool, clip_id: Uuid, tag: &str) -> Result<(), AppError> {
    sqlx::query("INSERT INTO clip_tags (clip_id, tag) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(clip_id)
        .bind(tag)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

pub async fn remove_tag(pool: &PgPool, clip_id: Uuid, tag: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM clip_tags WHERE clip_id = $1 AND tag = $2")
        .bind(clip_id)
        .bind(tag)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

pub async fn set_tags(pool: &PgPool, clip_id: Uuid, tags: &[String]) -> Result<(), AppError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    sqlx::query("DELETE FROM clip_tags WHERE clip_id = $1")
        .bind(clip_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    for tag in tags {
        if !tag.trim().is_empty() {
            sqlx::query("INSERT INTO clip_tags (clip_id, tag) VALUES ($1, $2)")
                .bind(clip_id)
                .bind(tag.trim())
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
    }

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}
