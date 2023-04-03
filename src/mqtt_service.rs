use std::env::var;
use std::str::from_utf8;
use std::time::Duration;

use rumqttc::{AsyncClient, EventLoop, MqttOptions, Publish, QoS, TlsConfiguration, Transport};
use tokio::time;
use crate::DeviceLocation;

pub struct MqttService {
    mqtt_client: AsyncClient,
    mqtt_events: EventLoop,
    topics: Vec<String>,
}

impl MqttService {
    pub fn new(client_id: &str) -> MqttService {
        let mqtt_host = var("MQTT_HOST").unwrap_or(String::from("mqtts://mainframe.io:8884"));
        let mqtt_user = var("MQTT_USER").unwrap_or(String::from("test"));
        let mqtt_pass = var("MQTT_PASS").unwrap_or(String::from("test"));

        let mut mqtt_options = MqttOptions::parse_url(
            format!("{}?client_id={}",
                    mqtt_host,
                    client_id)
        ).unwrap();

        mqtt_options.set_credentials(mqtt_user, mqtt_pass);
        mqtt_options.set_transport(Transport::Tls(TlsConfiguration::Native));
        mqtt_options.set_keep_alive(Duration::from_secs(5));
        mqtt_options.set_max_packet_size(1024 * 1000, 1024 * 1000);

        let (mqtt_client, mqtt_events) = AsyncClient::new(mqtt_options, 10);

        MqttService {
            mqtt_client,
            mqtt_events,
            topics: Vec::new(),
        }
    }

    pub async fn receive(&mut self) -> Option<Vec<DeviceLocation>> {
        let result = time::timeout(
            Duration::from_millis(2000),
            self.mqtt_events.poll(),
        ).await;
        if result.is_ok() {
            let result = result.unwrap();

            if result.is_ok() {
                match result.unwrap() {
                    rumqttc::Event::Incoming(pkg) => {
                        match pkg {
                            rumqttc::v4::Packet::Publish(pkg) => {
                                return self.parse_packet(pkg);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        None
    }

    pub async fn publish(&mut self, topic: String, data: String) {
        let _ = time::timeout(
            Duration::from_millis(2000),
            self.mqtt_client.publish(topic, QoS::AtMostOnce, false, data),
        ).await;
    }

    pub async fn subscribe(&mut self, topic: &str) {
        self.topics.push(topic.to_string());
        self.mqtt_client.subscribe(topic, QoS::AtMostOnce).await.unwrap();
    }

    fn parse_packet(&self, pkg: Publish) -> Option<Vec<DeviceLocation>> {
        let data = from_utf8(&pkg.payload);
        return if data.is_ok() {
            serde_json::from_str(data.unwrap()).ok()
        } else {
            None
        };
    }
}