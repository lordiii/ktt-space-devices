use std::env::var;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use rocket::Config;
use rocket::fs::FileServer;
use rocket_dyn_templates::Template;
use serde::Serialize;

use crate::{AppState, DeviceLocation};
use crate::storage_service::StorageService;

mod web_server_paths;

#[derive(FromForm)]
pub struct SettingsForm {
    user_alias: String,
    device_alias: String,
    visibility: String,
}

#[derive(Serialize)]
pub struct VisibilityOption {
    id: String,
    value: String,
    checked: bool,
    description: String,
}

pub async fn launch_rocket(
    storage_service: Arc<Mutex<StorageService>>,
    devices: Arc<Mutex<Vec<DeviceLocation>>>,
    app_state: Arc<Mutex<AppState>>,
) {
    let debug = var("DEBUG").unwrap_or("0".into()).eq("1");

    let web_host = var("WEB_HOST").unwrap_or("127.0.0.1".into());
    let web_port = u16::from_str(
        &var("WEB_PORT").unwrap_or("8000".into())
    ).expect("Invalid Web Port");

    let rocket_config = if debug {
        Config {
            port: web_port,
            address: IpAddr::from_str(&web_host).expect("Invalid IP address").into(),
            ..Config::debug_default()
        }
    } else {
        Config {
            port: web_port,
            address: IpAddr::from_str(&web_host).expect("Invalid IP address").into(),
            ..Config::release_default()
        }
    };

    let _ = rocket::custom(rocket_config)
        .mount("/", routes![
            web_server_paths::index,
            web_server_paths::save_device_settings
        ])
        .mount("/js", FileServer::from("templates/js"))
        .mount("/css", FileServer::from("templates/css"))
        .mount("/images", FileServer::from("templates/images"))
        .register("/", catchers![
            web_server_paths::catch_404
        ])
        .register("/", catchers![
            web_server_paths::catch_500
        ])
        .attach(Template::custom(|_engines| {}))
        .manage(storage_service)
        .manage(devices)
        .manage(app_state)
        .launch()
        .await;
}

pub fn get_visibility_options(visibility: Option<String>) -> Vec<VisibilityOption> {
    let visibility: String = visibility.unwrap_or("all".to_string());

    vec![
        VisibilityOption {
            id: String::from("radioVisibilityAll"),
            value: String::from("all"),
            checked: visibility.eq("all"),
            description: String::from("Alles anzeigen, d.h. Name/Alias und Gerätename"),
        },
        VisibilityOption {
            id: String::from("radioVisibilityAlias"),
            value: String::from("user"),
            checked: visibility.eq("user"),
            description: String::from("Mit Name/Alias anzeigen"),
        },
        VisibilityOption {
            id: String::from("radioVisibilityAnonymous"),
            value: String::from("anon"),
            checked: visibility.eq("anon"),
            description: String::from("Als anonyme Person anzeigen"),
        },
        VisibilityOption {
            id: String::from("radioVisibilityNone"),
            value: String::from("ignore"),
            checked: visibility.eq("ignore"),
            description: String::from("Gar nicht anzeigen. Die wirklich paranoide Option. Meistens ist der obere Punkt besser."),
        },
    ]
}