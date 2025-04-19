use std::net::TcpListener;
use std::sync::LazyLock;

use secrecy::SecretString;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;

use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::startup::run;
use zero2prod::telemetry::{get_tracing_subscriber, init_tracing_subscriber};

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

static APP_TRACER: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "info".to_string();
    let app_subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let app_tracer =
            get_tracing_subscriber(app_subscriber_name, default_filter_level, std::io::stdout);
        init_tracing_subscriber(app_tracer);
    } else {
        let app_tracer =
            get_tracing_subscriber(app_subscriber_name, default_filter_level, std::io::sink);
        init_tracing_subscriber(app_tracer);
    };
});

async fn spawn_app() -> TestApp {
    LazyLock::force(&APP_TRACER);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);
    let mut configuration = get_configuration().expect("Failed to get configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let db_pool = configure_database(&configuration.database).await;
    let server = run(listener, db_pool.clone()).expect("Failed to bind address");

    let _ = tokio::spawn(server);
    TestApp { address, db_pool }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create default Postgres db to use as a maintenance db
    let maintenance_settings = DatabaseSettings {
        database_name: "postgres".to_string(),
        username: "postgres".to_string(),
        // password: "password".to_string(),
        password: SecretString::from("password"),
        ..config.clone()
    };
    // Connect to maintenance db and use it to create a new db with the config settings
    let mut connection = PgConnection::connect_with(&maintenance_settings.connect_options())
        .await
        .expect("Couldn't connect to Postgres.");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    // Create a connection pool and migrate the database
    let connection_pool = PgPool::connect_with(config.connect_options())
        .await
        .expect("Couldn't connect to Postgres.");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate database.");

    connection_pool
}

#[tokio::test]
async fn health_check_works() {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health_check", &test_app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(&format!("{}/subscriptions", &test_app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subscribe_returns_a_400_for_missing_data() {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &test_app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message,
        );
    }
}
