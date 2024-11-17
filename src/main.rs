/// This is a dumb way to easily factory reset shelly device
/// https://shelly-api-docs.shelly.cloud/docs-ble/encryption/
use btleplug::api::{Central, Manager as _, Peripheral};
use btleplug::platform::Manager;
use clap::Parser;
use tokio::time::Duration;
use uuid::Uuid;

/// Simple CLI tool to connect to a Bluetooth device and send data
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// MAC address of the target Bluetooth device
    #[arg(short, long)]
    macaddr: String,

    /// UUID of the characteristic to write to (default: b0a7e40f-2b87-49db-801c-eb3686a24bdb)
    #[arg(short, long, default_value = "b0a7e40f-2b87-49db-801c-eb3686a24bdb")]
    uuid: String,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments
    let args = Args::parse();

    println!(
        "Press 10 seconds pair button on the device, then pair the device with `bluetoothctl scan`, then pair, enter 0000 as pin, \
        then as passkey enter the encryption code shown on the Android app."
    );

    // Extract MAC address and UUID
    let target_mac = &args.macaddr;
    let target_uuid = Uuid::parse_str(&args.uuid)?;

    // Initialize BLE manager
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    if adapters.is_empty() {
        eprintln!("No Bluetooth adapters found!");
        return Ok(());
    }
    let central = adapters.into_iter().next().unwrap();

    // Start scanning for devices
    central.start_scan(Default::default()).await?;
    tokio::time::sleep(Duration::from_secs(5)).await; // Allow time for scanning

    // Find the target device
    let devices = central.peripherals().await?;
    let device = devices.into_iter().find(|d| {
        // Await the properties of the device
        if let Ok(Some(props)) = futures::executor::block_on(d.properties()) {
            props.address.to_string().eq_ignore_ascii_case(target_mac)
        } else {
            false
        }
    });

    let device = match device {
        Some(device) => device,
        None => {
            eprintln!("Device with MAC address {} not found!", target_mac);
            return Ok(());
        }
    };

    // Connect to the device
    device.connect().await?;
    println!("Connected to device: {}", target_mac);

    // Discover services and characteristics
    device.discover_services().await?;
    let characteristics = device.characteristics();
    let target_characteristic = characteristics.iter().find(|c| c.uuid == target_uuid);

    let target_characteristic = match target_characteristic {
        Some(c) => c,
        None => {
            eprintln!("Characteristic with UUID {} not found!", target_uuid);
            return Ok(());
        }
    };

    // Write data to the characteristic
    let data = b"1";
    device
        .write(
            target_characteristic,
            data,
            btleplug::api::WriteType::WithResponse,
        )
        .await?;
    println!("Data sent successfully, device resetted !");

    // Disconnect
    device.disconnect().await?;
    println!("Disconnected from device.");

    Ok(())
}
