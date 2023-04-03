#[macro_use]
extern crate rocket;

use std::env::var;
use std::sync::{Arc, Mutex};

use dotenv::dotenv;
use rocket::tokio;
use serde::Deserialize;
use serde::Serialize;
use tokio::task::JoinHandle;

use crate::mqtt_service::MqttService;
use crate::storage_service::StorageService;

mod web_service;
mod storage_service;
mod mqtt_service;

pub struct AppState {
    shutdown: bool,
    send_devices: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DeviceSettings {
    user_alias: String,
    device_alias: String,
    mac_address: String,
    visibility: String,
    last_changed: u128,
}

#[derive(Deserialize)]
pub struct DeviceLocation {
    ipv4: String,
    ipv6: Vec<String>,
    device_mac: String,
    remote_ip: String,
    remote_mac: String,
    location: String,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let app_state: Arc<Mutex<AppState>> = Arc::new(
        Mutex::new(
            AppState {
                shutdown: false,
                send_devices: false,
            }
        )
    );

    let devices: Arc<Mutex<Vec<DeviceLocation>>> = Arc::new(
        Mutex::new(Vec::new())
    );
    let storage_service: Arc<Mutex<StorageService>> = Arc::new(
        Mutex::new(StorageService::new())
    );

    let threads = [
        task_publish_devices(
            app_state.clone(),
            storage_service.clone(),
            devices.clone(),
        ),
        task_get_devices(
            app_state.clone(),
            devices.clone(),
        ),
        task_rocket(
            app_state.clone(),
            storage_service.clone(),
            devices.clone(),
        )
    ];

    for t in threads {
        let _ = t.await;
    }
}

fn task_publish_devices(
    app_state: Arc<Mutex<AppState>>,
    storage_service: Arc<Mutex<StorageService>>,
    devices: Arc<Mutex<Vec<DeviceLocation>>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut mqtt = MqttService::new("task_publish_devices");

        let topic = var("MQTT_PUBLISH_TOPIC").expect("Missing MQTT Publish Topic");
        if topic.is_empty() {
            panic!("Missing MQTT Publish Topic");
        }

        loop {
            if app_state.lock().unwrap().shutdown {
                println!("Ending publish Devices Task");
                break;
            }

            if app_state.lock().unwrap().send_devices {
                let json_data = get_current_device_json(
                    storage_service.clone(),
                    devices.clone(),
                );

                mqtt.publish(topic.to_string(), json_data.to_string()).await;
                app_state.lock().unwrap().send_devices = false;
            }

            mqtt.receive().await;
        }
    })
}

fn get_current_device_json(
    storage_service: Arc<Mutex<StorageService>>,
    mac_addresses: Arc<Mutex<Vec<DeviceLocation>>>,
) -> String {
    let mac_addresses = mac_addresses.lock().unwrap();
    storage_service.lock().unwrap().stringify(&mac_addresses)
}

fn task_rocket(
    app_state: Arc<Mutex<AppState>>,
    storage_service: Arc<Mutex<StorageService>>,
    devices: Arc<Mutex<Vec<DeviceLocation>>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        web_service::launch_rocket(
            storage_service,
            devices,
            app_state.clone(),
        ).await;

        let mut app_state = app_state.lock().unwrap();
        app_state.shutdown = true;
    })
}

fn task_get_devices(
    app_state: Arc<Mutex<AppState>>,
    devices: Arc<Mutex<Vec<DeviceLocation>>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut mqtt = MqttService::new("task_get_wifi_devices");

        let topic = var("MQTT_DEVICE_TOPIC").expect("Missing MQTT Device Topic");
        if topic.is_empty() {
            panic!("Missing MQTT Device Topic");
        }
        mqtt.subscribe(&topic).await;

        loop {
            if app_state.lock().unwrap().shutdown {
                println!("Ending WiFi Devices Task");
                break;
            }

            let data = mqtt.receive().await;
            if data.is_some() {
                app_state.lock().unwrap().send_devices = true;
                *devices.lock().unwrap() = data.unwrap();
            }
        }
    })
}
