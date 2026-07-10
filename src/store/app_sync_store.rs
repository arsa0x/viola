use super::{
    APP_STATE_KEYS_TABLE, APP_STATE_MUTATION_MACS_TABLE, APP_STATE_VERSIONS_TABLE,
    AppStateMutationMacRecord, RedbStore,
};
use async_trait::async_trait;
use redb::ReadableTable;
use whatsapp_rust::{
    store::{
        AppStateSyncKey, AppSyncStore,
        error::{Result, StoreError},
    },
    wacore::appstate::{hash::HashState, processor::AppStateMutationMAC},
};

#[async_trait]
impl AppSyncStore for RedbStore {
    /// Get an app state sync key by ID.
    async fn get_sync_key(&self, key_id: &[u8]) -> Result<Option<AppStateSyncKey>> {
        self.with_read_txn(APP_STATE_KEYS_TABLE, |table| {
            match table
                .get((key_id, self.device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                Some(data) => {
                    let decoded = self.decode(data.value())?;
                    Ok(Some(decoded))
                }
                None => Ok(None),
            }
        })
    }

    /// Set an app state sync key.
    async fn set_sync_key(&self, key_id: &[u8], key: AppStateSyncKey) -> Result<()> {
        let encoded = self.encode(&key)?;
        let device_id = self.device_id;
        let key_id = key_id.to_vec();

        self.with_write_txn(APP_STATE_KEYS_TABLE, move |table| {
            table
                .insert((key_id.as_slice(), device_id), encoded.as_slice())
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Get the app state version for a collection.
    async fn get_version(&self, name: &str) -> Result<HashState> {
        self.with_read_txn(APP_STATE_VERSIONS_TABLE, |table| {
            match table
                .get((name, self.device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                Some(data) => {
                    let decoded = self.decode(data.value())?;
                    Ok(decoded)
                }
                None => Ok(HashState::default()),
            }
        })
    }

    /// Set the app state version for a collection.
    async fn set_version(&self, name: &str, state: HashState) -> Result<()> {
        let encoded = self.encode(&state)?;
        let name = name.to_string();
        let device_id = self.device_id;

        self.with_write_txn(APP_STATE_VERSIONS_TABLE, move |table| {
            table
                .insert((name.as_str(), device_id), encoded.as_slice())
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Store mutation MACs for a version.
    async fn put_mutation_macs(
        &self,
        name: &str,
        version: u64,
        mutations: &[AppStateMutationMAC],
    ) -> Result<()> {
        if mutations.is_empty() {
            return Ok(());
        }
        let name = name.to_string();
        let device_id = self.device_id;
        let this = self.clone();
        let mutations = mutations.to_vec();

        self.with_write_txn(APP_STATE_MUTATION_MACS_TABLE, move |table| {
            for m in mutations {
                let record = AppStateMutationMacRecord {
                    version,
                    value_mac: m.value_mac.clone(),
                };
                let encoded = this.encode(&record)?;
                table
                    .insert(
                        (name.as_str(), device_id, m.index_mac.as_slice()),
                        encoded.as_slice(),
                    )
                    .map_err(|e| StoreError::Database(Box::new(e)))?;
            }
            Ok(())
        })
        .await
    }

    /// Get a mutation MAC by index.
    async fn get_mutation_mac(&self, name: &str, index_mac: &[u8]) -> Result<Option<Vec<u8>>> {
        self.with_read_txn(APP_STATE_MUTATION_MACS_TABLE, |table| {
            match table
                .get((name, self.device_id, index_mac))
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                Some(data) => {
                    let decoded: AppStateMutationMacRecord = self.decode(data.value())?;
                    Ok(Some(decoded.value_mac))
                }
                None => Ok(None),
            }
        })
    }

    /// Batch variant of [`get_mutation_mac`]: fetch many previous-MAC values in a
    /// single backend round-trip. The default delegates to per-item lookups;
    /// backends with a set-membership query (SQL `IN (...)`) should override to
    /// avoid an N+1 (one DB round-trip per mutation in appstate sync).
    ///
    /// Index MACs are full HMAC-SHA256 outputs, so the batch path passes them as
    /// inline `[u8; 32]` arrays ([`crate::appstate_sync::IndexMac`]) — no per-MAC
    /// heap allocation on either side of the call.
    async fn get_mutation_macs(
        &self,
        name: &str,
        index_macs: &[[u8; 32]],
    ) -> Result<std::collections::HashMap<[u8; 32], Vec<u8>>> {
        // let mut out = std::collections::HashMap::with_capacity(index_macs.len());
        // for index_mac in index_macs {
        //     if let Some(mac) = self.get_mutation_mac(name, index_mac).await? {
        //         out.insert(index_mac.clone(), mac);
        //     }
        // }
        // Ok(out)
        self.with_read_txn(APP_STATE_MUTATION_MACS_TABLE, |table| {
            let mut out = std::collections::HashMap::with_capacity(index_macs.len());
            for index_mac in index_macs {
                if let Some(data) = table
                    .get((name, self.device_id, index_mac.as_slice()))
                    .map_err(|e| StoreError::Database(Box::new(e)))?
                {
                    let decoded: AppStateMutationMacRecord = self.decode(data.value())?;
                    out.insert(index_mac.clone(), decoded.value_mac);
                }
            }
            Ok(out)
        })
    }

    /// Delete mutation MACs by their index MACs.
    async fn delete_mutation_macs(&self, name: &str, index_macs: &[Vec<u8>]) -> Result<()> {
        if index_macs.is_empty() {
            return Ok(());
        }
        let index_macs = index_macs.to_vec();
        let device_id = self.device_id;
        let name = name.to_string();

        self.with_write_txn(APP_STATE_MUTATION_MACS_TABLE, move |table| {
            for index_mac in index_macs {
                table
                    .remove((name.as_str(), device_id, index_mac.as_slice()))
                    .map_err(|e| StoreError::Database(Box::new(e)))?;
            }
            Ok(())
        })
        .await
    }

    /// Delete every mutation MAC for a collection. Called on snapshot re-sync so the
    /// MAC store is rebuilt from the snapshot, matching the ltHash baseline; leftover
    /// entries would corrupt the next patch's ltHash.
    async fn clear_mutation_macs(&self, name: &str) -> Result<()> {
        let name = name.to_string();
        let device_id = self.device_id;

        self.with_write_txn(APP_STATE_MUTATION_MACS_TABLE, move |table| {
            let mut to_remove = Vec::new();

            for result in table
                .range::<(&str, u8, &[u8])>(..)
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                let (k, _) = result.map_err(|e| StoreError::Database(Box::new(e)))?;
                let (db_name, db_device_id, db_index_mac) = k.value();

                if db_name == name && db_device_id == device_id {
                    to_remove.push(db_index_mac.to_vec());
                }
            }

            for mac in to_remove {
                table
                    .remove((name.as_str(), device_id, mac.as_slice()))
                    .map_err(|e| StoreError::Database(Box::new(e)))?;
            }
            Ok(())
        })
        .await
    }

    /// Get the most recently stored app state sync key ID.
    async fn get_latest_sync_key_id(&self) -> Result<Option<Vec<u8>>> {
        self.with_read_txn(APP_STATE_KEYS_TABLE, |table| {
            for result in table
                .range::<(&[u8], u8)>(..)
                .map_err(|e| StoreError::Database(Box::new(e)))?
                .rev()
            {
                let (k, _) = result.map_err(|e| StoreError::Database(Box::new(e)))?;
                let (db_key_id, db_device_id) = k.value();

                if db_device_id == self.device_id {
                    return Ok(Some(db_key_id.to_vec()));
                }
            }
            Ok(None)
        })
    }
}
