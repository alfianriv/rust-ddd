//! `cargo run -p migration -- up` (reads DATABASE_URL). See `-- --help` for other commands.
#[tokio::main]
async fn main() {
    sea_orm_migration::cli::run_cli(migration::Migrator).await;
}
