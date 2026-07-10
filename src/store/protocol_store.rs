use super::{
    BASE_KEYS_TABLE, BaseKeyRecord, DEVICE_REGISTRY_TABLE, GROUP_METADATA_TABLE,
    LID_PN_MAPPING_TABLE, RedbStore, SENDER_KEY_DEVICES_TABLE, SENT_MESSAGES_TABLE,
    SentMessageRecord, TC_TOKENS_TABLE,
};
use async_trait::async_trait;
use redb::ReadableTable;
use std::collections::HashSet;
use whatsapp_rust::{
    store::{
        DeviceListRecord, LidPnMappingEntry, ProtocolStore, TcTokenEntry,
        error::{Result, StoreError},
    },
    wacore,
};

#[async_trait]
impl ProtocolStore for RedbStore {
    // --- Per-Device Sender Key Tracking (matches WA Web's participant.senderKey Map) ---

    /// Get the sender key distribution status for all known devices in a group.
    /// Returns `(device_jid_string, has_key)` pairs where `has_key` indicates
    /// whether the device has a valid sender key (`true`) or needs fresh SKDM (`false`).
    async fn get_sender_key_devices(&self, group_jid: &str) -> Result<Vec<(String, bool)>> {
        self.with_read_txn(SENDER_KEY_DEVICES_TABLE, |table| {
            let mut results = Vec::new();
            for result in table
                .range::<(&str, u8, &str)>(..)
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                let (k, v) = result.map_err(|e| StoreError::Database(Box::new(e)))?;
                let (db_group, db_device_id, db_device_jid) = k.value();

                if db_group == group_jid && db_device_id == self.device_id {
                    let has_key = v.value() == 1;
                    results.push((db_device_jid.to_string(), has_key));
                }
            }
            Ok(results)
        })
    }

    /// Set sender key status for devices. Called with `has_key=true` after successful
    /// SKDM distribution (WA Web: `markHasSenderKey`), or `has_key=false` to mark
    /// devices as needing fresh SKDM (WA Web: `markForgetSenderKey`).
    async fn set_sender_key_status(&self, group_jid: &str, entries: &[(&str, bool)]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }
        let device_id = self.device_id;
        let group_jid = group_jid.to_string();

        let entries: Vec<(String, bool)> = entries
            .iter()
            .map(|&(jid, has_key)| (jid.to_string(), has_key))
            .collect();

        self.with_write_txn(SENDER_KEY_DEVICES_TABLE, move |table| {
            for (device_jid, has_key) in entries {
                let val = if has_key { 1 } else { 0 };
                table
                    .insert((group_jid.as_str(), device_id, device_jid.as_str()), val)
                    .map_err(|e| StoreError::Database(Box::new(e)))?;
            }
            Ok(())
        })
        .await
    }

    /// Clear all sender key device tracking for a group (on sender key rotation).
    async fn clear_sender_key_devices(&self, group_jid: &str) -> Result<()> {
        let group_jid = group_jid.to_string();
        let device_id = self.device_id;

        self.with_write_txn(SENDER_KEY_DEVICES_TABLE, move |table| {
            let mut to_remove = Vec::new();
            for result in table
                .range::<(&str, u8, &str)>(..)
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                let (k, _) = result.map_err(|e| StoreError::Database(Box::new(e)))?;
                let (db_group, db_device_id, db_device_jid) = k.value();

                if db_group == group_jid && db_device_id == device_id {
                    to_remove.push(db_device_jid.to_string());
                }
            }
            for device_jid in to_remove {
                table
                    .remove((group_jid.as_str(), device_id, device_jid.as_str()))
                    .map_err(|e| StoreError::Database(Box::new(e)))?;
            }
            Ok(())
        })
        .await
    }

    /// Delete specific `sender_key_devices` rows by device JID across all groups.
    /// Mirrors WA Web's per-group `senderKey.delete(deviceJid)` cleanup.
    async fn delete_sender_key_device_rows(&self, device_jids: &[&str]) -> Result<()> {
        if device_jids.is_empty() {
            return Ok(());
        }

        let target_jids: HashSet<String> = device_jids.iter().map(|s| s.to_string()).collect();
        let device_id = self.device_id;

        self.with_write_txn(SENDER_KEY_DEVICES_TABLE, move |table| {
            let mut to_remove = Vec::new();

            for result in table
                .range::<(&str, u8, &str)>(..)
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                let (k, _) = result.map_err(|e| StoreError::Database(Box::new(e)))?;
                let (db_group, db_device_id, db_device_jid) = k.value();

                if db_device_id == device_id && target_jids.contains(db_device_jid) {
                    to_remove.push((db_group.to_string(), db_device_jid.to_string()));
                }
            }
            for (g_jid, d_jid) in to_remove {
                table
                    .remove((g_jid.as_str(), device_id, d_jid.as_str()))
                    .map_err(|e| StoreError::Database(Box::new(e)))?;
            }
            Ok(())
        })
        .await
    }

    /// Clear all sender key device tracking across ALL groups.
    /// Called on identity change (raw_id mismatch) to force SKDM redistribution.
    async fn clear_all_sender_key_devices(&self) -> Result<()> {
        let device_id = self.device_id;

        self.with_write_txn(SENDER_KEY_DEVICES_TABLE, move |table| {
            let mut to_remove = Vec::new();
            for result in table
                .range::<(&str, u8, &str)>(..)
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                let (k, _) = result.map_err(|e| StoreError::Database(Box::new(e)))?;
                let (db_group, db_device_id, db_device_jid) = k.value();

                if db_device_id == device_id {
                    to_remove.push((db_group.to_string(), db_device_jid.to_string()));
                }
            }
            for (g_jid, d_jid) in to_remove {
                table
                    .remove((g_jid.as_str(), device_id, d_jid.as_str()))
                    .map_err(|e| StoreError::Database(Box::new(e)))?;
            }
            Ok(())
        })
        .await
    }

    // --- LID-PN Mapping ---

    /// Get a mapping by LID.
    async fn get_lid_mapping(&self, lid: &str) -> Result<Option<LidPnMappingEntry>> {
        self.with_read_txn(LID_PN_MAPPING_TABLE, |table| {
            if let Some(data) = table
                .get((lid, self.device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                let decoded: LidPnMappingEntry = self.decode(data.value())?;
                Ok(Some(decoded))
            } else {
                Ok(None)
            }
        })
    }

    /// Get a mapping by phone number (returns the most recent LID for that phone).
    async fn get_pn_mapping(&self, phone: &str) -> Result<Option<LidPnMappingEntry>> {
        self.with_read_txn(LID_PN_MAPPING_TABLE, |table| {
            for result in table
                .range::<(&str, u8)>(..)
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                let (k, v) = result.map_err(|e| StoreError::Database(Box::new(e)))?;
                let (_, db_device_id) = k.value();

                if db_device_id == self.device_id {
                    let decoded: LidPnMappingEntry = self.decode(v.value())?;
                    if decoded.phone_number == phone {
                        return Ok(Some(decoded));
                    }
                }
            }
            Ok(None)
        })
    }

    /// Store or update a LID-PN mapping.
    async fn put_lid_mapping(&self, entry: &LidPnMappingEntry) -> Result<()> {
        let encoded = self.encode(entry)?;
        let device_id = self.device_id;
        let lid = entry.lid.to_string();

        self.with_write_txn(LID_PN_MAPPING_TABLE, move |table| {
            table
                .insert((lid.as_str(), device_id), encoded.as_slice())
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Batched variant of `put_lid_mapping`. Backends should override with a
    /// single transaction; the default loops for correctness. Mirrors WA Web's
    /// `WAWebDBCreateLidPnMappings.createLidPnMappings({ mappings, … })`.
    async fn put_lid_mappings(&self, entries: &[LidPnMappingEntry]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut prepared_entries = Vec::with_capacity(entries.len());
        for entry in entries {
            let encoded = self.encode(entry)?;
            prepared_entries.push((entry.lid.to_string(), encoded));
        }

        let device_id = self.device_id;

        self.with_write_txn(LID_PN_MAPPING_TABLE, move |table| {
            for (lid, encoded) in prepared_entries {
                table
                    .insert((lid.as_str(), device_id), encoded.as_slice())
                    .map_err(|e| StoreError::Database(Box::new(e)))?;
            }
            Ok(())
        })
        .await
    }

    /// Get all LID-PN mappings (for cache warm-up).
    async fn get_all_lid_mappings(&self) -> Result<Vec<LidPnMappingEntry>> {
        self.with_read_txn(LID_PN_MAPPING_TABLE, |table| {
            let mut results = Vec::new();
            for result in table
                .range::<(&str, u8)>(..)
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                let (k, v) = result.map_err(|e| StoreError::Database(Box::new(e)))?;
                let (_, db_device_id) = k.value();

                if db_device_id == self.device_id {
                    let decoded: LidPnMappingEntry = self.decode(v.value())?;
                    results.push(decoded);
                }
            }
            Ok(results)
        })
    }

    // --- Base Key Collision Detection ---

    /// Save the base key for a session address during retry collision detection.
    async fn save_base_key(&self, address: &str, message_id: &str, base_key: &[u8]) -> Result<()> {
        let record = BaseKeyRecord {
            base_key: base_key.to_vec(),
            created_at: wacore::time::now_secs() as i32,
        };
        let encoded = self.encode(&record)?;
        let address = address.to_string();
        let device_id = self.device_id;
        let message_id = message_id.to_string();

        self.with_write_txn(BASE_KEYS_TABLE, move |table| {
            table
                .insert(
                    (address.as_str(), message_id.as_str(), device_id),
                    encoded.as_slice(),
                )
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Check if the current session has the same base key as the saved one.
    async fn has_same_base_key(
        &self,
        address: &str,
        message_id: &str,
        current_base_key: &[u8],
    ) -> Result<bool> {
        self.with_read_txn(BASE_KEYS_TABLE, |table| {
            if let Some(data) = table
                .get((address, message_id, self.device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                let decoded: BaseKeyRecord = self.decode(data.value())?;
                Ok(decoded.base_key == current_base_key)
            } else {
                Ok(false)
            }
        })
    }

    /// Delete a base key entry.
    async fn delete_base_key(&self, address: &str, message_id: &str) -> Result<()> {
        let address = address.to_string();
        let message_id = message_id.to_string();
        let device_id = self.device_id;

        self.with_write_txn(BASE_KEYS_TABLE, move |table| {
            table
                .remove((address.as_str(), message_id.as_str(), device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    // --- Device Registry ---

    /// Update the device list for a user (called after usync responses).
    async fn update_device_list(&self, record: DeviceListRecord) -> Result<()> {
        let encoded = self.encode(&record)?;
        let device_id = self.device_id;

        self.with_write_txn(DEVICE_REGISTRY_TABLE, move |table| {
            table
                .insert((record.user.as_str(), device_id), encoded.as_slice())
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Batched variant of `update_device_list`. Backends should override with
    /// a single transaction; the default loops for correctness. Important on
    /// usync of large groups, where the per-row commit + spawn_blocking
    /// overhead dominates wall-clock time when called once per participant.
    async fn update_device_lists(&self, records: Vec<DeviceListRecord>) -> Result<()> {
        // for record in records {
        //     self.update_device_list(record).await?;
        // }
        // Ok(())
        if records.is_empty() {
            return Ok(());
        }
        let device_id = self.device_id;
        let this = self.clone();

        self.with_write_txn(DEVICE_REGISTRY_TABLE, move |table| {
            for record in records {
                let encoded = this.encode(&record)?;
                table
                    .insert((record.user.as_str(), device_id), encoded.as_slice())
                    .map_err(|e| StoreError::Database(Box::new(e)))?;
            }
            Ok(())
        })
        .await
    }

    /// Get all known devices for a user.
    async fn get_devices(&self, user: &str) -> Result<Option<DeviceListRecord>> {
        self.with_read_txn(DEVICE_REGISTRY_TABLE, |table| {
            if let Some(data) = table
                .get((user, self.device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                let decoded: DeviceListRecord = self.decode(data.value())?;
                Ok(Some(decoded))
            } else {
                Ok(None)
            }
        })
    }

    /// Delete a device list record, forcing a network re-fetch on next query.
    async fn delete_devices(&self, user: &str) -> Result<()> {
        let user = user.to_string();
        let device_id = self.device_id;

        self.with_write_txn(DEVICE_REGISTRY_TABLE, move |table| {
            table
                .remove((user.as_str(), device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    // --- Group Metadata Cache (WA Web participant-phash re-query skip) ---

    /// Get the persisted, opaque serialized group metadata blob for `group_jid`.
    /// The blob is a caller-serialized GroupInfo snapshot; backends without group
    /// persistence return `None` (the group is then re-queried in full).
    async fn get_group_metadata(&self, group_jid: &str) -> Result<Option<Vec<u8>>> {
        self.with_read_txn(GROUP_METADATA_TABLE, |table| {
            if let Some(data) = table
                .get((group_jid, self.device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                Ok(Some(data.value().to_vec()))
            } else {
                Ok(None)
            }
        })
    }

    /// Persist (upsert) the serialized group metadata blob for `group_jid`.
    /// No-op by default; backends override to enable the phash re-query skip.
    async fn put_group_metadata(&self, group_jid: &str, blob: &[u8]) -> Result<()> {
        let device_id = self.device_id;
        let group_jid = group_jid.to_string();
        let blob = blob.to_vec();

        self.with_write_txn(GROUP_METADATA_TABLE, move |table| {
            table
                .insert((group_jid.as_str(), device_id), blob.as_slice())
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Remove the persisted group metadata blob for `group_jid` (e.g. on leave),
    /// so the next query re-fetches in full instead of comparing a stale phash.
    /// No-op by default.
    async fn delete_group_metadata(&self, group_jid: &str) -> Result<()> {
        let device_id = self.device_id;
        let group_jid = group_jid.to_string();

        self.with_write_txn(GROUP_METADATA_TABLE, move |table| {
            table
                .remove((group_jid.as_str(), device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    // --- TcToken Storage ---

    /// Get a trusted contact token for a JID (stored under LID).
    async fn get_tc_token(&self, jid: &str) -> Result<Option<TcTokenEntry>> {
        self.with_read_txn(TC_TOKENS_TABLE, |table| {
            if let Some(data) = table
                .get((jid, self.device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                let decoded: TcTokenEntry = self.decode(data.value())?;
                Ok(Some(decoded))
            } else {
                Ok(None)
            }
        })
    }

    /// Store or update a trusted contact token for a JID.
    async fn put_tc_token(&self, jid: &str, entry: &TcTokenEntry) -> Result<()> {
        let encoded = self.encode(entry)?;
        let jid = jid.to_string();
        let device_id = self.device_id;

        self.with_write_txn(TC_TOKENS_TABLE, move |table| {
            table
                .insert((jid.as_str(), device_id), encoded.as_slice())
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Delete a trusted contact token for a JID.
    async fn delete_tc_token(&self, jid: &str) -> Result<()> {
        let jid = jid.to_string();
        let device_id = self.device_id;

        self.with_write_txn(TC_TOKENS_TABLE, move |table| {
            table
                .remove((jid.as_str(), device_id))
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Get all JIDs that have stored tc tokens.
    async fn get_all_tc_token_jids(&self) -> Result<Vec<String>> {
        self.with_read_txn(TC_TOKENS_TABLE, |table| {
            let mut jids = Vec::new();
            for result in table
                .range::<(&str, u8)>(..)
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                let (k, _) = result.map_err(|e| StoreError::Database(Box::new(e)))?;
                let (db_jid, db_device_id) = k.value();

                if db_device_id == self.device_id {
                    jids.push(db_jid.to_string());
                }
            }
            Ok(jids)
        })
    }

    /// Delete tc tokens that have no live state left. A row is removed only when
    /// its received token is expired-or-absent (`token_timestamp < token_cutoff`
    /// or empty) **and** its sender bucket is expired-or-absent
    /// (`sender_timestamp < sender_cutoff` or null), so recent sender state is
    /// never dropped just because the received token expired. Returns count deleted.
    async fn delete_expired_tc_tokens(&self, token_cutoff: i64, sender_cutoff: i64) -> Result<u32> {
        let device_id = self.device_id;

        self.with_write_txn(TC_TOKENS_TABLE, move |table| {
            let mut to_remove = Vec::new();

            for result in table
                .range::<(&str, u8)>(..)
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                let (k, v) = result.map_err(|e| StoreError::Database(Box::new(e)))?;
                let (jid, db_device_id) = k.value();

                if db_device_id != device_id {
                    continue;
                }

                let entry: TcTokenEntry = serde_json::from_slice(v.value())
                    .map_err(|e| StoreError::Database(Box::new(e)))?;

                let token_expired = entry.token_timestamp < token_cutoff;

                let sender_expired = entry
                    .sender_timestamp
                    .map(|ts| ts < sender_cutoff)
                    .unwrap_or(true);

                if token_expired && sender_expired {
                    to_remove.push(jid.to_owned());
                }
            }

            let deleted = to_remove.len() as u32;

            for jid in to_remove {
                table
                    .remove((jid.as_str(), device_id))
                    .map_err(|e| StoreError::Database(Box::new(e)))?;
            }

            Ok(deleted)
        })
        .await
    }

    // --- Sent Message Store (retry support, matches WA Web's getMessageTable) ---

    /// Store a sent message's serialized payload for retry handling.
    /// Called after each send_message(); the payload is the protobuf-encoded Message.
    async fn store_sent_message(
        &self,
        chat_jid: &str,
        message_id: &str,
        payload: &[u8],
    ) -> Result<()> {
        let record = SentMessageRecord {
            payload: payload.to_vec(),
            created_at: wacore::time::now_secs() as i64,
        };
        let encoded = self.encode(&record)?;
        let device_id = self.device_id;
        let chat_jid = chat_jid.to_string();
        let message_id = message_id.to_string();

        self.with_write_txn(SENT_MESSAGES_TABLE, move |table| {
            table
                .insert(
                    (chat_jid.as_str(), message_id.as_str(), device_id),
                    encoded.as_slice(),
                )
                .map_err(|e| StoreError::Database(Box::new(e)))?;
            Ok(())
        })
        .await
    }

    /// Retrieve and delete a sent message (atomic take). Returns serialized payload.
    /// Called when a retry receipt arrives; consuming prevents double-retry.
    async fn take_sent_message(&self, chat_jid: &str, message_id: &str) -> Result<Option<Vec<u8>>> {
        let chat_jid = chat_jid.to_string();
        let message_id = message_id.to_string();
        let device_id = self.device_id;
        let this = self.clone();

        self.with_write_txn(SENT_MESSAGES_TABLE, move |table| {
            let mut record_opt = None;
            {
                if let Some(data) = table
                    .get((chat_jid.as_str(), message_id.as_str(), device_id))
                    .map_err(|e| StoreError::Database(Box::new(e)))?
                {
                    let decoded: SentMessageRecord = this.decode(data.value())?;
                    record_opt = Some(decoded.payload);
                }
            }

            if let Some(payload) = record_opt {
                table
                    .remove((chat_jid.as_str(), message_id.as_str(), device_id))
                    .map_err(|e| StoreError::Database(Box::new(e)))?;
                Ok(Some(payload))
            } else {
                Ok(None)
            }
        })
        .await
    }

    /// Delete sent messages older than cutoff (unix timestamp seconds). Returns count deleted.
    async fn delete_expired_sent_messages(&self, cutoff_timestamp: i64) -> Result<u32> {
        let device_id = self.device_id;
        let this = self.clone();

        self.with_write_txn(SENT_MESSAGES_TABLE, move |table| {
            let mut to_remove = Vec::new();
            for result in table
                .range::<(&str, &str, u8)>(..)
                .map_err(|e| StoreError::Database(Box::new(e)))?
            {
                let (k, v) = result.map_err(|e| StoreError::Database(Box::new(e)))?;
                let (db_chat, db_msg_id, db_device_id) = k.value();

                if db_device_id == device_id {
                    let decoded: SentMessageRecord = this.decode(v.value())?;
                    if decoded.created_at < cutoff_timestamp {
                        to_remove.push((db_chat.to_string(), db_msg_id.to_string()));
                    }
                }
            }

            let count = to_remove.len() as u32;
            for (chat, msg_id) in to_remove {
                table
                    .remove((chat.as_str(), msg_id.as_str(), device_id))
                    .map_err(|e| StoreError::Database(Box::new(e)))?;
            }
            Ok(count)
        })
        .await
    }
}
