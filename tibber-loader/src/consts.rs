pub const fn get_user_agent() -> &'static str {
    concat!("CLI ", env!("CARGO_PKG_VERSION"))
}
