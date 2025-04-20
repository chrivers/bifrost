use std::{iter, sync::Arc, time::Duration};

use chrono::{DateTime, Days, Local, NaiveTime, Timelike, Weekday};
use tokio::{spawn, sync::Mutex, task::JoinHandle, time::sleep};
use tokio_schedule::{every, Job};
use uuid::Uuid;

use hue::{
    api::{
        BehaviorInstance, BehaviorInstanceConfiguration, BehaviorInstanceUpdate,
        GroupedLightUpdate, Light, LightEffectActionUpdate, LightEffectsV2Update, LightUpdate, On,
        RType, Resource, ResourceLink, WakeupConfiguration,
    },
    clamp::Clamp,
    effect_duration::EffectDuration,
};

use crate::{backend::BackendRequest, error::ApiResult, resource::Resources};

#[derive(Debug)]
pub struct Scheduler {
    jobs: Vec<JoinHandle<()>>,
    res: Arc<Mutex<Resources>>,
    behavior_instances: Vec<ScheduleBehaviorInstance>,
}

impl Scheduler {
    pub const fn new(res: Arc<Mutex<Resources>>) -> Self {
        Self {
            jobs: vec![],
            behavior_instances: vec![],
            res,
        }
    }

    pub async fn update(&mut self) {
        let new_behavior_instances = self.get_behavior_instances().await;
        if new_behavior_instances != self.behavior_instances {
            self.behavior_instances = new_behavior_instances;
            self.update_jobs();
        }
    }

    fn update_jobs(&mut self) {
        for job in &self.jobs {
            job.abort();
        }
        self.jobs = self
            .behavior_instances
            .iter()
            .filter(|ScheduleBehaviorInstance(_, bi)| bi.enabled)
            .flat_map(|ScheduleBehaviorInstance(id, bi)| match &bi.configuration {
                BehaviorInstanceConfiguration::Wakeup(wakeup_configuration) => {
                    wakeup(self.res.clone(), id, wakeup_configuration)
                }
            })
            .collect();
    }

    async fn get_behavior_instances(&self) -> Vec<ScheduleBehaviorInstance> {
        self.res
            .lock()
            .await
            .get_resources_by_type(RType::BehaviorInstance)
            .into_iter()
            .filter_map(|r| match r.obj {
                Resource::BehaviorInstance(behavior_instance) => {
                    Some(ScheduleBehaviorInstance(r.id, behavior_instance))
                }
                _ => None,
            })
            .collect()
    }
}

fn wakeup(
    res: Arc<Mutex<Resources>>,
    id: &Uuid,
    wakeup_configuration: &WakeupConfiguration,
) -> Vec<JoinHandle<()>> {
    let jobs = create_wake_up_jobs(id, wakeup_configuration);
    jobs.into_iter()
        .map(move |job| spawn(job.create(res.clone())))
        .collect()
}

async fn disable_behavior_instance(id: Uuid, res: Arc<Mutex<Resources>>) {
    let upd = BehaviorInstanceUpdate::default().with_enabled(false);
    let upd_result = res
        .lock()
        .await
        .update::<BehaviorInstance>(&id, |bi| *bi += upd);
    if let Err(err) = upd_result {
        log::error!("Failed to disable behavior instance {:?}", err);
    }
}

#[derive(Debug, PartialEq)]
struct ScheduleBehaviorInstance(Uuid, BehaviorInstance);

#[derive(Debug)]
enum ScheduleType {
    Recurring(Weekday),
    Once(),
}

pub struct WakeupJob {
    resource_id: Uuid,
    schedule_type: ScheduleType,
    configuration: WakeupConfiguration,
}

