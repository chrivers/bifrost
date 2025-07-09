use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use hue::clamp::Clamp;
use hue::effect_duration::EffectDuration;
use hue::zigbee::{GradientParams, GradientStyle, HueZigbeeUpdate};
use tokio::time::sleep;
use uuid::Uuid;

use bifrost_api::backend::BackendRequest;
use hue::api::{
    ColorTemperatureUpdate, Entertainment, EntertainmentConfiguration, GroupedLight,
    GroupedLightUpdate, Light, LightEffectsV2Update, LightGradientMode, LightUpdate, RType,
    Resource, ResourceLink, Room, RoomUpdate, Scene, SceneActive, SceneStatus, SceneStatusEnum,
    SceneUpdate, ZigbeeDeviceDiscoveryUpdate,
};
use hue::error::HueError;
use hue::stream::HueStreamLightsV2;
use z2m::api::DeviceRead;
use z2m::update::{DeviceEffect, DeviceUpdate};

use crate::backend::z2m::Z2mBackend;
use crate::backend::z2m::entertainment::EntStream;
use crate::backend::z2m::websocket::Z2mWebSocket;
use crate::error::ApiResult;
use crate::model::state::AuxData;

impl Z2mBackend {
    #[allow(clippy::match_same_arms)]
    fn make_hue_specific_update(upd: &LightUpdate) -> ApiResult<HueZigbeeUpdate> {
        let mut hz = HueZigbeeUpdate::new();

        if let Some(on) = &upd.on {
            hz = hz.with_on_off(on.on);
        }

        if let Some(br) = &upd.dimming {
            hz = hz.with_brightness((br.brightness / 100.0).unit_to_u8_clamped_light());
        }

        if let Some(ColorTemperatureUpdate { mirek: Some(mirek) }) = upd.color_temperature {
            hz = hz.with_color_mirek(mirek);
        }

        if let Some(xy) = &upd.color {
            hz = hz.with_color_xy(xy.xy);
        }

        if let Some(grad) = &upd.gradient {
            hz = hz.with_gradient_colors(
                grad.mode.map_or(GradientStyle::Linear, Into::into),
                grad.points.iter().map(|c| c.color.xy).collect(),
            )?;

            hz = hz.with_gradient_params(GradientParams {
                scale: match grad.mode {
                    Some(LightGradientMode::InterpolatedPalette) => 0x28,
                    Some(LightGradientMode::InterpolatedPaletteMirrored) => 0x18,
                    Some(LightGradientMode::RandomPixelated) => 0x38,
                    None => 0x18,
                },
                offset: 0x00,
            });
        }

        if let Some(LightEffectsV2Update {
            action: Some(act), ..
        }) = &upd.effects_v2
        {
            if let Some(fx) = act.effect {
                hz = hz.with_effect_type(fx.into());
            }
            if let Some(speed) = &act.parameters.speed {
                hz = hz.with_effect_speed(speed.unit_to_u8_clamped());
            }
            if let Some(mirek) = &act.parameters.color_temperature.and_then(|ct| ct.mirek) {
                hz = hz.with_color_mirek(*mirek);
            }
            if let Some(color) = &act.parameters.color {
                hz = hz.with_color_xy(color.xy);
            }
        }

        if let Some(act) = &upd.timed_effects {
            if let Some(fx) = act.effect {
                hz = hz.with_effect_type(fx.into());
            }

            if let Some(duration) = act.duration {
                hz = hz.with_effect_duration(EffectDuration::from_ms(duration)?);
            }
        }

        Ok(hz)
    }

