use hue::error::HueError;
use serde_json::Value;
use uuid::{Uuid, uuid};

use hue::api::{
    Bridge, Device, Entertainment, EntertainmentConfiguration, EntertainmentConfigurationAction,
    EntertainmentConfigurationChannels, EntertainmentConfigurationLocations,
    EntertainmentConfigurationNew, EntertainmentConfigurationServiceLocations,
    EntertainmentConfigurationStatus, EntertainmentConfigurationStreamMembers,
    EntertainmentConfigurationStreamProxy, EntertainmentConfigurationStreamProxyMode,
    EntertainmentConfigurationStreamProxyUpdate, EntertainmentConfigurationUpdate, Light,
    LightMode, Position, RType, Resource, ResourceLink,
};

use crate::error::ApiResult;
use crate::resource::Resources;
use crate::routes::auth::STANDARD_APPLICATION_ID;
use crate::routes::clip::{ApiV2Result, V2Reply};
use crate::server::appstate::AppState;

// FIXME: These are hard-coded values fitting for an LCX005 gradient light chain
pub const POSITIONS: &[Position] = &[
    Position {
        x: -0.4,
        y: 0.8,
        z: -0.4,
    },
    Position {
        x: -0.4,
        y: 0.8,
        z: 0.4,
    },
    Position {
        x: -0.22,
        y: 0.8,
        z: 0.4,
    },
    Position {
        x: 0.0,
        y: 0.8,
        z: 0.4,
    },
    Position {
        x: 0.22,
        y: 0.8,
        z: 0.4,
    },
    Position {
        x: 0.4,
        y: 0.8,
        z: 0.4,
    },
    Position {
        x: 0.4,
        y: 0.8,
        z: -0.4,
    },
];

pub async fn post_resource(state: &AppState, req: Value) -> ApiV2Result {
    let new: EntertainmentConfigurationNew = serde_json::from_value(req)?;

    let mut lock = state.res.lock().await;

    let locations = EntertainmentConfigurationLocations {
        service_locations: new
            .locations
            .service_locations
            .into_iter()
            .map(Into::into)
            .collect(),
    };

    let channels = make_channels(&lock, &locations.service_locations)?;
    let light_services = make_services(&lock, &locations.service_locations)?;

    let auto_node = find_bridge_entertainment(&lock)?;

    let obj = Resource::EntertainmentConfiguration(EntertainmentConfiguration {
        name: new.metadata.name.clone(),
        configuration_type: new.configuration_type,
        metadata: new.metadata,
        status: EntertainmentConfigurationStatus::Inactive,
        stream_proxy: new.stream_proxy.map_or_else(
            || EntertainmentConfigurationStreamProxy {
                mode: EntertainmentConfigurationStreamProxyMode::Auto,
                node: auto_node,
            },
            |sp| match sp {
                EntertainmentConfigurationStreamProxyUpdate::Auto => {
                    EntertainmentConfigurationStreamProxy {
                        mode: EntertainmentConfigurationStreamProxyMode::Auto,
                        node: auto_node,
                    }
                }
                EntertainmentConfigurationStreamProxyUpdate::Manual { node } => {
                    EntertainmentConfigurationStreamProxy {
                        mode: EntertainmentConfigurationStreamProxyMode::Manual,
                        node,
                    }
                }
            },
        ),
        channels,
        locations,
        light_services,
        active_streamer: None,
    });

    let rlink = ResourceLink::new(Uuid::new_v4(), obj.rtype());
    lock.add(&rlink, obj)?;
    drop(lock);

    V2Reply::ok(rlink)
}

fn make_channels(
    lock: &Resources,
    locations: &[EntertainmentConfigurationServiceLocations],
) -> ApiResult<Vec<EntertainmentConfigurationChannels>> {
    let mut channels: Vec<EntertainmentConfigurationChannels> = vec![];

    let mut channel_id = 0;
    for location in locations {
        let Some(pos) = location.positions.first() else {
            continue;
        };
        let ent: &Entertainment = lock.get(&location.service)?;

        if let Some(segs) = &ent.segments {
            if segs.segments.len() > 1 {
                for index in 0..segs.segments.len() {
                    channels.push(EntertainmentConfigurationChannels {
                        channel_id,
                        position: POSITIONS[index % POSITIONS.len()].clone(),
                        members: vec![EntertainmentConfigurationStreamMembers {
                            service: location.service,
                            index: u16::try_from(index)?,
                        }],
                    });
                    channel_id += 1;
                }
                continue;
            }
        }

        channels.push(EntertainmentConfigurationChannels {
            channel_id,
            position: pos.clone(),
            members: vec![EntertainmentConfigurationStreamMembers {
                service: location.service,
                index: 0,
            }],
        });
        channel_id += 1;
    }

    Ok(channels)
}

