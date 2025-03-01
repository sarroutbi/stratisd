// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::Path;

use devicemapper::Device;

use crate::{
    engine::{
        strat_engine::{
            backstore::crypt::shared::{setup_crypt_device, setup_crypt_metadata_handle},
            metadata::StratisIdentifiers,
        },
        types::{DevicePath, EncryptionInfo, Name},
    },
    stratis::StratisResult,
};

/// Handle for reading metadata of a device that does not need to be active.
#[derive(Debug, Clone)]
pub struct CryptMetadataHandle {
    pub(super) physical_path: DevicePath,
    pub(super) identifiers: StratisIdentifiers,
    pub(super) encryption_info: EncryptionInfo,
    pub(super) activation_name: String,
    pub(super) pool_name: Option<Name>,
    pub(super) device: Device,
}

impl CryptMetadataHandle {
    pub(super) fn new(
        physical_path: DevicePath,
        identifiers: StratisIdentifiers,
        encryption_info: EncryptionInfo,
        activation_name: String,
        pool_name: Option<Name>,
        device: Device,
    ) -> Self {
        CryptMetadataHandle {
            physical_path,
            identifiers,
            encryption_info,
            activation_name,
            pool_name,
            device,
        }
    }

    /// Set up a handle to a crypt device for accessing metadata on the device.
    pub fn setup(physical_path: &Path) -> StratisResult<Option<CryptMetadataHandle>> {
        match setup_crypt_device(physical_path)? {
            Some(ref mut device) => setup_crypt_metadata_handle(device, physical_path),
            None => Ok(None),
        }
    }

    /// Get the encryption info for this encrypted device.
    pub fn encryption_info(&self) -> &EncryptionInfo {
        &self.encryption_info
    }

    /// Return the path to the device node of the underlying storage device
    /// for the encrypted device.
    pub fn luks2_device_path(&self) -> &Path {
        &self.physical_path
    }

    /// Get the Stratis device identifiers for a given encrypted device.
    pub fn device_identifiers(&self) -> &StratisIdentifiers {
        &self.identifiers
    }

    /// Get the name of the activated device when it is activated.
    pub fn activation_name(&self) -> &str {
        &self.activation_name
    }

    /// Get the pool name recorded in the LUKS2 metadata.
    pub fn pool_name(&self) -> Option<&Name> {
        self.pool_name.as_ref()
    }

    /// Device number for LUKS2 device.
    pub fn device(&self) -> &Device {
        &self.device
    }
}