    async fn backend_light_update(
        &self,
        z2mws: &mut Z2mWebSocket,
        link: &ResourceLink,
        upd: &LightUpdate,
    ) -> ApiResult<()> {
        let Some(topic) = self.rmap.get(link) else {
            return Ok(());
        };

        let mut lock = self.state.lock().await;

        // We cannot recover .mode from backend updates, since these only contain
        // the gradient colors. So we have no choice, but to update the mode
        // here. Otherwise, the information would be lost.
        if let Some(mode) = upd.gradient.as_ref().and_then(|gr| gr.mode) {
            lock.update::<Light>(&link.rid, |light| {
                if let Some(gr) = &mut light.gradient {
                    gr.mode = mode;
                }
            })?;
        }
        let hue_effects = lock.get::<Light>(link)?.effects.is_some();
        drop(lock);

        let mut payload: Option<DeviceUpdate> = None;

        // handle "identify" request (light breathing)
        if upd.identify.is_some() {
            // handle "identify" request (light breathing)

            payload = Some(
                payload
                    .unwrap_or_default()
                    .with_effect(DeviceEffect::Breathe),
            );

            let tx = self.message_tx.clone();
            let topic = topic.clone();

            // spawn task to stop effect after a few seconds
            let _job = tokio::spawn(async move {
                sleep(Self::LIGHT_BREATHE_DURATION).await;

                let upd = DeviceUpdate::new().with_effect(DeviceEffect::FinishEffect);
                tx.send((topic, upd))
            });
        }

        if !hue_effects {
            /* send generic light update */
            let transition = upd
                .dynamics
                .as_ref()
                .and_then(|d| d.duration.map(|duration| f64::from(duration) / 1000.0))
                .or_else(|| {
                    if upd.dimming.is_some()
                        || upd.color_temperature.is_some()
                        || upd.color.is_some()
                    {
                        Some(0.4)
                    } else {
                        None
                    }
                });
            payload = Some(
                payload
                    .unwrap_or_default()
                    .with_state(upd.on.map(|on| on.on))
                    .with_brightness(upd.dimming.map(|dim| dim.brightness / 100.0 * 254.0))
                    .with_color_temp(upd.color_temperature.and_then(|ct| ct.mirek))
                    .with_color_xy(upd.color.map(|col| col.xy))
                    .with_transition(transition)
                    // We don't want to send gradient updates twice, but if hue
                    // effects are not supported for this light, this is the best
                    // (and only) way to do it
                    .with_gradient(upd.gradient.clone()),
            );
        };

        if let Some(payload) = payload {
            z2mws.send_update(topic, &payload).await?;
        }

        /* if supported send hue-specific effects update */
        if hue_effects {
            let mut hz = Self::make_hue_specific_update(upd)?;

            if !hz.is_empty() {
                hz = hz.with_fade_speed(0x0001);

                let read_payload = DeviceRead::default()
                    .with_state(
                        hz.onoff.is_some() || hz.brightness.is_some() || hz.effect_type.is_some(),
                    )
                    .with_color(
                        hz.color_mirek.is_some()
                            || hz.color_xy.is_some()
                            || hz.effect_type.is_some(),
                    )
                    .with_brightness(hz.brightness.is_some() || hz.effect_type.is_some());

                z2mws.send_hue_effects(topic, hz).await?;

                // Do an explicit attribute read since Hue specific updates do not automatically update z2m state
                z2mws.send_read(&topic, &read_payload).await?;
            }
        }

        Ok(())
    }

    async fn backend_scene_create(
        &self,
        z2mws: &mut Z2mWebSocket,
        link_scene: &ResourceLink,
        sid: u32,
        scene: &Scene,
    ) -> ApiResult<()> {
        let Some(topic) = self.rmap.get(&scene.group) else {
            return Ok(());
        };

        log::info!("New scene: {link_scene:?} ({})", scene.metadata.name);

        let mut lock = self.state.lock().await;

        let auxdata = AuxData::new()
            .with_topic(&scene.metadata.name)
            .with_index(sid);

        lock.aux_set(link_scene, auxdata);

        z2mws
            .send_scene_store(topic, &scene.metadata.name, sid)
            .await?;

        lock.add(link_scene, Resource::Scene(scene.clone()))?;
        drop(lock);

        Ok(())
    }

