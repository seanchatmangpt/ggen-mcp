use clap::Parser;
use spreadsheet_mcp::{
    CliArgs, LoggingConfig, ServerConfig, init_logging, run_server, shutdown_telemetry,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize structured logging with OpenTelemetry integration
    let logging_config = LoggingConfig::from_env();
    let _guard = init_logging(logging_config)?;

    let cli = CliArgs::parse();
    let config = ServerConfig::from_args(cli)?;

    // Validate configuration before server startup (fail-fast)
    config.validate()?;

    // Run server and handle graceful shutdown
    let result = run_server(config).await;

    // Ensure traces are flushed before exit
    shutdown_telemetry();

    result
}
