fn main() -> anyhow::Result<()> {
    let runtime = norma::runtime::bootstrap()?;
    norma::ui::shell::run(runtime.app_state, runtime.updates);
    Ok(())
}