impl WakeupJob {
    fn start_datetime(&self, now: DateTime<Local>) -> Result<DateTime<Local>, &'static str> {
        let start_time = self.start_time()?;
        let next = match now.with_time(start_time) {
            chrono::offset::LocalResult::Single(time) => time,
            chrono::offset::LocalResult::Ambiguous(_, latest) => latest,
            chrono::offset::LocalResult::None => {
                return Err("with time");
            }
        };
        let wakeup_datetime = if next < now {
            next.checked_add_days(Days::new(1)).ok_or("add day")?
        } else {
            next
        };
        Ok(wakeup_datetime)
    }

    fn start_time(&self) -> Result<NaiveTime, &'static str> {
        let job_time = &self.configuration.when.time_point.time;
        let scheduled_wakeup_time =
            NaiveTime::from_hms_opt(job_time.hour, job_time.minute, 0).ok_or("naive time")?;
        // although the scheduled time in the Hue app is the time when lights are at full brightness
        // the job start time is considered to be when the fade in effects starts
        let fade_in_duration = self.configuration.fade_in_duration.to_std();
        Ok(scheduled_wakeup_time - fade_in_duration)
    }

    async fn create(self, res: Arc<Mutex<Resources>>) {
        log::debug!(
            "Created new behavior instance job: {:?}",
            self.configuration
        );
        let now = Local::now();
        let result = match &self.schedule_type {
            ScheduleType::Recurring(weekday) => self.create_recurring(*weekday, res).await,
            ScheduleType::Once() => self.run_once(now, res),
        };
        if let Err(err) = result {
            log::error!("Failed to create wake up job: {}", err);
        }
    }

    async fn create_recurring(
        &self,
        weekday: Weekday,
        res: Arc<Mutex<Resources>>,
    ) -> Result<(), &'static str> {
        let fade_in_start = self.start_time()?;
        every(1)
            .week()
            .on(weekday)
            .at(
                fade_in_start.hour(),
                fade_in_start.minute(),
                fade_in_start.second(),
            )
            .perform(move || {
                let wakeup_configuration = self.configuration.clone();
                let res = res.clone();
                async move {
                    spawn(run_wake_up(wakeup_configuration.clone(), res.clone()));
                }
            })
            .await;
        Ok(())
    }

    fn run_once(
        self,
        now: DateTime<Local>,
        res: Arc<Mutex<Resources>>,
    ) -> Result<(), &'static str> {
        let fade_in_datetime = self.start_datetime(now)?;
        let time_until_fade_in = (fade_in_datetime - now).to_std().ok().ok_or("duration")?;
        spawn(async move {
            sleep(time_until_fade_in).await;
            run_wake_up(self.configuration.clone(), res.clone()).await;
            disable_behavior_instance(self.resource_id, res).await;
        });

        Ok(())
    }
}

fn create_wake_up_jobs(resource_id: &Uuid, configuration: &WakeupConfiguration) -> Vec<WakeupJob> {
    let weekdays = configuration.when.weekdays();

    let schedule_types: Box<dyn Iterator<Item = ScheduleType>> = weekdays.map_or_else(
        || Box::new(iter::once(ScheduleType::Once())) as Box<dyn Iterator<Item = ScheduleType>>,
        |weekdays| Box::new(weekdays.into_iter().map(ScheduleType::Recurring)),
    );
    schedule_types
        .map(|schedule_type| WakeupJob {
            resource_id: *resource_id,
            schedule_type,
            configuration: configuration.clone(),
        })
        .collect()
}

async fn run_wake_up(config: WakeupConfiguration, res: Arc<Mutex<Resources>>) {
    log::debug!("Running scheduled behavior instance:, {:#?}", config);
    #[allow(clippy::option_if_let_else)]
    let resource_links = config.where_field.iter().flat_map(|room| {
        if let Some(items) = &room.items {
            items.clone()
        } else {
            vec![room.group]
        }
    });

    let requests = {
        let lock = res.lock().await;
        resource_links
            .into_iter()
            .filter_map(|resource_link| {
                let resource = lock.get_resource_by_id(&resource_link.rid);
                match resource {
                    Ok(resource) => Some((resource_link, resource)),
                    Err(err) => {
                        log::warn!("Failed to get resource: {}", err);
                        None
                    }
                }
            })
            .flat_map(|(resource_link, resource)| match resource.obj {
                Resource::Room(room) => room
                    .grouped_light_service()
                    .map_or_else(Vec::new, |grouped_light| {
                        vec![WakeupRequest::Group(*grouped_light)]
                    }),
                Resource::Light(_light) => {
                    vec![WakeupRequest::Light(resource_link)]
                }
                Resource::BridgeHome(_bridge_home) => {
                    let all_rooms = lock.get_resources_by_type(RType::Room);
                    all_rooms
                        .into_iter()
                        .filter_map(|room_resource| match room_resource.obj {
                            Resource::Room(room) => {
                                let grouped_light = room.grouped_light_service()?;
                                Some(WakeupRequest::Group(*grouped_light))
                            }
                            _ => None,
                        })
                        .collect()
                }
                _ => Vec::new(),
            })
            .collect::<Vec<_>>()
    };

    for request in &requests {
        if let Err(err) = request.on(res.clone(), config.clone()).await {
            log::warn!("Failed to turn on wake up light: {}", err);
        }
    }

    // wait until fade in has completed
    // otherwise the behavior instance can be disabled before it has actually finished
    sleep(config.fade_in_duration.to_std()).await;

    if let Some(duration) = config.turn_lights_off_after {
        sleep(duration.to_std()).await;

        for request in &requests {
            if let Err(err) = request.off(res.clone()).await {
                log::warn!("Failed to turn off wake up light: {}", err);
            }
        }
    }
}

