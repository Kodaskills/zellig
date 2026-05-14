use zellig::cli::run;
use zellig::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    run().await
}
