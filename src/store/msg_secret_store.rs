use crate::store::redb_store::{MSG_SECRETS_TABLE, MsgSecretRecord, RedbStore};
use async_trait::async_trait;
use redb::ReadableTable;
use whatsapp_rust::{
    store::{
        MsgSecretEntry, MsgSecretStore,
        error::{Result, StoreError},
    },
    wacore,
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
        secret: &[u8],
    ) -> Result<()> {
        self.put_msg_secrets(vec![MsgSecretEntry {
            chat: chat.to_string(),
            sender: sender.to_string(),
            msg_id: msg_id.to_string(),
            secret: secret.to_vec(),
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

        let now = wacore::time::now_secs();
        let device_id = self.device_id;

        let write_txn = self
            .connection
            .begin_write()
            .map_err(|e| StoreError::Database(Box::new(e)))?;

        {
            let mut table = write_txn
                .open_table(MSG_SECRETS_TABLE)
                .map_err(|e| StoreError::Database(Box::new(e)))?;

            for entry in &entries {
                let key = (
                    entry.chat.as_str(),
                    entry.sender.as_str(),
                    entry.msg_id.as_str(),
                    device_id,
                );

                let record = match table
                    .get(key)
                    .map_err(|e| StoreError::Database(Box::new(e)))?
                {
                    Some(existing) => {
                        let mut record: MsgSecretRecord = self.decode(existing.value())?;

                        record.secret = entry.secret.clone();
                        record.created_at = now;

                        // expires_at logic sama seperti SQLite
                        record.expires_at = match (record.expires_at, entry.expires_at) {
                            (0, _) | (_, 0) => 0,
                            (a, b) => a.max(b),
                        };

                        record.message_ts = record.message_ts.max(entry.message_ts);

                        record
                    }
                    None => MsgSecretRecord {
                        secret: entry.secret.clone(),
                        created_at: now,
                        expires_at: entry.expires_at,
                        message_ts: entry.message_ts,
                    },
                };

                let encoded = self.encode(&record)?;

                table
                    .insert(key, encoded.as_slice())
                    .map_err(|e| StoreError::Database(Box::new(e)))?;
            }
        }

        write_txn
            .commit()
            .map_err(|e| StoreError::Database(Box::new(e)))?;

        Ok(entries.len())
    }

    /// Fetch the persisted secret; returns `None` if absent.
    async fn get_msg_secret(
        &self,
        chat: &str,
        sender: &str,
        msg_id: &str,
    ) -> Result<Option<Vec<u8>>> {
        let device_id = self.device_id;

        self.with_read_txn(MSG_SECRETS_TABLE, |table| {
            let key = (chat, sender, msg_id, device_id);

            let value = table
                .get(key)
                .map_err(|e| StoreError::Database(Box::new(e)))?;

            match value {
                Some(v) => {
                    let record: MsgSecretRecord = self.decode(v.value())?;
                    Ok(Some(record.secret))
                }
                None => Ok(None),
            }
        })
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

        self.with_read_txn(MSG_SECRETS_TABLE, |table| {
            let key = (chat, sender, msg_id, device_id);

            let value = table
                .get(key)
                .map_err(|e| StoreError::Database(Box::new(e)))?;

            match value {
                Some(v) => {
                    let record: MsgSecretRecord = self.decode(v.value())?;

                    Ok(Some((record.secret, record.message_ts)))
                }
                None => Ok(None),
            }
        })
    }

    /// Delete rows whose non-zero `expires_at` is at or before
    /// `cutoff_timestamp` (absolute unix seconds; callers pass "now"). Rows
    /// with `expires_at = 0` (never) are kept. Returns the number removed so
    /// the keepalive cleanup can log/throttle.
    async fn delete_expired_msg_secrets(&self, cutoff_timestamp: i64) -> Result<u32> {
        let device_id = self.device_id;

        let write_txn = self
            .connection
            .begin_write()
            .map_err(|e| StoreError::Database(Box::new(e)))?;

        let mut deleted = 0u32;

        let mut to_delete: Vec<(String, String, String, u8)> = Vec::new();

        {
            let table = write_txn
                .open_table(MSG_SECRETS_TABLE)
                .map_err(|e| StoreError::Database(Box::new(e)))?;

            let iter = table
                .iter()
                .map_err(|e| StoreError::Database(Box::new(e)))?;

            for item in iter {
                let (key, value) = item.map_err(|e| StoreError::Database(Box::new(e)))?;

                let (chat, sender, msg_id, dev) = key.value();

                if dev != device_id {
                    continue;
                }

                let record: MsgSecretRecord = self.decode(value.value())?;

                // expires_at == 0 => never expire
                if record.expires_at != 0 && record.expires_at <= cutoff_timestamp {
                    to_delete.push((chat.to_owned(), sender.to_owned(), msg_id.to_owned(), dev));
                }
            }
        }

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
    }
}
