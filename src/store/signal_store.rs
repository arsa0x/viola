use super::{
    IDENTITIES_TABLE, PREKEYS_TABLE, PreKeyRecord, RedbStore, SENDER_KEYS_TABLE, SESSIONS_TABLE,
    SIGNED_PREKEYS_TABLE,
};
use async_trait::async_trait;
use bytes::Bytes;
use redb::ReadableTable;
use whatsapp_rust::store::{
    SignalStore,
    error::{Result, StoreError},
};

#[async_trait]
impl SignalStore for RedbStore {
    /// Store an identity key for a remote address
    async fn put_identity(&self, address: &str, key: [u8; 32]) -> Result<()> {
        let address = address.to_string();
        let device_id = self.device_id;
        self.with_write_txn(IDENTITIES_TABLE, move |table| {
            table
                .insert((address.as_str(), device_id), &key)
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Load an identity key for a remote address
    async fn load_identity(&self, address: &str) -> Result<Option<[u8; 32]>> {
        self.with_read_txn(IDENTITIES_TABLE, |table| {
            match table
                .get((address, self.device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                Some(data) => Ok(Some(*data.value())),
                None => Ok(None),
            }
        })
    }

    /// Delete an identity key
    async fn delete_identity(&self, address: &str) -> Result<()> {
        let address = address.to_string();
        let device_id = self.device_id;
        self.with_write_txn(IDENTITIES_TABLE, move |table| {
            table
                .remove((address.as_str(), device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Get an encrypted session for an address
    async fn get_session(&self, address: &str) -> Result<Option<Bytes>> {
        self.with_read_txn(SESSIONS_TABLE, |table| {
            match table
                .get((address, self.device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                Some(data) => Ok(Some(Bytes::copy_from_slice(data.value()))),
                None => Ok(None),
            }
        })
    }

    /// Store an encrypted session
    async fn put_session(&self, address: &str, session: &[u8]) -> Result<()> {
        let address = address.to_string();
        let session = session.to_vec();
        let device_id = self.device_id;
        self.with_write_txn(SESSIONS_TABLE, move |table| {
            table
                .insert((address.as_str(), device_id), session.as_slice())
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Delete a session
    async fn delete_session(&self, address: &str) -> Result<()> {
        let address = address.to_string();
        let device_id = self.device_id;
        self.with_write_txn(SESSIONS_TABLE, move |table| {
            table
                .remove((address.as_str(), device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Check if a session exists (default implementation uses get_session)
    async fn has_session(&self, address: &str) -> Result<bool> {
        match self
            .get_session(address)
            .await
            .map_err(|e| StoreError::Database(Box::new(e)))?
        {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    /// Store a pre-key
    async fn store_prekey(&self, id: u32, record: &[u8], uploaded: bool) -> Result<()> {
        let record = PreKeyRecord {
            key: record.to_vec(),
            uploaded,
        };

        let encoded = self.encode(&record)?;

        let device_id = self.device_id;

        self.with_write_txn(PREKEYS_TABLE, move |table| {
            table
                .insert((id, device_id), encoded.as_slice())
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Store multiple pre-keys in a single batch operation (default loops over store_prekey)
    async fn store_prekeys_batch(&self, keys: &[(u32, Bytes)], uploaded: bool) -> Result<()> {
        let device_id = self.device_id;
        let mut r = Vec::with_capacity(keys.len());
        for (id, record) in keys {
            let data = PreKeyRecord {
                key: record.to_vec(),
                uploaded,
            };

            let encoded = self.encode(&data)?;
            r.push((*id, encoded));
        }

        self.with_write_txn(PREKEYS_TABLE, move |table| {
            for (id, encoded) in r {
                table
                    .insert((id, device_id), encoded.as_slice())
                    .map_err(|e| StoreError::Database(Box::new(e)))?;
            }
            Ok(())
        })
        .await
    }

    /// Load a pre-key by ID
    async fn load_prekey(&self, id: u32) -> Result<Option<Bytes>> {
        self.with_read_txn(PREKEYS_TABLE, |table| {
            match table
                .get((id, self.device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                Some(data) => {
                    let decoded: PreKeyRecord = self.decode(data.value())?;
                    Ok(Some(Bytes::from(decoded.key)))
                }
                None => Ok(None),
            }
        })
    }

    /// Load multiple pre-keys by ID in a single batch operation (default loops over load_prekey)
    async fn load_prekeys_batch(&self, ids: &[u32]) -> Result<Vec<(u32, Bytes)>> {
        self.with_read_txn(PREKEYS_TABLE, |table| {
            let mut result: Vec<(u32, Bytes)> = Vec::with_capacity(ids.len());

            for id in ids {
                if let Some(data) = table
                    .get((*id, self.device_id))
                    .map_err(|e| StoreError::Database(Box::new(e)))?
                {
                    let decoded: PreKeyRecord = self.decode(data.value())?;
                    result.push((*id, Bytes::from(decoded.key)));
                }
            }
            Ok(result)
        })
    }

    /// Mark already-stored pre-keys as uploaded WITHOUT inserting.
    async fn mark_prekeys_uploaded(&self, ids: &[u32]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let ids = ids.to_vec();
        let device_id = self.device_id;

        let this = self.clone();

        self.with_write_txn(PREKEYS_TABLE, move |table| {
            for id in ids {
                let mut record_to_update = None;
                {
                    if let Some(data) = table
                        .get((id, device_id))
                        .map_err(|e| StoreError::Database(Box::new(e)))?
                    {
                        let record: PreKeyRecord = this.decode(data.value())?;
                        record_to_update = Some(record);
                    }
                }
                if let Some(mut record) = record_to_update {
                    record.uploaded = true;
                    let encoded = this.encode(&record)?;
                    table
                        .insert((id, device_id), encoded.as_slice())
                        .map_err(|e| StoreError::Database(Box::new(e)))?;
                }
            }
            Ok(())
        })
        .await
    }

    /// Remove a pre-key
    async fn remove_prekey(&self, id: u32) -> Result<()> {
        let device_id = self.device_id;
        self.with_write_txn(PREKEYS_TABLE, move |table| {
            table
                .remove((id, device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Get the highest pre-key ID currently stored
    async fn get_max_prekey_id(&self) -> Result<u32> {
        self.with_read_txn(PREKEYS_TABLE, |table| {
            let iter = table
                .range::<(u32, u8)>(..)
                .map_err(|e| StoreError::Database(Box::new(e)))?;

            for result in iter.rev() {
                let (key_access, _) = result.map_err(|e| StoreError::Database(Box::new(e)))?;
                let (id, db_device_id) = key_access.value();
                if db_device_id == self.device_id {
                    return Ok(id);
                }
            }
            Ok(0)
        })
    }

    /// Store a signed pre-key
    async fn store_signed_prekey(&self, id: u32, record: &[u8]) -> Result<()> {
        let record = record.to_vec();
        let device_id = self.device_id;
        self.with_write_txn(SIGNED_PREKEYS_TABLE, move |table| {
            table
                .insert((id, device_id), record.as_slice())
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Load a signed pre-key by ID
    async fn load_signed_prekey(&self, id: u32) -> Result<Option<Vec<u8>>> {
        self.with_read_txn(SIGNED_PREKEYS_TABLE, |table| {
            match table
                .get((id, self.device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                Some(data) => Ok(Some(data.value().to_vec())),
                None => Ok(None),
            }
        })
    }

    /// Load all signed pre-keys (returns id, record pairs)
    async fn load_all_signed_prekeys(&self) -> Result<Vec<(u32, Vec<u8>)>> {
        self.with_read_txn(SIGNED_PREKEYS_TABLE, |table| {
            let mut result: Vec<(u32, Vec<u8>)> = Vec::new();

            let iter = table
                .range::<(u32, u8)>(..)
                .map_err(|e| StoreError::Database(Box::new(e)))?;

            for item in iter {
                let (key_access, value_access) =
                    item.map_err(|e| StoreError::Database(Box::new(e)))?;
                let (id, db_device_id) = key_access.value();

                if db_device_id == self.device_id {
                    result.push((id, value_access.value().to_vec()));
                }
            }

            Ok(result)
        })
    }

    /// Remove a signed pre-key
    async fn remove_signed_prekey(&self, id: u32) -> Result<()> {
        let device_id = self.device_id;
        self.with_write_txn(SIGNED_PREKEYS_TABLE, move |table| {
            table
                .remove((id, device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Store a sender key for group messaging
    async fn put_sender_key(&self, address: &str, record: &[u8]) -> Result<()> {
        let address = address.to_string();
        let record = record.to_vec();
        let device_id = self.device_id;
        self.with_write_txn(SENDER_KEYS_TABLE, move |table| {
            table
                .insert((address.as_str(), device_id), record.as_slice())
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Get a sender key
    async fn get_sender_key(&self, address: &str) -> Result<Option<Vec<u8>>> {
        self.with_read_txn(SENDER_KEYS_TABLE, |table| {
            match table
                .get((address, self.device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                Some(data) => Ok(Some(data.value().to_vec())),
                None => Ok(None),
            }
        })
    }

    /// Delete a sender key
    async fn delete_sender_key(&self, address: &str) -> Result<()> {
        let address = address.to_string();
        let device_id = self.device_id;
        self.with_write_txn(SENDER_KEYS_TABLE, move |table| {
            table
                .remove((address.as_str(), device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }
}
