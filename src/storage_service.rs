use std::collections::HashMap;
use std::env::var;
use std::fs;

use serde::{Deserialize, Serialize};

use crate::{DeviceLocation, DeviceSettings};

#[allow(non_snake_case)]
#[derive(Serialize)]
struct MqttStatus {
    people: Vec<MqttStatusUser>,
    peopleCount: usize,
    deviceCount: usize,
    unknownDevicesCount: usize,
}

#[derive(Serialize)]
struct MqttStatusUser {
    name: String,
    devices: Vec<MqttStatusDevice>,
}

#[derive(Serialize)]
struct MqttStatusDevice {
    name: String,
    location: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StorageService {
    json_path: String,
    devices: HashMap<String, DeviceSettings>,
}

impl StorageService {
    pub fn new() -> StorageService {
        let json_path = var("DEVICE_JSON_PATH")
            .unwrap_or(String::from("device_data.json"));

        let stored_data = fs::read_to_string(&json_path).ok();
        let mut devices: HashMap<String, DeviceSettings> = HashMap::new();
        if stored_data.is_some() {
            let stored_devices: Vec<DeviceSettings> = serde_json::from_str(
                &stored_data.unwrap()
            ).unwrap();

            for device in stored_devices {
                devices.insert(device.mac_address.to_string(), device);
            }
        }

        StorageService {
            json_path,
            devices,
        }
    }

    pub fn stringify(&self, devices: &Vec<DeviceLocation>) -> String {
        let mut current_users: HashMap<String, Vec<(&DeviceLocation, DeviceSettings)>> = HashMap::new();
        let mut hidden: usize = 0;

        for network_device in devices.iter() {
            let device = self.fetch_device_by_mac(&network_device.device_mac);
            if device.is_some() {
                let device = device.unwrap();
                let user_alias = device.user_alias.to_string();

                if device.visibility == "ignore" {
                    hidden += 1;
                }

                let mut devices = current_users
                    .get(&user_alias)
                    .unwrap_or(&Vec::new())
                    .clone();

                devices.push((&network_device, device));
                current_users.insert(user_alias.to_string(), devices);
            }
        }

        let mut status = MqttStatus {
            people: Vec::new(),
            peopleCount: current_users.len() - hidden,
            deviceCount: devices.len() - hidden,
            unknownDevicesCount: devices.len() - current_users.len() - hidden,
        };

        for (user_alias, devices) in current_users {
            let mut user = MqttStatusUser {
                name: user_alias.to_string(),
                devices: Vec::new(),
            };

            for device in devices {
                let status_device = MqttStatusDevice {
                    name: device.1.device_alias.to_string(),
                    location: device.0.location.to_string(),
                };

                user.devices.push(status_device);
            }

            status.people.push(user);
        }

        serde_json::to_string(&status).unwrap()
    }

    pub fn fetch_device_by_mac(&self, mac_address: &str) -> Option<DeviceSettings> {
        let device = self.devices.get(mac_address);

        return if device.is_some() {
            Some(device.unwrap().clone())
        } else {
            None
        };
    }

    pub fn count_devices(&self) -> usize {
        self.devices.len()
    }

    pub fn persist_device(&mut self, mac_address: &String, device: DeviceSettings) {
        self.devices.insert(mac_address.to_string(), device);
        self.flush_to_json();
    }

    fn flush_to_json(&self) {
        let mut devices: Vec<DeviceSettings> = Vec::with_capacity(self.devices.capacity());
        for device in &self.devices {
            devices.push(device.1.clone());
        }

        let json = serde_json::to_string(devices.as_slice()).unwrap();
        fs::write(&self.json_path, json).unwrap();
    }
}