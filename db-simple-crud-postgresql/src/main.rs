mod api;
mod db;
mod load_test;

use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "clinical-crud")]
#[command(about = "CRUD application for clinical trial PostgreSQL tables", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the REST API server
    Server {
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
        /// Port to listen on
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
    /// Run functional CRUD tests
    Test,
    /// Run concurrent load testing
    LoadTest {
        /// Number of concurrent clients
        #[arg(long, default_value_t = 50)]
        clients: usize,
        /// Total number of operations across all clients
        #[arg(long, default_value_t = 1000)]
        operations: usize,
        /// Table to test against
        #[arg(long, default_value = "dm")]
        table: String,
        /// Read ratio (0.0-1.0)
        #[arg(long, default_value_t = 0.6)]
        read_ratio: f64,
        /// Write ratio (0.0-1.0)
        #[arg(long, default_value_t = 0.2)]
        write_ratio: f64,
        /// Update ratio (0.0-1.0)
        #[arg(long, default_value_t = 0.15)]
        update_ratio: f64,
        /// Delete ratio (0.0-1.0)
        #[arg(long, default_value_t = 0.05)]
        delete_ratio: f64,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let pool = db::create_pool();

    // Verify DB connection
    info!("Connecting to PostgreSQL...");
    db::check_health(&pool).await?;
    info!("Database connection OK");

    match cli.command {
        Commands::Server { host, port } => {
            info!("Starting REST API server on {}:{}", host, port);
            info!(
                "Swagger UI available at http://{}:{}/swagger-ui",
                host, port
            );

            let state = api::AppState { pool };
            let app = api::build_router(state);

            let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;
            axum::serve(listener, app).await?;
        }
        Commands::Test => {
            info!("Running functional tests...");
            load_test::run_functional_tests(&pool).await?;
        }
        Commands::LoadTest {
            clients,
            operations,
            table,
            read_ratio,
            write_ratio,
            update_ratio,
            delete_ratio,
        } => {
            let config = load_test::LoadTestConfig {
                num_clients: clients,
                num_operations: operations,
                read_ratio,
                write_ratio,
                update_ratio,
                delete_ratio,
                table,
            };
            load_test::run_load_test(&pool, config).await?;
        }
    }

    Ok(())
}