    async fn backend_scene_update(
        &mut self,
        z2mws: &mut Z2mWebSocket,
        link: &ResourceLink,
        upd: &SceneUpdate,
    ) -> ApiResult<()> {
        let mut lock = self.state.lock().await;

        let scene = lock.get::<Scene>(link)?;

        let index = lock
            .aux_get(link)?
            .index
            .ok_or(HueError::NotFound(link.rid))?;

        if let Some(recall) = &upd.recall {
            if recall.action == Some(SceneStatusEnum::Active) {
                let scenes = lock.get_scenes_for_room(&scene.group.rid);
                for rid in scenes {
                    lock.update::<Scene>(&rid, |scn| {
                        scn.status = Some(SceneStatus {
                            active: if rid == link.rid {
                                SceneActive::Static
                            } else {
                                SceneActive::Inactive
                            },
                            last_recall: None,
                        });
                    })?;
                }

                let room = lock.get::<Scene>(link)?.group;
                drop(lock);

                if let Some(topic) = self.rmap.get(&room).cloned() {
                    log::info!("[{}] Recall scene: {link:?}", self.name);

                    let mut lock = self.state.lock().await;
                    self.learner.learn_scene_recall(link, &mut lock)?;

                    z2mws.send_scene_recall(&topic, index).await?;
                }
            } else {
                log::error!("Scene recall type not supported: {recall:?}");
            }
        } else {
            // We're not recalling the scene, so we are updating the scene
            let room = lock.get::<Scene>(link)?.group;

            if let Some(topic) = self.rmap.get(&room).cloned() {
                log::info!("[{}] Store scene: {link:?}", self.name);

                let scene = lock.get::<Scene>(link)?;
                z2mws
                    .send_scene_store(&topic, &scene.metadata.name, index)
                    .await?;

                // We have requested z2m to update the scene, so update
                // the state database accordingly
                lock.update::<Scene>(&link.rid, |scene| {
                    *scene += upd;
                })?;

                drop(lock);
            }
        }

        Ok(())
    }

    async fn backend_grouped_light_update(
        &self,
        z2mws: &mut Z2mWebSocket,
        link: &ResourceLink,
        upd: &GroupedLightUpdate,
    ) -> ApiResult<()> {
        let room = self.state.lock().await.get::<GroupedLight>(link)?.owner;

        if let Some(topic) = self.rmap.get(&room) {
            z2mws.send_update(topic, &upd.into()).await?;
        }

        Ok(())
    }

    async fn backend_room_update(
        &self,
        z2mws: &mut Z2mWebSocket,
        link: &ResourceLink,
        upd: &RoomUpdate,
    ) -> ApiResult<()> {
        let lock = self.state.lock().await;

        if let Some(children) = &upd.children {
            if let Some(topic) = self.rmap.get(link) {
                let room = lock.get::<Room>(link)?.clone();
                drop(lock);

                let known_existing: BTreeSet<_> = room
                    .children
                    .iter()
                    .filter(|device| self.rmap.contains_key(device))
                    .collect();

                let known_new: BTreeSet<_> = children
                    .iter()
                    .filter(|device| self.rmap.contains_key(device))
                    .collect();

                for add in known_new.difference(&known_existing) {
                    let friendly_name = &self.rmap[add];
                    z2mws.send_group_member_add(topic, friendly_name).await?;
                }

                for remove in known_existing.difference(&known_new) {
                    let friendly_name = &self.rmap[remove];
                    z2mws.send_group_member_remove(topic, friendly_name).await?;
                }
            }
        }

        Ok(())
    }

    async fn backend_delete(&self, z2mws: &mut Z2mWebSocket, link: &ResourceLink) -> ApiResult<()> {
        match link.rtype {
            RType::Scene => {
                let lock = self.state.lock().await;
                let room = lock.get::<Scene>(link)?.group;
                let index = lock
                    .aux_get(link)?
                    .index
                    .ok_or(HueError::NotFound(link.rid))?;
                drop(lock);

                if let Some(topic) = self.rmap.get(&room) {
                    z2mws.send_scene_remove(topic, index).await?;
                }
            }

            RType::Device => {
                if let Some(dev) = self
                    .rmap
                    .get(link)
                    .and_then(|topic| self.network.get(topic))
                {
                    let addr = dev.ieee_address.to_string();
                    log::info!(
                        "[{}] Requesting z2m removal of {} ({})",
                        self.name,
                        &addr,
                        dev.friendly_name
                    );

                    z2mws.send_device_remove(addr).await?;
                }
            }

            rtype => {
                log::warn!(
                    "[{}] Deleting objects of type {rtype:?} is not supported",
                    self.name
                );
            }
        }
        Ok(())
    }

