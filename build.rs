fn main() -> std::io::Result<()> {
    fluent_zero_build::generate_static_cache("assets/locales");

    Ok(())
}
