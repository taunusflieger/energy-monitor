pub mod pulse;
pub mod tibber;
pub mod topic;

#[cfg(test)]
mod test {
    use crate::tibber::dto;
    use crate::topic::Topic;
    #[test]
    fn test_encode_decode() {
        let price_info = dto::PriceInformation {
            total: 100.0,
            level: dto::PriceLevel::Cheap,
        };

        let topic = Topic::new("Tibber/price_information");
        let encoded = topic.encode(&price_info);
        let decoded = topic.decode(&encoded).unwrap();

        assert_eq!(price_info, decoded);
    }
}
