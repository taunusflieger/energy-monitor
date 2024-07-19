use anyhow::{anyhow, Context};
use energy_monitor_lib::{
    pulse::{dto::Consumption, topics::PULSE_CONSUMPTION_TOPIC},
    tibber::{dto, topics::TIBBER_PRICE_INFORMATION_TOPIC},
};
use log::{debug, error, info};
use reqwest::Client;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use sml_rs::parser::{
    common::Value,
    complete::{parse, MessageBody},
};
use sml_rs::transport::decode;
use std::error::Error;
use syslog::{Facility, Formatter3164};
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
    let formatter = Formatter3164 {
        facility: Facility::LOG_DAEMON,
        hostname: None,
        process: "emtibberd".into(),
        pid: 0,
    };

    env_logger::init();
    syslog::unix(formatter).expect("Failed to initialize syslog");

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

    // When first stating the application, we want to fetch the current price
    if let Err(e) = get_tibber_data_and_publish(&client.clone()).await {
        error!("Failed to retive Tibber current price info: {:?}", e);
    } else {
        info!("Successfully retrieved and published Tibber current price info");
    }

    // This job runs every 10 seconds and retrieves the current power consumption
    // from the Pulse Bridge
    let mut pulse_bridge_job = Job::new_async("1/10 * * * * *", move |_, _| {
        let publish_client_tibber_data = pulse_bridge_client.clone();

        Box::pin(async move {
            if let Err(e) = get_pulse_bridge_data_and_publish(&publish_client_tibber_data).await {
                error!("Failed Tibber API job: {:?}", e);
            }
        })
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
        Box::pin(async move {
            if let Err(e) = get_tibber_data_and_publish(&publish_client_tibber_data).await {
                error!("Failed Pulse Bridge job: {:?}", e);
            }
        })
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
            if let Err(e) = eventloop.poll().await {
                // In case of an error stop event loop and terminate task
                // this will result in aborting the program
                error!("Error MQTT Event loop returned: {:?}", e);
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

async fn get_tibber_data_and_publish(
    publish_client_tibber_data: &AsyncClient,
) -> Result<(), anyhow::Error> {
    println!("Executing Tibber job");
    let config = Config::new(TIBBER_API_URL)?;

    let session = tibber_loader::Session::new(config)
        .await
        .context("Failed to create Tibber API session")?;

    // Try to get the current price 3 times. If this is not successful,
    // we will try again in the next run of the job
    let mut retry_cnt = 0;
    loop {
        match session
            .get_current_price()
            .await
            .context("Failed to get current price from Tibber API")
        {
            Ok(Some(price)) => {
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

                return publish_client_tibber_data
                    .publish(
                        TIBBER_PRICE_INFORMATION_TOPIC.name(),
                        QoS::AtMostOnce,
                        false,
                        TIBBER_PRICE_INFORMATION_TOPIC.encode(&price_information),
                    )
                    .await
                    .context("Failed to publish current price Tibber message");
            }
            Ok(None) => {
                info!("No price information available");
            }
            Err(e) => {
                error!("Failed to get current price: {:?}", e);
            }
        }
        retry_cnt += 1;
        if retry_cnt == 3 {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        debug!("Retrying to get current price from Tibber API count ={retry_cnt}");
    }
    if retry_cnt == 3 {
        Err(anyhow::anyhow!(
            "Failed to get current price from Tibber API"
        ))
    } else {
        Ok(())
    }
}

async fn get_pulse_bridge_data_and_publish(
    publish_client_tibber_data: &AsyncClient,
) -> Result<(), anyhow::Error> {
    info!("Executing data fetch from Pulse Bridge job");

    let pulse_bridge_pwd = std::env::var("PULSE_BRIDGE_PASSWORD")
        .context("PULSE_BRIDGE_PASSWORD is not set")
        .and_then(|pwd| {
            if pwd.is_empty() {
                error!("PULSE_BRIDGE_PASSWORD is empty");
                std::process::exit(1);
            } else {
                Ok(pwd)
            }
        })?;

    let bridge_client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .context("Failed to build HTTP client")?;

    let mut resp = bridge_client
        .get(PULSE_BRIDGE_URL)
        .basic_auth(PULSE_BRIDGE_USERNAME, Some(pulse_bridge_pwd))
        .send()
        .await
        .context("Failed to read data from Tibber Bridge")?;

    let mut buffer = bytes::BytesMut::new();

    while let Some(chunk) = resp.chunk().await? {
        buffer.extend_from_slice(&chunk);
    }

    // We have only 1 message
    let message = decode(buffer.to_vec())[0]
        .clone()
        .context("Failed to decode pulse bridge message")?;
    let result = parse(&message).context("Failed to parse pulse bridge message")?;

    // 2nd message is GetListResponse with the values we are interested
    if result.messages.len() < 2 {
        Err(anyhow!("Expected 2 messages, got less"))
    } else {
        match &result.messages[1].message_body {
            MessageBody::GetListResponse(le) => {
                for entry in &le.val_list {
                    // Current power consumption
                    if entry.obj_name == [1, 0, 16, 7, 0, 255] {
                        let current_power = match entry.value {
                            Value::I32(v) => v,
                            _ => 0,
                        };
                        info!("Power = {current_power}W");

                        let res = publish_client_tibber_data
                            .publish(
                                PULSE_CONSUMPTION_TOPIC.name(),
                                QoS::AtMostOnce,
                                false,
                                PULSE_CONSUMPTION_TOPIC.encode(&Consumption {
                                    consumption: current_power,
                                }),
                            )
                            .await
                            .context("Failed to publish current consumption message");
                        return res;
                    }
                }
                Err(anyhow!("No power consumption data in pluse bridge data"))
            }
            _ => Err(anyhow!(
                "Wrong structure in pulse bridge data expected ListEntry missing"
            )),
        }
    }
}
