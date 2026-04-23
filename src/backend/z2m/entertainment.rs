use std::collections::BTreeMap;

use serde_json::json;

use hue::stream::HueStreamLightsV2;
use hue::zigbee::{
    EntertainmentZigbeeStream, HueEntFrameLightRecord, LightRecordMode,
    PHILIPS_HUE_ZIGBEE_VENDOR_ID,
};
use z2m::request::Z2mRequest;
use zcl::attr::ZclDataType;

use crate::backend::z2m::websocket::Z2mWebSocket;
use crate::error::ApiResult;

pub struct EntStream {
    pub stream: EntertainmentZigbeeStream,
    pub target: String,
    pub addrs: BTreeMap<String, Vec<u16>>,
    pub channels: BTreeMap<u8, (u16, LightRecordMode)>,
}

impl EntStream {
    #[must_use]
    pub fn new(
        counter: u32,
        target: &str,
        addrs: BTreeMap<String, Vec<u16>>,
        channels: BTreeMap<u8, (u16, LightRecordMode)>,
    ) -> Self {
        Self {
            stream: EntertainmentZigbeeStream::new(counter),
            target: target.to_string(),
            addrs,
            channels,
        }
    }


    #[must_use]
    pub fn z2m_set_entertainment_brightness(brightness: u8) -> Z2mRequest<'static> {
        Z2mRequest::Write {
            cluster: EntertainmentZigbeeStream::CLUSTER,
            payload: json!({
                EntertainmentZigbeeStream::CMD_LIGHT_BALANCE.to_string(): {
                    "manufacturerCode": PHILIPS_HUE_ZIGBEE_VENDOR_ID,
                    "type": ZclDataType::ZclU8 as u8,
                    "value": brightness,
                }
            }),
        }
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    #[must_use]
    pub fn generate_frame(&self, frame: &HueStreamLightsV2) -> Vec<HueEntFrameLightRecord> {
        let mut blks = vec![];
        match frame {
            HueStreamLightsV2::Rgb(rgb) => {
                for light in rgb {
                    let (xy, bright) = light.rgb.to_xy();

                    let brightness = (bright / 255.0 * 2047.0).clamp(1.0, 2047.0) as u16;
                    if let Some((chan, mode)) = self.channels.get(&light.channel) {
                        let raw = xy.to_quant();
                        let lrec = HueEntFrameLightRecord::new(*chan, brightness, *mode, raw);
                        blks.push(lrec);
                    }
                }
            }
            HueStreamLightsV2::Xy(xy) => {
                for light in xy {
                    let (xy, bright) = light.xy.to_xy();

                    let brightness = (bright / 255.0 * 2047.0).clamp(1.0, 2047.0) as u16;
                    if let Some((chan, mode)) = self.channels.get(&light.channel) {
                        let raw = xy.to_quant();
                        let lrec = HueEntFrameLightRecord::new(*chan, brightness, *mode, raw);
                        blks.push(lrec);
                    }
                }
            }
        }

        blks
    }

    pub async fn start_stream(&mut self, z2mws: &mut Z2mWebSocket) -> ApiResult<()> {
        log::debug!("Entertainment addrs: {:#?}", &self.addrs);
        log::debug!("Entertainment channels: {:#?}", &self.channels);
        for (dev, segments) in &self.addrs {
            let z2mreq = Self::z2m_set_entertainment_brightness(0xFE);
            z2mws.send(dev, &z2mreq).await?;

            if segments.len() <= 1 {
                continue;
            }

            let mapping = self.stream.segment_mapping(segments)?;
            z2mws.send_zigbee_message(dev, &mapping).await?;
        }

        self.stop_stream(z2mws).await?;

        Ok(())
    }

    pub async fn stop_stream(&mut self, z2mws: &mut Z2mWebSocket) -> ApiResult<()> {
        let stop = self.stream.reset()?;
        for topic in self.addrs.keys() {
            log::debug!("Sending stop to {topic}");
            z2mws.send_zigbee_message(topic, &stop).await?;
        }

        Ok(())
    }

    pub async fn frame(
        &mut self,
        z2mws: &mut Z2mWebSocket,
        frame: &HueStreamLightsV2,
    ) -> ApiResult<()> {
        let blks = self.generate_frame(frame);

        let message = self.stream.frame(blks)?;
        z2mws.send_entertainment_frame(&self.target, &message).await
    }
}