fn make_services(
    lock: &Resources,
    locations: &[EntertainmentConfigurationServiceLocations],
) -> ApiResult<Vec<ResourceLink>> {
    let mut res = vec![];

    for location in locations {
        let ent: &Entertainment = lock.get(&location.service)?;
        if let Some(ren) = ent.renderer_reference {
            res.push(ren);
        }
    }

    Ok(res)
}

fn find_bridge_entertainment(lock: &Resources) -> ApiResult<ResourceLink> {
    let bridge_id = lock.get_resource_ids_by_type(RType::Bridge)[0];

    let bridge: &Bridge = lock.get_id(bridge_id)?;

    let bridge_dev: &Device = lock.get_id(bridge.owner.rid)?;

    let bridge_ent = bridge_dev
        .services
        .iter()
        .find(|obj| obj.rtype == RType::Entertainment)
        .copied()
        .ok_or(HueError::NotFound(bridge_id))?;

    Ok(bridge_ent)
}

pub async fn put_resource_id(state: &AppState, rlink: ResourceLink, put: Value) -> ApiV2Result {
    let upd: EntertainmentConfigurationUpdate = serde_json::from_value(put)?;

    let mut lock = state.res.lock().await;

    let mut locations = None;
    let mut channels = vec![];
    let mut light_services = vec![];

    if let Some(locs) = &upd.locations {
        let newlocs = EntertainmentConfigurationLocations {
            service_locations: locs
                .service_locations
                .clone()
                .into_iter()
                .map(Into::into)
                .collect(),
        };
        channels = make_channels(&lock, &newlocs.service_locations)?;
        light_services = make_services(&lock, &newlocs.service_locations)?;
        locations = Some(newlocs);
    }

    if let Some(action) = &upd.action {
        let ent: &EntertainmentConfiguration = lock.get(&rlink)?;
        let svc = ent.light_services.clone();

        lock.update::<Light>(&svc[0].rid, |light| {
            light.mode = match action {
                EntertainmentConfigurationAction::Start => LightMode::Streaming,
                EntertainmentConfigurationAction::Stop => LightMode::Normal,
            }
        })?;
    }

    let bridge_ent = find_bridge_entertainment(&lock)?;

    lock.update::<EntertainmentConfiguration>(&rlink.rid, |ec| {
        if let Some(_locations) = upd.locations {
            ec.locations = locations.unwrap();
            ec.channels = channels;
            ec.light_services = light_services;
        }

        if let Some(proxy) = upd.stream_proxy {
            match proxy {
                EntertainmentConfigurationStreamProxyUpdate::Auto => {
                    ec.stream_proxy.mode = EntertainmentConfigurationStreamProxyMode::Auto;
                    ec.stream_proxy.node = bridge_ent;
                }
                EntertainmentConfigurationStreamProxyUpdate::Manual { node } => {
                    ec.stream_proxy.mode = EntertainmentConfigurationStreamProxyMode::Manual;
                    ec.stream_proxy.node = node;
                }
            }
        }

        if let Some(ctype) = upd.configuration_type {
            ec.configuration_type = ctype;
        }

        if let Some(md) = upd.metadata {
            ec.metadata = md;
        }

        if let Some(action) = upd.action {
            match action {
                EntertainmentConfigurationAction::Start => {
                    ec.active_streamer =
                        Some(RType::AuthV1.link_to(uuid!(STANDARD_APPLICATION_ID)));
                    ec.status = EntertainmentConfigurationStatus::Active;
                }
                EntertainmentConfigurationAction::Stop => {
                    ec.active_streamer = None;
                    ec.status = EntertainmentConfigurationStatus::Inactive;
                }
            }
        }
    })?;

    drop(lock);

    V2Reply::ok(rlink)
}
