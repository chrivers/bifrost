use mdns_sd::{ServiceDaemon, ServiceInfo};

use crate::{hue, state::AppState};

#[must_use]
pub fn register_mdns(appstate: &AppState) -> ServiceDaemon {
    /* Create a new mDNS daemon. */
    let mdns = ServiceDaemon::new().expect("Could not create service daemon");
    let service_type = "_hue._tcp.local.";
    let instance_name = "bifrost";

    /* With `enable_addr_auto()`, we can give empty addrs and let the lib find them. */
    /* If the caller knows specific addrs to use, then assign the addrs here. */
    let my_addrs = "";
    let service_hostname = format!("{instance_name}{service_type}");
    let port = 80;

    let properties = [
        ("modelid", hue::HUE_BRIDGE_V2_MODEL_ID),
        ("bridgeid", &appstate.bridge_id()),
    ];

    let service_info = ServiceInfo::new(
        service_type,
        instance_name,
        &service_hostname,
        my_addrs,
        port,
        &properties[..],
    )
    .expect("valid service info")
    .enable_addr_auto();

    mdns.register(service_info)
        .expect("Failed to register mDNS service");

    log::info!("Registered service {}.{}", &instance_name, &service_type);

    mdns
}
