pub fn migrate() -> anyhow::Result<()> {
    crate::dbopt::run_migrations()?;
    Ok(())
}
