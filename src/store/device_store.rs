use super::redb_store::{DEVICE_TABLE, RedbStore};
use async_trait::async_trait;
use whatsapp_rust::{
    store::{
        DeviceStore,
        error::{Result, StoreError},
    },
    wacore::store::Device,
};

const DEVICE_ROW_ID: u32 = 1;

#[async_trait]
impl DeviceStore for RedbStore {
    /// Save device data.
    async fn save(&self, device: &Device) -> Result<()> {
        let encoded = bincode::serde::encode_to_vec(device, bincode::config::standard())
            .map_err(|e| StoreError::Database(Box::new(e)))?;

        self.with_write_txn(DEVICE_TABLE, |table| {
            table
                .insert(DEVICE_ROW_ID, encoded.as_slice())
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
    }

    /// Load device data.
    async fn load(&self) -> Result<Option<Device>> {
        self.with_read_txn(DEVICE_TABLE, |table| {
            if let Some(data) = table
                .get(DEVICE_ROW_ID)
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                let (decoded, _): (Device, usize) =
                    bincode::serde::decode_from_slice(data.value(), bincode::config::standard())
                        .map_err(|e| StoreError::Database(Box::new(e)))?;
                Ok(Some(decoded))
            } else {
                Ok(None)
            }
        })
    }

    /// Check if a device exists.
    async fn exists(&self) -> Result<bool> {
        self.with_read_txn(DEVICE_TABLE, |table| {
            let has_key = table
                .get(DEVICE_ROW_ID)
                .map_err(|e| StoreError::Database(Box::new(e)))?
                .is_some();
            Ok(has_key)
        })
    }

    /// Create a new device row and return its generated device_id.
    async fn create(&self) -> Result<i32> {
        let device = Device::new();

        let bytes = bincode::serde::encode_to_vec(&device, bincode::config::standard())
            .map_err(|e| StoreError::Database(Box::new(e)))?;

        self.with_write_txn(DEVICE_TABLE, |table| {
            table
                .insert(DEVICE_ROW_ID, bytes.as_slice())
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(DEVICE_ROW_ID as i32)
        })
    }

    /// Create a snapshot of the database state.
    /// The argument `name` can be used to label the snapshot file.
    /// `extra_content` can be used to save a related binary blob (e.g. the message that caused the failure).
    async fn snapshot_db(&self, _name: &str, _extra_content: Option<&[u8]>) -> Result<()> {
        Ok(())
    }
}
