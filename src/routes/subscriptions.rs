use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Info {
    email: String,
    name: String,
}

pub async fn subscribe(form: web::Form<Info>, db_pool: web::Data<PgPool>) -> HttpResponse {
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
    .await
    {
        println!("Couldn't add subscriber: {e}");
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}
