use anyhow::{Context, Result};
use awtrix3::{dto::*, topics::*};
use energy_monitor_lib::{
    opendtu::topics::{OPEN_DTU_AC_POWER_TOPIC, OPEN_DTU_AC_YIELD_DAY_TOPIC},
    pulse::topics::PULSE_CONSUMPTION_TOPIC,
    tibber::{
        dto::{PriceInformation, PriceLevel},
        topics::TIBBER_PRICE_INFORMATION_TOPIC,
    },
};
use log::{debug, error, info};
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Publish, QoS};
use std::time::Duration;
use syslog::{Facility, Formatter3164};
use tokio::{sync::mpsc, time::sleep};
mod awtrix3;

const MQTT_CLIENT_NAME: &str = "matrix-display-updater";
const MQTT_BROKER_ADDRESS: &str = "rpiserver";
const MQTT_BROKER_PORT: u16 = 1883;

#[tokio::main]
async fn main() -> Result<()> {
    let formatter = Formatter3164 {
        facility: Facility::LOG_DAEMON,
        hostname: None,
        process: "emdisplayd".into(),
        pid: 0,
    };

    env_logger::init();
    syslog::unix(formatter).expect("Failed to initialize syslog");

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

    let (mut tx, mut rx) = mpsc::channel(10);

    tokio::spawn(async move {
        if let Err(e) = process_event_loop(&mut eventloop, &mut tx).await {
            error!("Error processing event loop = {:?}", e);
            std::process::exit(1);
        }
    });

    // Clone the client to use in the publishing task
    let publish_client = client.clone();
    tokio::spawn(async move {
        if let Err(e) = handle_messages(publish_client, &mut rx).await {
            error!("Error handling messages = {:?}", e);
            std::process::exit(1);
        }
    });

    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

async fn process_event_loop(
    eventloop: &mut EventLoop,
    tx: &mut mpsc::Sender<Event>,
) -> Result<(), anyhow::Error> {
    loop {
        match eventloop
            .poll()
            .await
            .context("Error polling notfication from mqtt event loop")
        {
            Ok(notification) => {
                tx.send(notification)
                    .await
                    .context("Error sending message on channel")?;
            }
            Err(e) => {
                error!("Error polling event loop = {:?}", e);
                return Err(e);
            }
        }
    }
}

async fn handle_messages(
    client: AsyncClient,
    rx: &mut mpsc::Receiver<Event>,
) -> Result<(), anyhow::Error> {
    while let Some(notification) = rx.recv().await {
        debug!("Received = {:?}", notification);
        if let rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish)) = notification {
            match publish.topic.as_str() {
                topic if topic == OPEN_DTU_AC_YIELD_DAY_TOPIC.name() => {
                    publish_yield_day(&client, &publish)
                        .await
                        .context("Error publishing yield day")?;
                }
                topic if topic == OPEN_DTU_AC_POWER_TOPIC.name() => {
                    publish_current_production(&client, &publish)
                        .await
                        .context("Error publishing current production")?;
                }
                topic if topic == PULSE_CONSUMPTION_TOPIC.name() => {
                    publish_current_consumption(&client, &publish)
                        .await
                        .context("Error publishing current consumption")?;
                }
                topic if topic == TIBBER_PRICE_INFORMATION_TOPIC.name() => {
                    publish_current_price(&client, &publish)
                        .await
                        .context("Error publishing current price")?;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

async fn publish_yield_day(client: &AsyncClient, publish: &Publish) -> Result<(), anyhow::Error> {
    let yield_day = OPEN_DTU_AC_YIELD_DAY_TOPIC.decode(&publish.payload)?;

    info!("yield today: {}W", yield_day);
    client
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
    Ok(())
}

async fn publish_current_production(
    client: &AsyncClient,
    publish: &Publish,
) -> Result<(), anyhow::Error> {
    let current_power = OPEN_DTU_AC_POWER_TOPIC.decode(&publish.payload)?;

    info!("Current production: {:0.0}W", current_power);
    client
        .publish(
            MATRIX_DISPLAY_APP_CURRENT_PRODUCTION_TOPIC.name(),
            QoS::AtMostOnce,
            false,
            MATRIX_DISPLAY_APP_CURRENT_PRODUCTION_TOPIC.encode(&CustomApplication {
                text: format!("{:0.0}", current_power),
                duration: Some(5),
                icon: Some(37515.to_string()),
                ..Default::default()
            }),
        )
        .await?;
    Ok(())
}

async fn publish_current_consumption(
    client: &AsyncClient,
    publish: &Publish,
) -> Result<(), anyhow::Error> {
    let consumption = PULSE_CONSUMPTION_TOPIC
        .decode(&publish.payload)?
        .consumption;

    info!("Current consumption: {}W", consumption);
    client
        .publish(
            MATRIX_DISPLAY_APP_CURRENT_CONSUMPTION_TOPIC.name(),
            QoS::AtMostOnce,
            false,
            MATRIX_DISPLAY_APP_CURRENT_CONSUMPTION_TOPIC.encode(&CustomApplication {
                text: format!("{:0.1}", consumption as f32 / 1000.0),
                duration: Some(5),
                icon: Some(55888.to_string()),
                life_time: Some(10), // if no update within 10 seconds remove
                ..Default::default()
            }),
        )
        .await?;
    Ok(())
}

async fn publish_current_price(
    client: &AsyncClient,
    publish: &Publish,
) -> Result<(), anyhow::Error> {
    let price_information: PriceInformation =
        TIBBER_PRICE_INFORMATION_TOPIC.decode(&publish.payload)?;

    info!("Current price: {} Euro", price_information.total);
    client
        .publish(
            MATRIX_DISPLAY_APP_CURRENT_PRICE_TOPIC.name(),
            QoS::AtMostOnce,
            false,
            MATRIX_DISPLAY_APP_CURRENT_PRICE_TOPIC.encode(&CustomApplication {
                text: format!("{:0.2}", price_information.total),
                duration: Some(2),
                icon: Some(54231.to_string()),
                color: Some(color_from_price_level(price_information.level).to_string()),
                life_time: Some(60 * 62), // 1 hour and 2 minutes to make sure the price is updated
                ..Default::default()
            }),
        )
        .await?;
    Ok(())
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
