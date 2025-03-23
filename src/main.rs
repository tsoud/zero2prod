use std::net::TcpListener;

use secrecy::ExposeSecret;
use sqlx::PgPool;

use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_tracing_subscriber, init_tracing_subscriber};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app_tracing_subscriber =
        get_tracing_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_tracing_subscriber(app_tracing_subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool =
        PgPool::connect_lazy(&configuration.database.connection_string().expose_secret())
            // .await
            .expect("Couldn't connect to Postgres.");
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool)?.await?;

    Ok(())
}
