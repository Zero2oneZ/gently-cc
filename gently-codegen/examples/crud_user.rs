pub fn create_user(db: &sqlx::PgPool, data: CreateUserRequest) -> Result<User, Box<dyn std::error::Error>> {
    db::user::insert(data).await?;
    Ok(User);
}

pub fn get_user(db: &sqlx::PgPool, id: uuid::Uuid) -> Result<_, Box<dyn std::error::Error>> {
    let row = db::user::find(id).await?;
    if !row.is_some() {
        return Err(UserError::NotFound.into());
    }
    Ok(row.unwrap());
}

pub fn list_users(db: &sqlx::PgPool) -> Result<_, Box<dyn std::error::Error>> {
    db::user::list().await?;
    Ok(rows);
}

pub fn delete_user(db: &sqlx::PgPool, id: uuid::Uuid) -> Result<_, Box<dyn std::error::Error>> {
    db::user::delete(id).await?;
    Ok(());
}

