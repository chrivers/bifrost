use serde::Deserialize;
use serde_json::Value;
use tokio_tungstenite::tungstenite;
use uuid::Uuid;

use hue::api::{DimmingUpdate, GroupedLight, Light, LightUpdate, RType, Resource, Room};
use z2m::api::{
    BridgeDevices, DeviceRemoveResponse, GroupMemberChange, Message, RawMessage, Response,
};
use z2m::update::DeviceUpdate;

use crate::backend::z2m::Z2mBackend;
use crate::error::{ApiError, ApiResult};

impl Z2mBackend {
    async fn handle_update_light(&mut self, uuid: &Uuid, devupd: &DeviceUpdate) -> ApiResult<()> {
        let upd: LightUpdate = devupd.into();

        let mut lock = self.state.lock().await;
        lock.update::<Light>(uuid, |light| *light += &upd)?;

        self.learner.learn(uuid, &lock, devupd)?;
        self.learner.collect(&mut lock)?;
        drop(lock);

        Ok(())
    }

    async fn handle_update_grouped_light(&self, uuid: &Uuid, upd: &DeviceUpdate) -> ApiResult<()> {
        let mut res = self.state.lock().await;
        res.update::<GroupedLight>(uuid, |glight| {
            if let Some(state) = &upd.state {
                glight.on = Some((*state).into());
            }

            if let Some(b) = upd.brightness {
                glight.dimming = Some(DimmingUpdate {
                    brightness: b / 254.0 * 100.0,
                });
            }
        })
    }

