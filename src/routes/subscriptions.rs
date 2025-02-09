use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Info {
    email: String,
    name: String,
}

pub async fn subscribe(form: web::Form<Info>, db_pool: web::Data<PgPool>) -> HttpResponse {
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Adding a new subscriber.",
        %request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    );
    let _request_span_guard = request_span.enter();

    let query_span = tracing::info_span!("Adding new subscriber details to database...");

    if let Err(e) = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now(),
    )
    .execute(db_pool.get_ref())
    .instrument(query_span)
    .await
    {
        tracing::error!(
            "request_id {} - Couldn't add subscriber: {:?}",
            request_id,
            e
        );
        return HttpResponse::InternalServerError().finish();
    }

    tracing::info!("request_id {} - New subscriber details saved.", request_id);
    HttpResponse::Ok().finish()
}
