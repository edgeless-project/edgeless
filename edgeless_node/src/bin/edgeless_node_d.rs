fn main() -> anyhow::Result<()> {
    let async_runtime = tokio::runtime::Builder::new_current_thread().build()?;
    Ok(())
}