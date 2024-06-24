pub mod dto;
pub mod topics;

#[cfg(test)]
mod test {
    use crate::awtrix3::dto::CustomApplication;

    #[test]
    fn test_default_custom_application() {
        let custom_application = CustomApplication::default();

        assert_eq!(custom_application.icon, None);
    }
}
