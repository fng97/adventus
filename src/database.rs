use sqlx::PgPool;
use std::env;

pub async fn migrate(db: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(db)
        .await
        .expect("Failed to migrate the database");
}

pub fn local_database_url(db: &str) -> String {
    format!("{}/{}", url_without_db(), db)
}

fn url_without_db() -> String {
    format!(
        "postgres://postgres:password@{}:5432",
        env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string())
    )
}

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use sqlx::{Connection, Executor, PgConnection, PgPool};
    use uuid::Uuid;

    pub async fn get_test_database() -> PgPool {
        let database_name = Uuid::new_v4().to_string(); // unique db for each test

        // Create database
        let _ = PgConnection::connect(&url_without_db())
            .await
            .expect("Failed to connect to Postgres")
            .execute(format!(r#"CREATE DATABASE "{}";"#, database_name).as_str())
            .await
            .expect("Failed to create database.");

        let connection_string = local_database_url(&database_name);

        // Migrate database
        let connection_pool = PgPool::connect(&connection_string)
            .await
            .expect("Failed to connect to Postgres.");

        migrate(&connection_pool).await;

        connection_pool
    }
}
