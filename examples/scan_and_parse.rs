//! End-to-end example: scan for a real BLE peripheral, connect, subscribe to
//! notifications on the Cycling Power or Heart Rate characteristic, and feed
//! the raw payload bytes into this crate's parsers.
//!
//! This is the missing piece the README's snippet doesn't show — that
//! snippet only parses a byte slice you already have in hand. This example
//! shows where those bytes actually come from: [`btleplug`] as the BLE
//! transport, `cycling-ble` for turning its raw notification payloads into
//! typed readings.
//!
//! Run with:
//!
//! ```text
//! cargo run --example scan_and_parse
//! ```
//!
//! Requires a Bluetooth adapter and a nearby peripheral advertising the
//! Cycling Power Service (0x1818) or Heart Rate Service (0x180D) — a power
//! meter, smart trainer, or heart rate strap. Scans for 5 seconds, connects
//! to the first matching device found, subscribes to its measurement
//! characteristic, and prints parsed readings as they arrive.

use std::error::Error;
use std::time::Duration;

use btleplug::api::{bleuuid::uuid_from_u16, Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use tokio::time;
use uuid::Uuid;

use cycling_ble::{heart_rate, power};

const CYCLING_POWER_SERVICE: Uuid = uuid_from_u16(0x1818);
const CYCLING_POWER_MEASUREMENT: Uuid = uuid_from_u16(0x2A63);
const HEART_RATE_SERVICE: Uuid = uuid_from_u16(0x180D);
const HEART_RATE_MEASUREMENT: Uuid = uuid_from_u16(0x2A37);

const SCAN_DURATION: Duration = Duration::from_secs(5);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await?;
    let adapter = manager
        .adapters()
        .await?
        .into_iter()
        .next()
        .ok_or("no Bluetooth adapter found")?;

    println!("scanning for {SCAN_DURATION:?}...");
    adapter.start_scan(ScanFilter::default()).await?;
    time::sleep(SCAN_DURATION).await;
    adapter.stop_scan().await?;

    let peripheral = find_cycling_peripheral(&adapter)
        .await?
        .ok_or("no Cycling Power or Heart Rate peripheral found")?;

    let properties = peripheral.properties().await?.unwrap_or_default();
    println!(
        "connecting to {}...",
        properties.local_name.as_deref().unwrap_or("(unnamed)")
    );
    peripheral.connect().await?;
    peripheral.discover_services().await?;

    let characteristics = peripheral.characteristics();

    if let Some(power_char) = characteristics
        .iter()
        .find(|c| c.uuid == CYCLING_POWER_MEASUREMENT)
    {
        peripheral.subscribe(power_char).await?;
        println!("subscribed to Cycling Power Measurement, waiting for notifications...");
    } else if let Some(hr_char) = characteristics
        .iter()
        .find(|c| c.uuid == HEART_RATE_MEASUREMENT)
    {
        peripheral.subscribe(hr_char).await?;
        println!("subscribed to Heart Rate Measurement, waiting for notifications...");
    } else {
        return Err("connected peripheral has neither characteristic".into());
    }

    let mut notifications = peripheral.notifications().await?;
    while let Some(notification) = notifications.next().await {
        match notification.uuid {
            CYCLING_POWER_MEASUREMENT => match power::parse(&notification.value) {
                Ok(measurement) => println!(
                    "power: {} W{}",
                    measurement.instantaneous_power_watts,
                    measurement
                        .pedal_power_balance
                        .map(|b| format!(", balance: {:.1}%", b.percent))
                        .unwrap_or_default()
                ),
                Err(e) => eprintln!("failed to parse Cycling Power Measurement: {e}"),
            },
            HEART_RATE_MEASUREMENT => match heart_rate::parse(&notification.value) {
                Ok(measurement) => println!("heart rate: {} bpm", measurement.bpm),
                Err(e) => eprintln!("failed to parse Heart Rate Measurement: {e}"),
            },
            _ => {}
        }
    }

    Ok(())
}

/// Scans the peripherals discovered so far for one advertising the Cycling
/// Power Service or Heart Rate Service.
async fn find_cycling_peripheral(adapter: &Adapter) -> Result<Option<Peripheral>, Box<dyn Error>> {
    for peripheral in adapter.peripherals().await? {
        let Some(properties) = peripheral.properties().await? else {
            continue;
        };
        if properties.services.contains(&CYCLING_POWER_SERVICE)
            || properties.services.contains(&HEART_RATE_SERVICE)
        {
            return Ok(Some(peripheral));
        }
    }
    Ok(None)
}
