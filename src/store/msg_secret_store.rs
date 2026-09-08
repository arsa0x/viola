use std::sync::Arc;

use super::{MSG_SECRETS_TABLE, MsgSecretRecord, RedbStore};
use redb::{ReadableDatabase, ReadableTable};
use whatsapp_rust::{
    async_trait,
    store::{
        MsgSecretEntry, MsgSecretStore,
        error::{Result, StoreError},
    },
    wacore::{self, reporting_token::MESSAGE_SECRET_SIZE},
};

#[async_trait]
impl MsgSecretStore for RedbStore {
    /// Persist `secret` (typically 32 bytes) under the composite key with NO
    /// expiry (`expires_at = 0`). Convenience wrapper over [`put_msg_secrets`].
    /// `chat`, `sender`, and `msg_id` are JID strings / message ID strings;
    /// callers should pass non-AD (no-device) form for the JIDs so lookups
    /// match regardless of which device echo'd the stanza back.
    ///
    /// Real call sites that compute a retention deadline build
    /// [`MsgSecretEntry`] directly and call [`put_msg_secrets`].
    ///
    /// [`put_msg_secrets`]: MsgSecretStore::put_msg_secrets
    async fn put_msg_secret(
        &self,
        chat: &str,
        sender: &str,
        msg_id: &str,
        secret: &[u8; MESSAGE_SECRET_SIZE],
    ) -> Result<()> {
        self.put_msg_secrets(vec![MsgSecretEntry {
            chat: Arc::from(chat),
            sender: Arc::from(sender),
            msg_id: Arc::from(msg_id),
            secret: *secret,
            expires_at: 0,
            message_ts: 0,
        }])
        .await?;
        Ok(())
    }

    /// Batched upsert carrying a per-row `expires_at` deadline. On key conflict
    /// implementations merge deterministically via [`merge_msg_secret_expiry`]
    /// (later deadline wins, `0` = "never" = infinity) so a redelivery or edit
    /// re-persist never shortens a window, and via [`merge_msg_secret_message_ts`]
    /// (the later non-zero parent time wins; a `0` never clobbers a known one).
    async fn put_msg_secrets(&self, entries: Vec<MsgSecretEntry>) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        let connection = Arc::clone(&self.connection);
        let device_id = self.device_id;