    async fn handle_update(&mut self, rid: &Uuid, payload: &Value) -> ApiResult<()> {
        if let Value::String(string) = payload {
            if string.is_empty() {
                log::debug!("Ignoring empty payload for {rid}");
                return Ok(());
            }
        }

        let upd = DeviceUpdate::deserialize(payload)?;

        let obj = self.state.lock().await.get_resource_by_id(rid)?.obj;
        match obj {
            Resource::Light(_) => {
                if let Err(e) = self.handle_update_light(rid, &upd).await {
                    log::error!("FAIL: {e:?} in {upd:?}");
                }
            }
            Resource::GroupedLight(_) => {
                if let Err(e) = self.handle_update_grouped_light(rid, &upd).await {
                    log::error!("FAIL: {e:?} in {upd:?}");
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_device_message(&mut self, msg: RawMessage) -> ApiResult<()> {
        if msg.topic.ends_with("/availability") || msg.topic.ends_with("/action") {
            // availability: https://www.zigbee2mqtt.io/guide/usage/mqtt_topics_and_messages.html#zigbee2mqtt-friendly-name-availability
            // action: https://www.home-assistant.io/integrations/device_trigger.mqtt/
            return Ok(());
        }

        let Some(ref val) = self.map.get(&msg.topic).copied() else {
            if !self.ignore.contains(&msg.topic) {
                log::warn!(
                    "[{}] Notification on unknown topic {}",
                    self.name,
                    &msg.topic
                );
            }
            return Ok(());
        };

        let res = self.handle_update(&val.rid, &msg.payload).await;
        if let Err(ref err) = res {
            log::error!(
                "Cannot parse update: {err}\n{}",
                serde_json::to_string_pretty(&msg.payload)?
            );
        }

        /* return Ok here, since we do not want to break the event loop */
        Ok(())
    }

    async fn bridge_devices(&mut self, devices: &BridgeDevices) -> ApiResult<()> {
        for dev in devices {
            self.network.insert(dev.friendly_name.clone(), dev.clone());
            if let Some(exp) = dev.expose_light() {
                log::info!(
                    "[{}] Adding light {:?}: [{}] ({})",
                    self.name,
                    dev.ieee_address,
                    dev.friendly_name,
                    dev.model_id.as_deref().unwrap_or("<unknown model>")
                );
                self.add_light(dev, exp).await?;
            } else {
                log::debug!(
                    "[{}] Ignoring unsupported device {}",
                    self.name,
                    dev.friendly_name
                );
                self.ignore.insert(dev.friendly_name.to_string());
            }
            /*
            if dev.expose_action() {
                log::info!(
                    "[{}] Adding switch {:?}: [{}] ({})",
                    self.name,
                    dev.ieee_address,
                    dev.friendly_name,
                    dev.model_id.as_deref().unwrap_or("<unknown model>")
                );
                self.add_switch(dev).await?;
            }
            */
        }

        Ok(())
    }

    async fn bridge_device_remove(&mut self, data: &DeviceRemoveResponse) -> ApiResult<()> {
        if let Some(rlink) = self.map.get(&data.id) {
            match rlink.rtype {
                RType::Light => {
                    let mut lock = self.state.lock().await;
                    let owner = lock.get::<Light>(rlink)?.owner;
                    log::info!("Removing device: {owner:?}");
                    lock.delete(&owner)?;
                }
                rtype => {
                    log::warn!("Cannot handle removing resource of type {rtype:?}");
                }
            }
        }

        if let Some(_rlink) = self.map.remove(&data.id) {
            self.rmap.retain(|_, v| *v != data.id);
        }

        Ok(())
    }

    #[allow(clippy::collapsible_else_if)]
    async fn bridge_group_member_change(
        &self,
        change: &GroupMemberChange,
        added: bool,
    ) -> ApiResult<()> {
        if let Some(light) = self.map.get(&change.device) {
            let mut lock = self.state.lock().await;
            let device = lock.get::<Light>(light)?.clone();

            let device_link = device.owner;
            if let Some(room) = self.map.get(&change.group) {
                let room_link = lock.get::<GroupedLight>(room)?.owner;
                let exists = lock
                    .get::<Room>(&room_link)?
                    .children
                    .contains(&device_link);

                if added {
                    if !exists {
                        lock.update(&room_link.rid, |room: &mut Room| {
                            room.children.insert(device_link);
                        })?;
                    }
                } else {
                    if exists {
                        lock.update(&room_link.rid, |room: &mut Room| {
                            room.children.remove(&device_link);
                        })?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_bridge_message(&mut self, msg: Message) -> ApiResult<()> {
        #[allow(unused_variables)]
        match &msg {
            Message::BridgeInfo(obj) => { /* println!("{obj:#?}"); */ }
            Message::BridgeHealth(_obj) => { /* println!("{obj:#?}"); */ }
            Message::BridgeLogging(obj) => { /* println!("{obj:#?}"); */ }
            Message::BridgeExtensions(obj) => { /* println!("{obj:#?}"); */ }
            Message::BridgeEvent(obj) => { /* println!("{obj:#?}"); */ }
            Message::BridgeDefinitions(obj) => { /* println!("{obj:#?}"); */ }
            Message::BridgeState(obj) => { /* println!("{obj:#?}"); */ }
            Message::BridgeConverters(obj) => { /* println!("{obj:#?}"); */ }
            Message::BridgeOptions(obj) => { /* println!("{obj:#?}"); */ }
            Message::BridgePermitJoin(obj) => {}
            Message::BridgeTouchlinkScan(obj) => {}
            Message::BridgeDeviceOptions(obj) => {}
            Message::BridgeNetworkmap(obj) => {}
            Message::BridgeDeviceOtaUpdateCheck(obj) => {}
            Message::BridgeDeviceConfigureReporting(obj) => {}
            Message::BridgeConfig(obj) => {}
            Message::BridgeResponseGroupAdd(obj) => {}
            Message::BridgeResponseGroupRemove(obj) => {}
            Message::BridgeResponseGroupRename(obj) => {}
            Message::BridgeResponseGroupOptions(obj) => {}

            Message::BridgeDevices(obj) => {
                self.bridge_devices(obj).await?;
            }

            Message::BridgeGroups(obj) => {
                /* println!("{obj:#?}"); */
                for grp in obj {
                    self.add_group(grp).await?;
                }
            }

            Message::BridgeGroupMembersAdd(change) | Message::BridgeGroupMembersRemove(change) => {
                let Response::Ok { data: change, .. } = change else {
                    log::warn!("[{}] Error reported from z2m: {change:?}", self.name);
                    return Ok(());
                };

                let added = matches!(msg, Message::BridgeGroupMembersAdd(_));
                self.bridge_group_member_change(change, added).await?;
            }

            Message::BridgeDeviceRemove(obj) => {
                let Response::Ok { data, .. } = obj else {
                    log::warn!("[{}] Error reported from z2m: {obj:?}", self.name);
                    return Ok(());
                };

                self.bridge_device_remove(data).await?;
            }
        }
        Ok(())
    }

    pub async fn handle_bridge_event(&mut self, pkt: tungstenite::Message) -> ApiResult<()> {
        let tungstenite::Message::Text(txt) = pkt else {
            log::error!("[{}] Received non-text message on websocket :(", self.name);
            return Err(ApiError::UnexpectedZ2mReply(pkt));
        };

        let raw_msg = serde_json::from_str::<RawMessage>(&txt);

        log::trace!("[{}] Incoming z2m message: {txt}", self.name);

        let msg = raw_msg.map_err(|err| {
            log::error!(
                "[{}] Invalid websocket message: {:#?} [{}..]",
                self.name,
                err,
                &txt.chars().take(128).collect::<String>()
            );
            err
        })?;

        /* bridge messages are handled differently. everything else is a device message */
        if !msg.topic.starts_with("bridge/") {
            return self.handle_device_message(msg).await;
        }

        match serde_json::from_str(&txt) {
            Ok(bridge_msg) => self.handle_bridge_message(bridge_msg).await,
            Err(err) => {
                match msg.topic.as_str() {
                    topic @ ("bridge/devices" | "bridge/groups") => {
                        log::error!(
                            "[{}] Failed to parse critical z2m bridge message on [{}]:",
                            self.name,
                            topic,
                        );
                        log::error!("[{}] {}", self.name, serde_json::to_string(&msg.payload)?);
                        Err(err)?
                    }
                    topic => {
                        log::error!(
                            "[{}] Failed to parse (non-critical) z2m bridge message on [{}]:",
                            self.name,
                            topic
                        );
                        log::error!("{}", serde_json::to_string(&msg.payload)?);

                        /* Suppress this non-critical error, to avoid breaking the event loop */
                        Ok(())
                    }
                }
            }
        }
    }
}
