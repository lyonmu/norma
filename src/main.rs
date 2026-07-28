fn main() -> anyhow::Result<()> {
    let runtime = norma::runtime::bootstrap()?;
    norma::ui::window::run(runtime.app_state, runtime.updates);
    Ok(())
}