        tokio::task::spawn_blocking(move || {
            let now = wacore::time::now_secs();

            let write_txn = connection
                .begin_write()
                .map_err(|e| StoreError::Database(Box::new(e)))?;

            {
                let mut table = write_txn
                    .open_table(MSG_SECRETS_TABLE)
                    .map_err(|e| StoreError::Database(Box::new(e)))?;

                for entry in &entries {
                    let key = (
                        entry.chat.as_ref(),
                        entry.sender.as_ref(),
                        entry.msg_id.as_ref(),
                        device_id,
                    );

                    let record = match table
                        .get(key)
                        .map_err(|e| StoreError::Database(Box::new(e)))?
                    {
                        Some(existing) => {
                            let mut record: MsgSecretRecord = super::decode(existing.value())?;

                            record.secret = entry.secret.to_vec();

                            // expires_at logic sama seperti SQLite
                            record.expires_at = match (record.expires_at, entry.expires_at) {
                                (0, _) | (_, 0) => 0,
                                (a, b) => a.max(b),
                            };

                            record.message_ts = record.message_ts.max(entry.message_ts);

                            record
                        }
                        None => MsgSecretRecord {
                            secret: entry.secret.to_vec(),
                            created_at: now,
                            expires_at: entry.expires_at,
                            message_ts: entry.message_ts,
                        },
                    };
                    let encoded = super::encode(&record)?;
                    table
                        .insert(key, encoded.as_slice())
                        .map_err(|e| StoreError::Database(Box::new(e)))?;
                }
            }

            write_txn
                .commit()
                .map_err(|e| StoreError::Database(Box::new(e)))?;

            Ok(entries.len())
        })
        .await
        .map_err(|e| StoreError::Database(Box::new(e)))?
    }

    /// Fetch the persisted secret; returns `None` if absent.
    async fn get_msg_secret(
        &self,
        chat: &str,
        sender: &str,
        msg_id: &str,
    ) -> Result<Option<Vec<u8>>> {
        let device_id = self.device_id;

        let connection = Arc::clone(&self.connection);
        let (chat, sender, msg_id) = (chat.to_owned(), sender.to_owned(), msg_id.to_owned());

        tokio::task::spawn_blocking(move || {
            let read_txn = connection
                .begin_read()
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            let table = read_txn
                .open_table(MSG_SECRETS_TABLE)
                .map_err(|e| StoreError::Database(Box::new(e)))?;

            match table
                .get((chat.as_str(), sender.as_str(), msg_id.as_str(), device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                Some(v) => {
                    let record: MsgSecretRecord = super::decode(v.value())?;
                    Ok(Some(record.secret))
                }
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| StoreError::Database(Box::new(e)))?
    }

    /// Fetch the secret together with the parent message's event time
    /// (`message_ts`, `0` when unknown), so the receive path can enforce the
    /// edit-processing window. Default pairs `get_msg_secret` with `0`;
    /// backends that store `message_ts` override this.
    async fn get_msg_secret_with_ts(
        &self,
        chat: &str,
        sender: &str,
        msg_id: &str,
    ) -> Result<Option<(Vec<u8>, i64)>> {
        let device_id = self.device_id;
        let connection = Arc::clone(&self.connection);
        let (chat, sender, msg_id) = (chat.to_owned(), sender.to_owned(), msg_id.to_owned());

        tokio::task::spawn_blocking(move || {
            let read_txn = connection
                .begin_read()
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            let table = read_txn
                .open_table(MSG_SECRETS_TABLE)
                .map_err(|e| StoreError::Database(Box::new(e)))?;

            match table
                .get((chat.as_str(), sender.as_str(), msg_id.as_str(), device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                Some(v) => {
                    let record: MsgSecretRecord = super::decode(v.value())?;

                    Ok(Some((record.secret, record.message_ts)))
                }
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| StoreError::Database(Box::new(e)))?
    }

    /// Delete rows whose non-zero `expires_at` is at or before
    /// `cutoff_timestamp` (absolute unix seconds; callers pass "now"). Rows
    /// with `expires_at = 0` (never) are kept. Returns the number removed so
    /// the keepalive cleanup can log/throttle.
    async fn delete_expired_msg_secrets(&self, cutoff_timestamp: i64) -> Result<u32> {
        let device_id = self.device_id;
        let connection = Arc::clone(&self.connection);

        tokio::task::spawn_blocking(move || {
            let write_txn = connection
                .begin_write()
                .map_err(|e| StoreError::Database(Box::new(e)))?;

            let mut deleted = 0u32;

            let to_delete: Vec<(String, String, String, u8)> = {
                let table = write_txn
                    .open_table(MSG_SECRETS_TABLE)
                    .map_err(|e| StoreError::Database(Box::new(e)))?;

                let mut keys = Vec::new();
                for item in table
                    .iter()
                    .map_err(|e| StoreError::Database(Box::new(e)))?
                {
                    let (key, value) = item.map_err(|e| StoreError::Database(Box::new(e)))?;
                    let (chat, sender, msg_id, dev) = key.value();

                    if dev != device_id {
                        continue;
                    }

                    let record: MsgSecretRecord = super::decode(value.value())?;
                    if record.expires_at != 0 && record.expires_at <= cutoff_timestamp {
                        keys.push((chat.to_owned(), sender.to_owned(), msg_id.to_owned(), dev));
                    }
                }
                keys
            };

            if !to_delete.is_empty() {
                let mut table = write_txn
                    .open_table(MSG_SECRETS_TABLE)
                    .map_err(|e| StoreError::Database(Box::new(e)))?;

                for (chat, sender, msg_id, dev) in to_delete {
                    table
                        .remove((chat.as_str(), sender.as_str(), msg_id.as_str(), dev))
                        .map_err(|e| StoreError::Database(Box::new(e)))?;

                    deleted += 1;
                }
            }

            write_txn
                .commit()
                .map_err(|e| StoreError::Database(Box::new(e)))?;

            Ok(deleted)
        })
        .await
        .map_err(|e| StoreError::Database(Box::new(e)))?
    }
}
