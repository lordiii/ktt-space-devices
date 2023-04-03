use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rocket::form::{Form, Strict};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};

use crate::{AppState, DeviceLocation, DeviceSettings, StorageService};
use crate::web_service::SettingsForm;

#[get("/")]
pub fn index(
    socket: SocketAddr,
    storage_service: &State<Arc<Mutex<StorageService>>>,
    network_devices: &State<Arc<Mutex<Vec<DeviceLocation>>>>,
) -> Template {
    let storage_service = storage_service.lock().unwrap();

    let mac_address = get_mac_by_ip(network_devices.inner(), socket.ip().to_string());

    return if mac_address.is_some() {
        let mac_address = mac_address.unwrap();
        let device = storage_service.fetch_device_by_mac(&mac_address);

        return if device.is_some() {
            let device = device.unwrap();

            Template::render("index", context! {
                user_alias: device.user_alias,
                device_alias: device.device_alias,
                mac_address: mac_address,
                visibility_options: crate::web_service::get_visibility_options(Some(device.visibility)),
                device_count: storage_service.count_devices(),
                has_data: true
            })
        } else {
            Template::render("index", context! {
                user_alias: String::new(),
                device_alias: String::new(),
                mac_address: mac_address,
                visibility_options: crate::web_service::get_visibility_options(None),
                device_count: storage_service.count_devices(),
                has_data: true
            })
        };
    } else {
        Template::render("index", context! {
            user_alias: String::from("????"),
            device_alias: String::from("????"),
            mac_address: String::from("????")
        })
    };
}

#[post("/device-settings", data = "<form>")]
pub fn save_device_settings(
    socket: SocketAddr,
    form: Form<Strict<SettingsForm>>,
    storage_service: &State<Arc<Mutex<StorageService>>>,
    network_devices: &State<Arc<Mutex<Vec<DeviceLocation>>>>,
    app_state: &State<Arc<Mutex<AppState>>>,
) -> Redirect {
    let mut storage_service = storage_service.lock().unwrap();
    let mac_address = get_mac_by_ip(network_devices.inner(), socket.ip().to_string());

    if mac_address.is_some() {
        let mac_address = mac_address.unwrap();

        let mut device = storage_service.fetch_device_by_mac(&mac_address).unwrap_or(
            DeviceSettings {
                user_alias: String::new(),
                device_alias: String::new(),
                mac_address: mac_address.to_string(),
                visibility: String::new(),
                last_changed: 0,
            }
        );

        device.user_alias = form.user_alias.to_string();
        device.device_alias = form.device_alias.to_string();
        device.visibility = form.visibility.to_string();

        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        device.last_changed = since_the_epoch.as_millis();

        storage_service.persist_device(&mac_address, device);
        let mut app_state = app_state.lock().unwrap();
        app_state.send_devices = true;
    }

    Redirect::to(uri!("/"))
}

fn get_mac_by_ip(network_devices: &Arc<Mutex<Vec<DeviceLocation>>>, ip: String) -> Option<String> {
    let network_devices = network_devices.lock().unwrap();

    let network_device = network_devices.iter().find(
        |device| device.ipv4.eq(&ip.to_string())
    );

    return if network_device.is_some() {
        Some(network_device.unwrap().device_mac.to_string())
    } else {
        None
    };
}

#[catch(404)]
pub fn catch_404() -> Redirect {
    Redirect::to(uri!("/"))
}

#[catch(500)]
pub fn catch_500() -> Redirect {
    Redirect::to(uri!("/"))
}