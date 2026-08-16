//! End-to-end example: scan for a Zwift Click controller, connect, perform
//! the handshake write it needs before it'll notify, subscribe, and feed
//! the raw payload bytes into [`cycling_ble::zwift_click::parse`].
//!
//! Unlike [`scan_and_parse`](scan_and_parse.rs), the Click doesn't just sit
//! there notifying once connected — its GATT characteristics stay silent
//! until the app writes [`HANDSHAKE_REQUEST`](cycling_ble::zwift_click::HANDSHAKE_REQUEST)
//! to its write characteristic. This example shows that extra step.
//!
//! Run with:
//!
//! ```text
//! cargo run --example zwift_click
//! ```
//!
//! Requires a Bluetooth adapter and a nearby Zwift Click, powered on and
//! not already connected to something else (e.g. the Zwift/Companion
//! apps). Scans for 5 seconds, connects to the first matching device
//! found, performs the handshake, and prints button-state changes as they
//! arrive.

use std::error::Error;
use std::time::{Duration, Instant};

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use tokio::time;
use uuid::Uuid;

use cycling_ble::zwift_click;

const CLICK_SERVICE: Uuid = Uuid::from_bytes([
    0x00, 0x00, 0x00, 0x01, 0x19, 0xCA, 0x46, 0x51, 0x86, 0xE5, 0xFA, 0x29, 0xDC, 0xDD, 0x09, 0xD1,
]);
const CLICK_ASYNC_NOTIFY: Uuid = Uuid::from_bytes([
    0x00, 0x00, 0x00, 0x02, 0x19, 0xCA, 0x46, 0x51, 0x86, 0xE5, 0xFA, 0x29, 0xDC, 0xDD, 0x09, 0xD1,
]);
const CLICK_SYNC_RX_WRITE: Uuid = Uuid::from_bytes([
    0x00, 0x00, 0x00, 0x03, 0x19, 0xCA, 0x46, 0x51, 0x86, 0xE5, 0xFA, 0x29, 0xDC, 0xDD, 0x09, 0xD1,
]);

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

    let peripheral = find_click(&adapter).await?.ok_or("no Zwift Click found")?;

    let properties = peripheral.properties().await?.unwrap_or_default();
    println!(
        "connecting to {}...",
        properties.local_name.as_deref().unwrap_or("(unnamed)")
    );
    peripheral.connect().await?;
    peripheral.discover_services().await?;

    let services = peripheral.services();
    if !services.iter().any(|s| s.uuid == CLICK_SERVICE) {
        eprintln!(
            "warning: connected peripheral doesn't expose the expected \
             Click GATT service ({CLICK_SERVICE}) — proceeding anyway, \
             but this may not be a real Click"
        );
    }

    let characteristics = peripheral.characteristics();
    let notify_char = characteristics
        .iter()
        .find(|c| c.uuid == CLICK_ASYNC_NOTIFY)
        .ok_or("connected peripheral is missing the Async notify characteristic")?;
    let write_char = characteristics
        .iter()
        .find(|c| c.uuid == CLICK_SYNC_RX_WRITE)
        .ok_or("connected peripheral is missing the SyncRx write characteristic")?;

    peripheral.subscribe(notify_char).await?;

    println!("sending handshake...");
    peripheral
        .write(
            write_char,
            zwift_click::HANDSHAKE_REQUEST,
            WriteType::WithoutResponse,
        )
        .await?;

    println!("waiting for button notifications...");
    let start = Instant::now();
    let mut notifications = peripheral.notifications().await?;
    let mut last_state = zwift_click::ClickButtonState::default();
    let mut last_raw: Option<Vec<u8>> = None;
    let mut heartbeat = time::interval(Duration::from_secs(5));
    loop {
        tokio::select! {
            maybe_notification = notifications.next() => {
                let Some(notification) = maybe_notification else {
                    println!("[{:>6.1}s] notification stream ended (disconnected)", start.elapsed().as_secs_f32());
                    break;
                };
                // The Click streams at a high rate even at idle; only print
                // when the payload actually changes, or button-press edges
                // get buried in identical idle frames.
                if last_raw.as_deref() != Some(notification.value.as_slice()) {
                    println!(
                        "[{:>6.1}s] raw notification: uuid={} value={:?}",
                        start.elapsed().as_secs_f32(),
                        notification.uuid,
                        notification.value
                    );
                    last_raw = Some(notification.value.clone());
                }
                if notification.uuid != CLICK_ASYNC_NOTIFY {
                    continue;
                }
                if zwift_click::is_handshake_ack(&notification.value) {
                    println!("handshake acknowledged");
                    continue;
                }
                match zwift_click::parse(&notification.value) {
                    Ok(Some(state)) if state != last_state => {
                        println!("{state:?}");
                        last_state = state;
                    }
                    Ok(_) => {}
                    Err(e) => eprintln!("failed to parse Zwift Click notification: {e}"),
                }
            }
            _ = heartbeat.tick() => {
                let connected = peripheral.is_connected().await.unwrap_or(false);
                println!("[{:>6.1}s] heartbeat: connected={connected}", start.elapsed().as_secs_f32());
                if !connected {
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Scans the peripherals discovered so far for one named "Zwift Click".
///
/// Matching on `CLICK_SERVICE` doesn't work here: that UUID is the GATT
/// service exposed *after* connecting, but the advertisement itself
/// carries a different, shorter service UUID (`0xfc82`) — confirmed by
/// scanning real hardware. Name matching is what actually works pre-connect.
async fn find_click(adapter: &Adapter) -> Result<Option<Peripheral>, Box<dyn Error>> {
    for peripheral in adapter.peripherals().await? {
        let Some(properties) = peripheral.properties().await? else {
            continue;
        };
        if properties.local_name.as_deref() == Some("Zwift Click") {
            return Ok(Some(peripheral));
        }
    }
    Ok(None)
}
