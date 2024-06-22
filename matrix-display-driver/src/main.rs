#![feature(inline_const_pat)]
use anyhow::Result;
use energy_monitor_lib::{
    pulse::topics::PULSE_CONSUMPTION_TOPIC,
    tibber::{
        dto::{PriceInformation, PriceLevel},
        topics::TIBBER_PRICE_INFORMATION_TOPIC,
    },
};
use log::{debug, info};
use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde_json::json;
use std::time::Duration;

const OPEN_DTU_AC_POWER_TOPIC: &str = "OpenDTU/ac/power";
const OPEN_DTU_AC_YIELD_DAY_TOPIC: &str = "OpenDTU/ac/yieldday";
const MQTT_CLIENT_NAME: &str = "matrix-display-updater";
const MQTT_BROKER_ADDRESS: &str = "iotstore";
const MQTT_BROKER_PORT: u16 = 1883;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!(
        "Starting Matrix Display Driver (emdisplayd) {}",
        env!("CARGO_PKG_VERSION")
    );

    let mut mqttoptions = MqttOptions::new(MQTT_CLIENT_NAME, MQTT_BROKER_ADDRESS, MQTT_BROKER_PORT);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    for topic in [
        OPEN_DTU_AC_POWER_TOPIC,
        OPEN_DTU_AC_YIELD_DAY_TOPIC,
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
                OPEN_DTU_AC_YIELD_DAY_TOPIC => {
                    let payload = String::from_utf8_lossy(&publish.payload);

                    info!("yield today: {}W", payload);

                    let publish_payload = json!({
                        "text": payload,
                        "duration": 5,
                        "icon": 52455
                    });

                    publish_client
                        .publish(
                            "matrixdisplay/custom/yieldday",
                            QoS::AtMostOnce,
                            false,
                            publish_payload.to_string(),
                        )
                        .await?;
                }
                OPEN_DTU_AC_POWER_TOPIC => {
                    let payload = String::from_utf8_lossy(&publish.payload);

                    info!("Current production: {}W", payload);

                    let publish_payload = json!({
                        "text": payload,
                        "duration": 5,
                        "icon": 37515
                    });

                    publish_client
                        .publish(
                            "matrixdisplay/custom/power",
                            QoS::AtMostOnce,
                            false,
                            publish_payload.to_string(),
                        )
                        .await?;
                }
                const { PULSE_CONSUMPTION_TOPIC.name() } => {
                    let consumption = PULSE_CONSUMPTION_TOPIC
                        .decode(&publish.payload)?
                        .consumption;

                    info!("Current consumption: {}W", consumption);

                    let publish_payload = json!({
                        "text": format!("{:0.1} kW", consumption as f32 / 1000.0),
                        "duration": 2
                    });

                    publish_client
                        .publish(
                            "matrixdisplay/custom/consumption",
                            QoS::AtMostOnce,
                            false,
                            publish_payload.to_string(),
                        )
                        .await?;
                }
                const { TIBBER_PRICE_INFORMATION_TOPIC.name() } => {
                    let price_information: PriceInformation =
                        TIBBER_PRICE_INFORMATION_TOPIC.decode(&publish.payload)?;

                    info!("Current price: {} Euro", price_information.total);

                    let color = match price_information.level {
                        PriceLevel::Cheap => "#66FF00",
                        PriceLevel::Expensive => "#FF0800",
                        PriceLevel::Normal => "#ED872D",
                        PriceLevel::VeryCheap => "#66FF00",
                        PriceLevel::VeryExpensive => "#FF0800",
                        PriceLevel::None => "#FF00FF",
                    };

                    let publish_payload = json!({
                        "text": format!("{:0.2}", price_information.total),
                        "duration": 2,
                        "icon": 23051,
                        "color": color
                    });

                    publish_client
                        .publish(
                            "matrixdisplay/custom/tibberprice",
                            QoS::AtMostOnce,
                            false,
                            publish_payload.to_string(),
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