    async fn backend_entertainment_start(
        &mut self,
        z2mws: &mut Z2mWebSocket,
        ent_id: &Uuid,
    ) -> ApiResult<()> {
        log::trace!("[{}] Entertainment start", self.name);
        let lock = self.state.lock().await;

        let ent: &EntertainmentConfiguration = lock.get_id(*ent_id)?;

        let mut chans = ent.channels.clone();

        let mut addrs: BTreeMap<String, Vec<u16>> = BTreeMap::new();
        let mut targets = vec![];
        chans.sort_by_key(|c| c.channel_id);

        log::trace!("[{}] Resolving entertainment channels", self.name);
        for chan in chans {
            for member in &chan.members {
                let ent: &Entertainment = lock.get(&member.service)?;
                let light_id = ent
                    .renderer_reference
                    .ok_or(HueError::NotFound(member.service.rid))?;
                let topic = self
                    .rmap
                    .get(&light_id)
                    .ok_or(HueError::NotFound(light_id.rid))?;
                let dev = self
                    .network
                    .get(topic)
                    .ok_or(HueError::NotFound(member.service.rid))?;

                let segment_addr = dev.network_address + member.index;

                addrs
                    .entry(dev.friendly_name.clone())
                    .or_default()
                    .push(segment_addr);

                targets.push(topic);
            }
        }
        log::debug!("Entertainment addresses: {addrs:04x?}");
        drop(lock);

        if let Some(target) = targets.first() {
            let mut es = EntStream::new(self.counter, target, addrs);

            // Not even a real Philips Hue bridge uses this trick!
            //
            // We set the entertainment mode fade speed ("smoothing")
            // to fit the target frame rate, to ensure perfectly smooth
            // transitionss, even at low frame rates!
            es.stream.set_smoothing_duration(self.throttle.interval())?;

            log::info!("Starting entertainment mode stream at {} fps", self.fps);

            es.start_stream(z2mws).await?;

            self.entstream = Some(es);
        }

        Ok(())
    }

    async fn backend_entertainment_frame(
        &mut self,
        z2mws: &mut Z2mWebSocket,
        frame: &HueStreamLightsV2,
    ) -> ApiResult<()> {
        if let Some(es) = &mut self.entstream {
            if self.throttle.tick() {
                es.frame(z2mws, frame).await?;
            }
        }

        Ok(())
    }

    async fn backend_entertainment_stop(&mut self, z2mws: &mut Z2mWebSocket) -> ApiResult<()> {
        log::debug!("Stopping entertainment mode..");
        if let Some(es) = &mut self.entstream.take() {
            let mut lock = self.state.lock().await;

            es.stop_stream(z2mws).await?;

            self.counter = es.stream.counter();

            for id in lock.get_resource_ids_by_type(RType::Light) {
                let light: &Light = lock.get_id(id)?;
                if light.is_streaming() {
                    lock.update(&id, Light::stop_streaming)?;
                }
            }

            for id in lock.get_resource_ids_by_type(RType::EntertainmentConfiguration) {
                let ec: &EntertainmentConfiguration = lock.get_id(id)?;
                if ec.is_streaming() {
                    lock.update(&id, EntertainmentConfiguration::stop_streaming)?;
                }
            }
            drop(lock);
        }

        Ok(())
    }

    async fn backend_zigbee_device_discovery(
        &self,
        z2mws: &mut Z2mWebSocket,
        _rlink: &ResourceLink,
        _zbd: &ZigbeeDeviceDiscoveryUpdate,
    ) -> ApiResult<()> {
        z2mws.send_permit_join(60 * 4, None).await
    }

    pub async fn handle_backend_event(
        &mut self,
        z2mws: &mut Z2mWebSocket,
        req: Arc<BackendRequest>,
    ) -> ApiResult<()> {
        self.learner.cleanup();

        match &*req {
            BackendRequest::LightUpdate(link, upd) => {
                self.backend_light_update(z2mws, link, upd).await
            }

            BackendRequest::SceneCreate(link, sid, scene) => {
                self.backend_scene_create(z2mws, link, *sid, scene).await
            }

            BackendRequest::SceneUpdate(link, upd) => {
                self.backend_scene_update(z2mws, link, upd).await
            }

            BackendRequest::GroupedLightUpdate(link, upd) => {
                self.backend_grouped_light_update(z2mws, link, upd).await
            }

            BackendRequest::RoomUpdate(link, upd) => {
                self.backend_room_update(z2mws, link, upd).await
            }

            BackendRequest::Delete(link) => self.backend_delete(z2mws, link).await,

            BackendRequest::EntertainmentStart(ent_id) => {
                self.backend_entertainment_start(z2mws, ent_id).await
            }

            BackendRequest::EntertainmentFrame(frame) => {
                self.backend_entertainment_frame(z2mws, frame).await
            }

            BackendRequest::EntertainmentStop() => self.backend_entertainment_stop(z2mws).await,

            BackendRequest::ZigbeeDeviceDiscovery(rlink, zbd) => {
                self.backend_zigbee_device_discovery(z2mws, rlink, zbd)
                    .await
            }
        }
    }
}
