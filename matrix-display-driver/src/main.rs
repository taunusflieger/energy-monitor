#![feature(inline_const_pat)]
use anyhow::Result;
use awtrix3::{dto::*, topics::*};
use energy_monitor_lib::{
    opendtu::topics::{OPEN_DTU_AC_POWER_TOPIC, OPEN_DTU_AC_YIELD_DAY_TOPIC},
    pulse::topics::PULSE_CONSUMPTION_TOPIC,
    tibber::{
        dto::{PriceInformation, PriceLevel},
        topics::TIBBER_PRICE_INFORMATION_TOPIC,
    },
};
use log::{debug, info};
use rumqttc::{AsyncClient, MqttOptions, QoS};
use std::time::Duration;
mod awtrix3;

const MQTT_CLIENT_NAME: &str = "matrix-display-updater";
const MQTT_BROKER_ADDRESS: &str = "iotstore";
const MQTT_BROKER_PORT: u16 = 1883;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!(
        "Starting Matrix Display Driver (emdisplayd) v{}",
        env!("CARGO_PKG_VERSION")
    );

    let mut mqttoptions = MqttOptions::new(MQTT_CLIENT_NAME, MQTT_BROKER_ADDRESS, MQTT_BROKER_PORT);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    for topic in [
        OPEN_DTU_AC_POWER_TOPIC.name(),
        OPEN_DTU_AC_YIELD_DAY_TOPIC.name(),
        PULSE_CONSUMPTION_TOPIC.name(),
        TIBBER_PRICE_INFORMATION_TOPIC.name(),
    ] {
        client.subscribe(topic, QoS::AtMostOnce).await?;
    }

    // Clone the client to use in the publishing task
    let publish_client = client.clone();

    while let Ok(notification) = eventloop.poll().await {
        debug!("Received = {:?}", notification);
        if let rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish)) = notification {
            match publish.topic.as_str() {
                const { OPEN_DTU_AC_YIELD_DAY_TOPIC.name() } => {
                    let yield_day = OPEN_DTU_AC_YIELD_DAY_TOPIC.decode(&publish.payload)?;

                    info!("yield today: {}W", yield_day);
                    publish_client
                        .publish(
                            MATRIX_DISPLAY_APP_YIELD_DAY_TOPIC.name(),
                            QoS::AtMostOnce,
                            false,
                            MATRIX_DISPLAY_APP_YIELD_DAY_TOPIC.encode(&CustomApplication {
                                text: yield_day.to_string(),
                                duration: Some(5),
                                icon: Some(52455.to_string()),
                                ..Default::default()
                            }),
                        )
                        .await?;
                }
                const { OPEN_DTU_AC_POWER_TOPIC.name() } => {
                    let current_power = OPEN_DTU_AC_POWER_TOPIC.decode(&publish.payload)?;

                    info!("Current production: {:0.0}W", current_power);
                    publish_client
                        .publish(
                            MATRIX_DISPLAY_APP_CURRENT_PRODUCTION_TOPIC.name(),
                            QoS::AtMostOnce,
                            false,
                            MATRIX_DISPLAY_APP_CURRENT_PRODUCTION_TOPIC.encode(
                                &CustomApplication {
                                    text: format!("{:0.0}", current_power),
                                    duration: Some(5),
                                    icon: Some(37515.to_string()),
                                    ..Default::default()
                                },
                            ),
                        )
                        .await?;
                }
                const { PULSE_CONSUMPTION_TOPIC.name() } => {
                    let consumption = PULSE_CONSUMPTION_TOPIC
                        .decode(&publish.payload)?
                        .consumption;

                    info!("Current consumption: {}W", consumption);
                    publish_client
                        .publish(
                            MATRIX_DISPLAY_APP_CURRENT_CONSUMPTION_TOPIC.name(),
                            QoS::AtMostOnce,
                            false,
                            MATRIX_DISPLAY_APP_CURRENT_CONSUMPTION_TOPIC.encode(
                                &CustomApplication {
                                    text: format!("{:0.1}", consumption as f32 / 1000.0),
                                    duration: Some(5),
                                    icon: Some(55888.to_string()),
                                    life_time: Some(10), // if no update within 10 seconds remove
                                    ..Default::default()
                                },
                            ),
                        )
                        .await?;
                }
                const { TIBBER_PRICE_INFORMATION_TOPIC.name() } => {
                    let price_information: PriceInformation =
                        TIBBER_PRICE_INFORMATION_TOPIC.decode(&publish.payload)?;

                    info!("Current price: {} Euro", price_information.total);
                    publish_client
                        .publish(
                            MATRIX_DISPLAY_APP_CURRENT_PRICE_TOPIC.name(),
                            QoS::AtMostOnce,
                            false,
                            MATRIX_DISPLAY_APP_CURRENT_PRICE_TOPIC.encode(&CustomApplication {
                                text: format!("{:0.2}", price_information.total),
                                duration: Some(2),
                                icon: Some(54231.to_string()),
                                color: Some(
                                    color_from_price_level(price_information.level).to_string(),
                                ),
                                life_time: Some(60 * 62), // 1 hour and 2 minutes to make sure the price is updated
                                ..Default::default()
                            }),
                        )
                        .await?;
                }
                _ => {}
            }
        }
    }
    // If the process finished it means that the connection to the broker was lost
    std::process::exit(1);
}

fn color_from_price_level(level: PriceLevel) -> &'static str {
    match level {
        PriceLevel::Cheap => "#66FF00",
        PriceLevel::Expensive => "#FF0800",
        PriceLevel::Normal => "#ED872D",
        PriceLevel::VeryCheap => "#66FF00",
        PriceLevel::VeryExpensive => "#FF0800",
        PriceLevel::None => "#FF00FF",
    }
}
