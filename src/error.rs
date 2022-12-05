use ledger_bluetooth::LedgerBleError;
use ledger_transport_hid::LedgerHIDError;
/*******************************************************************************
*  Licensed under the Apache License, Version 2.0 (the "License");
*  you may not use this file except in compliance with the License.
*  You may obtain a copy of the License at
*
*      http://www.apache.org/licenses/LICENSE-2.0
*
*  Unless required by applicable law or agreed to in writing, software
*  distributed under the License is distributed on an "AS IS" BASIS,
*  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
*  See the License for the specific language governing permissions and
*  limitations under the License.
********************************************************************************/
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LedgerUtilityError {
    /// Error from the HID transport
    #[error("{0}")]
    Hid(#[from] LedgerHIDError),
    /// Error from the bluetooth transport
    #[error("{0}")]
    Ble(#[from] LedgerBleError),
}
