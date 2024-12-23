use sqlx::PgPool;

pub enum Metrics {
    Rolls,
    Introductions,
}

impl AsRef<str> for Metrics {
    fn as_ref(&self) -> &str {
        match self {
            Metrics::Rolls => "rolls",
            Metrics::Introductions => "introductions",
        }
    }
}

pub async fn increment(pool: PgPool, metric: Metrics) -> Result<(), sqlx::Error> {
    let _ = sqlx::query!(
        r#"
        INSERT INTO metrics (metric_name, count)
        VALUES ($1, 1)
        ON CONFLICT (metric_name) DO UPDATE
        SET count = metrics.count + 1
        "#,
        metric.as_ref(),
    )
    .execute(&pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database_setup::test_utils::get_test_database;

    #[tokio::test]
    async fn test_increment() {
        // arrange
        let pool = get_test_database().await;

        // act
        increment(pool.clone(), Metrics::Rolls).await.unwrap();
        increment(pool.clone(), Metrics::Rolls).await.unwrap();
        increment(pool.clone(), Metrics::Introductions)
            .await
            .unwrap();

        let rolls = sqlx::query!("SELECT count FROM metrics WHERE metric_name = 'rolls'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let introductions =
            sqlx::query!("SELECT count FROM metrics WHERE metric_name = 'introductions'")
                .fetch_one(&pool)
                .await
                .unwrap();

        // assert
        assert_eq!(rolls.count, 2);
        assert_eq!(introductions.count, 1);
    }
}
