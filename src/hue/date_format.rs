const FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";

pub mod utc {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use serde::{self, de::Error, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(super::FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let dt = NaiveDateTime::parse_from_str(&s, super::FORMAT).map_err(Error::custom)?;
        Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
    }
}

pub mod local {
    use chrono::{DateTime, Local, NaiveDateTime};
    use serde::{self, de::Error, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Local>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(super::FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Local>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let dt = NaiveDateTime::parse_from_str(&s, super::FORMAT).map_err(Error::custom)?;
        dt.and_local_timezone(Local)
            .single()
            .ok_or_else(|| Error::custom("Localtime conversion failed"))
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};

    use crate::error::ApiResult;

    fn date() -> (&'static str, DateTime<Utc>) {
        let dt = Utc.with_ymd_and_hms(2014, 7, 8, 9, 10, 11).unwrap();
        ("\"2014-07-08T09:10:11Z\"", dt)
    }

    #[test]
    fn utc_de() -> ApiResult<()> {
        let (ds, d1) = date();

        let mut deser = serde_json::Deserializer::from_str(ds);
        let d2 = super::utc::deserialize(&mut deser)?;

        assert_eq!(d1, d2);
        Ok(())
    }

    #[test]
    fn utc_se() -> ApiResult<()> {
        let (s1, dt) = date();

        let mut s2 = vec![];
        let mut ser = serde_json::Serializer::new(&mut s2);
        super::utc::serialize(&dt, &mut ser)?;

        assert_eq!(s1.as_bytes(), s2);
        Ok(())
    }
}
