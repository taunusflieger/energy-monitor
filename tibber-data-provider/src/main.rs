#![feature(inline_const_pat)]
use energy_monitor_lib::{
    pulse::{dto::Consumption, topics::PULSE_CONSUMPTION_TOPIC},
    tibber::{dto, topics::TIBBER_PRICE_INFORMATION_TOPIC},
};
use futures_util::future::FutureExt;
use log::{error, info};
use reqwest::Client;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use sml_rs::parser::{
    common::Value,
    complete::{parse, MessageBody},
};
use sml_rs::transport::decode;
use std::error::Error;
use tibber_loader::{config::Config, gql::queries::PriceLevel};
use tokio::{task, time::Duration};
use tokio_cron_scheduler::{Job, JobScheduler};

const TIBBER_API_URL: &str = "https://api.tibber.com/v1-beta/gql";
const PULSE_BRIDGE_URL: &str = "http://192.168.100.60/data.json?node_id=1";
const MQTT_CLIENT_NAME: &str = "tibber_bridge_data_provider";
const MQTT_BROKER_ADDRESS: &str = "iotstore";
const MQTT_BROKER_PORT: u16 = 1883;
const PULSE_BRIDGE_USERNAME: &str = "admin";

#[tokio::main()]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    println!(
        "Starting Tibber Data Provider (emtibberd) v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Check if Tibber API key env variable is set
    // In case of failsure, fail early
    if Config::new(TIBBER_API_URL).is_err() {
        error!("Failed to load tibber config. Check if TIBBER_API_TOKEN is set");
        std::process::exit(1);
    }

    let sched = JobScheduler::new().await?;
    let mut handles = Vec::new();

    let mut mqttoptions = MqttOptions::new(MQTT_CLIENT_NAME, MQTT_BROKER_ADDRESS, MQTT_BROKER_PORT);
    mqttoptions.set_keep_alive(Duration::from_secs(10));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 5);

    let pulse_bridge_client = client.clone();
    let tibber_client = client.clone();

    // This job runs every 10 seconds and retrives the current power consumption
    // from the Pulse Bridge
    let mut pulse_bridge_job = Job::new_async("1/10 * * * * *", move |_, _| {
        let publish_client_tibber_data = pulse_bridge_client.clone();

        Box::pin(
            async move {
                println!("Executing data fetch from Pulse Bridge job");

                let pulse_bridge_pwd = if let Ok(pwd) = std::env::var("PULSE_BRIDGE_PASSWORD") {
                    if pwd.is_empty() {
                        error!("PULSE_BRIDGE_PASSWORD is empty");
                        std::process::exit(1);
                    } else {
                        pwd
                    }
                } else {
                    error!("PULSE_BRIDGE_PASSWORD is not set");
                    std::process::exit(1);
                };
                let bridge_client = Client::builder().timeout(Duration::from_secs(15)).build()?;
                let mut resp = bridge_client
                    .get(PULSE_BRIDGE_URL)
                    .basic_auth(PULSE_BRIDGE_USERNAME, Some(pulse_bridge_pwd))
                    .send()
                    .await?;

                let mut buffer: bytes::BytesMut = bytes::BytesMut::new();

                while let Some(chunk) = resp.chunk().await? {
                    buffer.extend_from_slice(chunk.to_vec().as_slice());
                }

                // We have only 1 message
                let message = decode(buffer.to_vec())[0].clone()?;
                let result = parse(&message)?;

                // 2nd message is GetListResponse with the values we are interested
                let len = result.messages.len();
                if len < 2 {
                    error!("Expected 2 messages, got {len}");
                    return Result::<_, anyhow::Error>::Ok(());
                }
                let get_list_response = result.messages[1].message_body.clone();

                match get_list_response {
                    MessageBody::GetListResponse(le) => {
                        for entry in le.val_list {
                            // Current power consumption
                            if entry.obj_name == [1, 0, 16, 7, 0, 255] {
                                let current_power = match entry.value {
                                    Value::I32(v) => v,
                                    _ => 0,
                                };
                                info!("Power = {current_power}W");

                                //let payload = PULSE_CONSUMPTION_TOPIC.encode(&consumption)?;
                                // Publish the message
                                publish_client_tibber_data
                                    .publish(
                                        PULSE_CONSUMPTION_TOPIC.name(),
                                        QoS::AtMostOnce,
                                        false,
                                        PULSE_CONSUMPTION_TOPIC.encode(&Consumption {
                                            consumption: current_power,
                                        }),
                                    )
                                    .await?;
                            }
                        }
                    }
                    _ => error!("Expected ListEntry"),
                };
                Result::<_, anyhow::Error>::Ok(())
            }
            .map(|res| {
                if let Err(err) = res {
                    error!(
                        "Failed to retive current power consumption from Pulse Bridge: {}",
                        err
                    );
                }
            }),
        )
    })?;

    pulse_bridge_job
        .on_stop_notification_add(
            &sched,
            Box::new(|job_id, notification_id, type_of_notification| {
                Box::pin(async move {
                    info!(
                        "Job {:?} was completed, notification {:?} ran ({:?})",
                        job_id, notification_id, type_of_notification
                    );
                })
            }),
        )
        .await?;

    let mut tibber_job = Job::new_async("0 2 * * * *", move |_, _| {
        let publish_client_tibber_data = tibber_client.clone();
        Box::pin(
            async move {
                println!("Executing Tibber job");
                let config = Config::new(TIBBER_API_URL)?;

                let session = tibber_loader::Session::new(config).await?;

                let current_price = session.get_current_price().await?;
                if let Some(price) = current_price {
                    info!("Current price: {:?} Euro", price.total);
                    info!("Price Level: {:?}", price.level);

                    let price_information = dto::PriceInformation {
                        total: price.total as f32,
                        level: match price.level {
                            PriceLevel::Cheap => dto::PriceLevel::Cheap,
                            PriceLevel::Expensive => dto::PriceLevel::Expensive,
                            PriceLevel::Normal => dto::PriceLevel::Normal,
                            PriceLevel::VeryCheap => dto::PriceLevel::VeryCheap,
                            PriceLevel::VeryExpensive => dto::PriceLevel::VeryExpensive,
                            PriceLevel::None => dto::PriceLevel::None,
                            _ => dto::PriceLevel::None,
                        },
                    };

                    // Publish the message
                    publish_client_tibber_data
                        .publish(
                            TIBBER_PRICE_INFORMATION_TOPIC.name(),
                            QoS::AtMostOnce,
                            false,
                            TIBBER_PRICE_INFORMATION_TOPIC.encode(&price_information),
                        )
                        .await?;
                } else {
                    error!("No current price");
                }
                Result::<_, anyhow::Error>::Ok(())
            }
            .map(|res| {
                if let Err(err) = res {
                    error!("Failed to retive Tibber current price info: {:?}", err);
                }
            }),
        )
    })?;

    tibber_job
        .on_stop_notification_add(
            &sched,
            Box::new(|job_id, notification_id, type_of_notification| {
                Box::pin(async move {
                    info!(
                        "Job {:?} was completed, notification {:?} ran ({:?})",
                        job_id, notification_id, type_of_notification
                    );
                })
            }),
        )
        .await?;

    sched.add(pulse_bridge_job).await?;
    sched.add(tibber_job).await?;
    sched.start().await?;

    handles.push(task::spawn(async move {
        loop {
            if (eventloop.poll().await).is_err() {
                // In case of an error stop event loop and terminate task
                // this will result in aborting the program
                break;
            }
        }
    }));

    // In case any of the tasks panic abort the program
    for handle in handles {
        if let Err(e) = handle.await {
            error!("Task panicked: {:?}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}
