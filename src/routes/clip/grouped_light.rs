use axum::{
    extract::{Path, State},
    routing::put,
    Json, Router,
};
use serde_json::Value;
use uuid::Uuid;

use crate::hue::api::{GroupedLight, GroupedLightUpdate, RType, V2Reply};
use crate::routes::clip::ApiV2Result;
use crate::state::AppState;
use crate::z2m::request::ClientRequest;
use crate::z2m::update::DeviceUpdate;

async fn put_grouped_light(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(put): Json<Value>,
) -> ApiV2Result {
    log::info!("PUT grouped_light/{id}");
    log::debug!("json data\n{}", serde_json::to_string_pretty(&put)?);

    let rlink = RType::GroupedLight.link_to(id);
    let lock = state.res.lock().await;
    lock.get::<GroupedLight>(&rlink)?;

    log::info!("PUT grouped_light/{id}: updating");

    let upd: GroupedLightUpdate = serde_json::from_value(put)?;

    let payload = DeviceUpdate::default()
        .with_state(upd.on.map(|on| on.on))
        .with_brightness(upd.dimming.map(|dim| dim.brightness / 100.0 * 254.0))
        .with_color_temp(upd.color_temperature.map(|ct| ct.mirek))
        .with_color_xy(upd.color.map(|col| col.xy));

    lock.z2m_request(ClientRequest::group_update(rlink, payload))?;

    drop(lock);

    V2Reply::ok(rlink)
}

pub fn router() -> Router<AppState> {
    Router::new().route("/:id", put(put_grouped_light))
}