enum WakeupRequest {
    Light(ResourceLink),
    Group(ResourceLink),
}

impl WakeupRequest {
    async fn on(&self, res: Arc<Mutex<Resources>>, config: WakeupConfiguration) -> ApiResult<()> {
        let light_supports_effects = match self {
            Self::Light(resource_link) => res
                .lock()
                .await
                .get::<Light>(resource_link)?
                .effects
                .is_some(),
            Self::Group(_) => false, // todo: implement when grouped light support effects
        };
        let use_sunrise_effect =
            light_supports_effects && config.style == Some("sunrise".to_string());

        if use_sunrise_effect {
            self.sunrise_on(&res, &config).await?;
        } else {
            self.transition_to_bright_on(res, &config).await?;
        }
        Ok(())
    }

    async fn transition_to_bright_on(
        &self,
        res: Arc<Mutex<Resources>>,
        config: &WakeupConfiguration,
    ) -> Result<(), crate::error::ApiError> {
        // As reported by the Hue bridge
        const WAKEUP_FADE_MIREK: u16 = 447;

        // Reset brightness and set color temperature
        let reset_backend_request = match self {
            Self::Light(resource_link) => {
                let payload = LightUpdate::default()
                    .with_on(Some(On::new(true)))
                    .with_brightness(Some(0.0))
                    .with_color_temperature(Some(WAKEUP_FADE_MIREK));
                BackendRequest::LightUpdate(*resource_link, payload)
            }
            Self::Group(resource_link) => {
                let payload = GroupedLightUpdate::default()
                    .with_on(Some(On::new(true)))
                    .with_brightness(Some(0.0))
                    .with_color_temperature(Some(WAKEUP_FADE_MIREK));
                BackendRequest::GroupedLightUpdate(*resource_link, payload)
            }
        };
        res.lock().await.backend_request(reset_backend_request)?;

        sleep(Duration::from_secs(1)).await;

        // Start fade in to set brightness
        let on_backend_request = match self {
            Self::Light(resource_link) => {
                let payload = LightUpdate::default()
                    .with_brightness(Some(config.end_brightness))
                    .with_transition(Some(config.fade_in_duration.seconds));
                BackendRequest::LightUpdate(*resource_link, payload)
            }
            Self::Group(resource_link) => {
                let payload = GroupedLightUpdate::default()
                    .with_brightness(Some(config.end_brightness))
                    .with_transition(Some(config.fade_in_duration.seconds));
                BackendRequest::GroupedLightUpdate(*resource_link, payload)
            }
        };
        res.lock().await.backend_request(on_backend_request)?;
        Ok(())
    }

    async fn sunrise_on(
        &self,
        res: &Arc<Mutex<Resources>>,
        config: &WakeupConfiguration,
    ) -> Result<(), crate::error::ApiError> {
        match self {
            Self::Light(resource_link) => {
                let mut payload = LightUpdate::default()
                    .with_on(Some(On::new(true)))
                    .with_brightness(Some(config.end_brightness));
                let effect_duration = EffectDuration::from_seconds(config.fade_in_duration.seconds);
                payload.effects_v2 = Some(LightEffectsV2Update {
                    action: Some(LightEffectActionUpdate {
                        effect: Some(hue::api::LightEffect::Sunrise),
                        parameters: hue::api::LightEffectParameters {
                            color: None,
                            color_temperature: None,
                            speed: Some(Clamp::unit_from_u8(effect_duration.0)),
                        },
                    }),
                });
                res.lock()
                    .await
                    .backend_request(BackendRequest::LightUpdate(*resource_link, payload))?;
            }
            Self::Group(_resource_link) => {}
        };
        Ok(())
    }

    async fn off(&self, res: Arc<Mutex<Resources>>) -> ApiResult<()> {
        let backend_request = match self {
            Self::Light(resource_link) => {
                let payload = LightUpdate::default().with_on(Some(On::new(false)));
                BackendRequest::LightUpdate(*resource_link, payload)
            }
            Self::Group(resource_link) => {
                let payload = GroupedLightUpdate::default().with_on(Some(On::new(false)));
                BackendRequest::GroupedLightUpdate(*resource_link, payload)
            }
        };

        res.lock().await.backend_request(backend_request)
    }
}
