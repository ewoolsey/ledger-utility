use std::ops::Deref;

use btleplug::{api::Peripheral, platform};
use error::LedgerUtilityError;
use ledger_bluetooth::TransportNativeBle;
use ledger_transport::{async_trait, APDUAnswer, APDUCommand, Exchange};
use ledger_transport_hid::{
    hidapi::{DeviceInfo, HidApi},
    TransportNativeHID,
};

pub mod error;

pub enum Ledger {
    Bluetooth(TransportNativeBle),
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
            Ledger::Bluetooth(transport) => Ok(transport.exchange(command).await?),
            Ledger::Usb(transport) => Ok(transport.exchange(command)?),
        }
    }
}

pub enum Device {
    Bluetooth(platform::Peripheral),
    Usb(DeviceInfo),
}

impl Device {
    pub async fn name(&self) -> Result<String, LedgerUtilityError> {
        match self {
            Device::Bluetooth(peripheral) => {
                let properties = peripheral.properties().await.unwrap().unwrap();
                let local_name = properties
                    .local_name
                    .unwrap_or(String::from("(peripheral name unknown)"));
                Ok(format!("Bluetooth: {}", local_name))
            }
            Device::Usb(device_info) => Ok(format!(
                "Usb: {}",
                device_info.product_string().unwrap_or_default()
            )),
        }
    }
}

pub struct Connection {
    bluetooth: platform::Manager,
    hid: HidApi,
}

impl Connection {
    pub async fn new() -> Self {
        Self {
            bluetooth: platform::Manager::new().await.unwrap(),
            hid: HidApi::new().unwrap(),
        }
    }

    pub async fn get_all_ledgers(&self) -> Result<Vec<Device>, LedgerUtilityError> {
        let mut ledgers = vec![];
        ledgers.extend(TransportNativeHID::list_ledgers(&self.hid).map(|x| Device::Usb(x.clone())));
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
            Device::Bluetooth(peripheral) => {
                let transport = TransportNativeBle::connect(peripheral).await?;
                Ok(Ledger::Bluetooth(transport))
            }
            Device::Usb(device_info) => {
                let transport = TransportNativeHID::open_device(&self.hid, &device_info)?;
                Ok(Ledger::Usb(transport))
            }
        }
    }

    pub async fn connect_with_name(
        &self,
        device_name: String,
    ) -> Result<Ledger, LedgerUtilityError> {
        let mut devices = self.get_all_ledgers().await?;
        let mut names = Vec::new();
        for device in &devices {
            names.push(device.name().await?);
        }
        let index = names
            .iter()
            .position(|x| x == &device_name)
            .ok_or(LedgerUtilityError::DeviceNotFound)?;
        let device = devices.swap_remove(index);

        self.connect(device).await
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
