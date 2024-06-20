pub const fn get_user_agent() -> &'static str {
    concat!("energy monitor ", env!("CARGO_PKG_VERSION"))
}
