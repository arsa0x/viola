mod app_sync_store;
mod device_store;
mod msg_secret_store;
mod protocol_store;
mod signal_store;

use bincode;
use redb::{ReadOnlyTable, ReadableDatabase, Table, TableDefinition};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use whatsapp_rust::store::error::{Result, StoreError};

#[derive(Clone)]
pub struct RedbStore {
    pub connection: Arc<redb::Database>,
    pub device_id: u8,
}

#[derive(Serialize, Deserialize)]
pub struct PreKeyRecord {
    pub key: Vec<u8>,
    pub uploaded: bool,
}

#[derive(Serialize, Deserialize)]
pub struct AppStateMutationMacRecord {
    pub version: u64,
    pub value_mac: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct BaseKeyRecord {
    pub base_key: Vec<u8>,
    pub created_at: i32,
}

#[derive(Serialize, Deserialize)]
pub struct SentMessageRecord {
    pub payload: Vec<u8>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgSecretRecord {
    pub secret: Vec<u8>,
    pub created_at: i64,
    pub expires_at: i64,
    pub message_ts: i64,
}

pub const MSG_SECRETS_TABLE: TableDefinition<(&str, &str, &str, u8), &[u8]> =
    TableDefinition::new("msg_secrets");

pub const IDENTITIES_TABLE: TableDefinition<(&str, u8), &[u8; 32]> =
    TableDefinition::new("identities");

pub const SESSIONS_TABLE: TableDefinition<(&str, u8), &[u8]> = TableDefinition::new("sessions");

pub const PREKEYS_TABLE: TableDefinition<(u32, u8), &[u8]> = TableDefinition::new("prekeys");

pub const SIGNED_PREKEYS_TABLE: TableDefinition<(u32, u8), &[u8]> =
    TableDefinition::new("signed_prekeys");

pub const SENDER_KEYS_TABLE: TableDefinition<(&str, u8), &[u8]> =
    TableDefinition::new("sender_keys");

pub const APP_STATE_KEYS_TABLE: TableDefinition<(&[u8], u8), &[u8]> =
    TableDefinition::new("app_state_keys");

pub const APP_STATE_VERSIONS_TABLE: TableDefinition<(&str, u8), &[u8]> =
    TableDefinition::new("app_state_versions");

pub const APP_STATE_MUTATION_MACS_TABLE: TableDefinition<(&str, u8, &[u8]), &[u8]> =
    TableDefinition::new("app_state_mutation_macs");

pub const SENDER_KEY_DEVICES_TABLE: TableDefinition<(&str, u8, &str), u32> =
    TableDefinition::new("sender_key_devices");

pub const LID_PN_MAPPING_TABLE: TableDefinition<(&str, u8), &[u8]> =
    TableDefinition::new("lid_pn_mapping");

pub const BASE_KEYS_TABLE: TableDefinition<(&str, &str, u8), &[u8]> =
    TableDefinition::new("base_keys");

pub const DEVICE_REGISTRY_TABLE: TableDefinition<(&str, u8), &[u8]> =
    TableDefinition::new("device_registry");

pub const GROUP_METADATA_TABLE: TableDefinition<(&str, u8), &[u8]> =
    TableDefinition::new("group_metadata");

pub const TC_TOKENS_TABLE: TableDefinition<(&str, u8), &[u8]> = TableDefinition::new("tc_tokens");

pub const SENT_MESSAGES_TABLE: TableDefinition<(&str, &str, u8), &[u8]> =
    TableDefinition::new("sent_messages");

pub const DEVICE_TABLE: TableDefinition<u32, &[u8]> = TableDefinition::new("device_store");

impl RedbStore {
    pub fn new(database_url: &str) -> Result<Self> {
        let connection =
            Arc::new(redb::Database::create(database_url).expect("failed to create database"));
        RedbStore::ensure_tables(&connection).expect("msg");
        Ok(Self {
            connection,
            device_id: 1,
        })
    }

    pub fn ensure_tables(db: &redb::Database) -> anyhow::Result<()> {
        let tx = db.begin_write()?;

        tx.open_table(DEVICE_TABLE)?;
        tx.open_table(IDENTITIES_TABLE)?;
        tx.open_table(SESSIONS_TABLE)?;
        tx.open_table(PREKEYS_TABLE)?;
        tx.open_table(SIGNED_PREKEYS_TABLE)?;
        tx.open_table(SENDER_KEYS_TABLE)?;
        tx.open_table(APP_STATE_KEYS_TABLE)?;
        tx.open_table(APP_STATE_VERSIONS_TABLE)?;
        tx.open_table(APP_STATE_MUTATION_MACS_TABLE)?;
        tx.open_table(SENDER_KEY_DEVICES_TABLE)?;
        tx.open_table(LID_PN_MAPPING_TABLE)?;
        tx.open_table(BASE_KEYS_TABLE)?;
        tx.open_table(DEVICE_REGISTRY_TABLE)?;
        tx.open_table(GROUP_METADATA_TABLE)?;
        tx.open_table(TC_TOKENS_TABLE)?;
        tx.open_table(SENT_MESSAGES_TABLE)?;
        tx.open_table(MSG_SECRETS_TABLE)?;

        tx.commit()?;
        Ok(())
    }

    pub fn new_with_device_id(database_url: &str, device_id: u8) -> Result<Self> {
        Ok(Self {
            connection: Arc::new(
                redb::Database::create(database_url).expect("failed to create database"),
            ),
            device_id,
        })
    }

    /// Executes a write operation on a single table within an atomic transaction.
    pub async fn with_write_txn<K, V, F, R>(
        &self,
        definition: TableDefinition<'static, K, V>,
        f: F,
    ) -> Result<R>
    where
        K: redb::Key + Send + Sync + 'static,
        V: redb::Value + Send + Sync + 'static,
        F: FnOnce(&mut Table<K, V>) -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        let connection = Arc::clone(&self.connection);
        tokio::task::spawn_blocking(move || {
            let write_txn = connection
                .begin_write()
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            let result = {
                let mut table = write_txn
                    .open_table(definition)
                    .map_err(|e| StoreError::Database(Box::new(e)))?;
                f(&mut table).map_err(|e| StoreError::Database(Box::new(e)))?
            };
            write_txn
                .commit()
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(result)
        })
        .await
        .map_err(|e| StoreError::Database(Box::new(e)))?
    }

    /// Executes a read operation on a single table within a read transaction.
    pub fn with_read_txn<K, V, F, R>(&self, definition: TableDefinition<K, V>, f: F) -> Result<R>
    where
        K: redb::Key + 'static,
        V: redb::Value + 'static,
        F: FnOnce(&ReadOnlyTable<K, V>) -> Result<R>,
    {
        let read_txn = self
            .connection
            .begin_read()
            .map_err(|e| StoreError::Database(Box::new(e)))?;

        let table = read_txn
            .open_table(definition)
            .map_err(|e| StoreError::Database(Box::new(e)))?;
        f(&table)
    }

    pub fn encode<T: Serialize>(&self, value: &T) -> Result<Vec<u8>> {
        bincode::serde::encode_to_vec(value, bincode::config::standard())
            .map_err(|e| StoreError::Database(Box::new(e)))
    }

    pub fn decode<T: serde::de::DeserializeOwned>(&self, bytes: &[u8]) -> Result<T> {
        let (decoded, _): (T, usize) =
            bincode::serde::decode_from_slice(bytes, bincode::config::standard())
                .map_err(|e| StoreError::Database(Box::new(e)))?;
        Ok(decoded)
    }
}
