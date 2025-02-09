use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Info {
    email: String,
    name: String,
}

#[instrument(
    name = "Adding a new subscriber",
    skip(form, db_pool),
    fields(
        request_id = %Uuid::new_v4(),
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(form: web::Form<Info>, db_pool: web::Data<PgPool>) -> HttpResponse {
    if let Ok(()) = insert_subscriber(db_pool.get_ref(), &form).await {
        return HttpResponse::Ok().finish();
    }
    HttpResponse::InternalServerError().finish()
}

#[instrument(name = "Saving new subscriber details to database", skip(pool, form))]
pub async fn insert_subscriber(pool: &PgPool, form: &Info) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now(),
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
