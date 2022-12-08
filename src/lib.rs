use std::{fmt::Debug, ops::Deref};

#[cfg(feature = "bluetooth")]
use btleplug::{api::Peripheral, platform};
use error::LedgerUtilityError;
#[cfg(feature = "bluetooth")]
use ledger_bluetooth::TransportNativeBle;

use ledger_transport::{async_trait, APDUAnswer, APDUCommand, Exchange};
#[cfg(feature = "usb")]
use ledger_transport_hid::{
    hidapi::{DeviceInfo, HidApi},
    TransportNativeHID,
};

pub mod error;

#[cfg(not(feature = "bluetooth"))]
#[cfg(not(feature = "usb"))]
compile_error!("You must enable at least one transport feature: bluetooth or usb");

pub enum Ledger {
    #[cfg(feature = "bluetooth")]
    Bluetooth(TransportNativeBle),
    #[cfg(feature = "usb")]
    Usb(TransportNativeHID),
}

#[async_trait]
impl Exchange for Ledger {
    type Error = LedgerUtilityError;
    type AnswerType = Vec<u8>;

    async fn exchange<I>(
        &self,
        command: &APDUCommand<I>,
    ) -> Result<APDUAnswer<Self::AnswerType>, Self::Error>
    where
        I: Deref<Target = [u8]> + Send + Sync,
    {
        match self {
            #[cfg(feature = "bluetooth")]
            Ledger::Bluetooth(transport) => Ok(transport.exchange(command).await?),
            #[cfg(feature = "usb")]
            Ledger::Usb(transport) => Ok(transport.exchange(command)?),
        }
    }
}

pub enum Device {
    #[cfg(feature = "bluetooth")]
    Bluetooth(platform::Peripheral),
    #[cfg(feature = "usb")]
    Usb(DeviceInfo),
}

impl Device {
    pub async fn name(&self) -> Result<String, LedgerUtilityError> {
        match self {
            #[cfg(feature = "bluetooth")]
            Device::Bluetooth(peripheral) => {
                if !peripheral.is_connected().await.unwrap() {
                    peripheral.connect().await.unwrap();
                }
                let properties = peripheral.properties().await.unwrap().unwrap();
                let local_name = properties
                    .local_name
                    .unwrap_or(String::from("(peripheral name unknown)"));
                Ok(format!("Bluetooth: {}", local_name))
            }
            #[cfg(feature = "usb")]
            Device::Usb(device_info) => Ok(format!(
                "Usb: {}",
                device_info.product_string().unwrap_or_default()
            )),
        }
    }
}

pub struct Connection {
    #[cfg(feature = "bluetooth")]
    bluetooth: platform::Manager,
    #[cfg(feature = "usb")]
    hid: HidApi,
}

impl Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("Connection");
        #[cfg(feature = "bluetooth")]
        debug.field("bluetooth", &self.bluetooth);
        debug.finish()
    }
}

impl Connection {
    pub async fn new() -> Self {
        Self {
            #[cfg(feature = "bluetooth")]
            bluetooth: platform::Manager::new().await.unwrap(),
            #[cfg(feature = "usb")]
            hid: HidApi::new().unwrap(),
        }
    }

    pub async fn get_all_ledgers(&self) -> Result<Vec<Device>, LedgerUtilityError> {
        let mut ledgers = vec![];
        #[cfg(feature = "usb")]
        ledgers.extend(TransportNativeHID::list_ledgers(&self.hid).map(|x| Device::Usb(x.clone())));
        #[cfg(feature = "bluetooth")]
        ledgers.extend(
            TransportNativeBle::list_ledgers(&self.bluetooth)
                .await?
                .into_iter()
                .map(Device::Bluetooth),
        );
        Ok(ledgers)
    }

    pub async fn connect(&self, device: Device) -> Result<Ledger, LedgerUtilityError> {
        match device {
            #[cfg(feature = "bluetooth")]
            Device::Bluetooth(peripheral) => {
                let transport = TransportNativeBle::connect(peripheral).await?;
                Ok(Ledger::Bluetooth(transport))
            }
            #[cfg(feature = "usb")]
            Device::Usb(device_info) => {
                let transport = TransportNativeHID::open_device(&self.hid, &device_info)?;
                Ok(Ledger::Usb(transport))
            }
        }
    }

    pub async fn connect_with_name(
        &self,
        device_name: String,
        num_retries: u8,
    ) -> Result<Ledger, LedgerUtilityError> {
        for _ in 0..num_retries {
            let mut devices = self.get_all_ledgers().await?;
            let mut names = Vec::new();
            for device in &devices {
                names.push(device.name().await?);
            }
            let index = match names.iter().position(|x| x == &device_name) {
                Some(i) => i,
                None => continue,
            };

            let device = devices.swap_remove(index);

            return self.connect(device).await;
        }
        Err(LedgerUtilityError::DeviceNotFound)
    }
}
#[cfg(test)]
mod test {
    use log::info;
    use serial_test::serial;

    use super::*;
    #[tokio::test]
    #[serial]
    async fn test_get_all_ledgers() {
        let connection = Connection::new().await;
        let ledgers = connection.get_all_ledgers().await.unwrap();
        for ledger in ledgers {
            info!("{}", ledger.name().await.unwrap())
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_connect() {
        let connection = Connection::new().await;
        let mut ledgers = connection.get_all_ledgers().await.unwrap();
        if let Some(ledger) = ledgers.pop() {
            connection.connect(ledger).await.unwrap();
        }
    }
}
