#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Agent {
    pub id: uuid::Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub enum AgentError {
    NotFound,
    Unauthorized,
    Conflict,
    Internal(String),
}

pub fn create_agent(db: &sqlx::PgPool, data: CreateAgentRequest) -> Result<Agent, Box<dyn std::error::Error>> {
    db::agent::insert(data).await?;
    Ok(Agent);
}

pub fn get_agent(db: &sqlx::PgPool, id: uuid::Uuid) -> Result<_, Box<dyn std::error::Error>> {
    let row = db::agent::find(id).await?;
    if !row.is_some() {
        return Err(AgentError::NotFound.into());
    }
    Ok(row.unwrap());
}

pub fn list_agents(db: &sqlx::PgPool) -> Result<_, Box<dyn std::error::Error>> {
    db::agent::list().await?;
    Ok(rows);
}

pub fn delete_agent(db: &sqlx::PgPool, id: uuid::Uuid) -> Result<_, Box<dyn std::error::Error>> {
    db::agent::delete(id).await?;
    Ok(());
}

pub mod agent_routes {
    pub fn get_agent(State(db): State<sqlx::PgPool>, Path(id): Path<uuid::Uuid>) -> Json(result.unwrap()).into_response() {
        let result = self::super::get_agent(&db, id).await?;
        if result.is_err() {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "not found"}))).into_response();
        }
        return Json(result.unwrap()).into_response();
    }

}

