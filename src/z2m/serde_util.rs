use std::any::type_name;
use std::fmt;
use std::marker::PhantomData;

use serde::de::{Deserialize, Deserializer, Unexpected};
use serde::{de, Serialize, Serializer};

pub fn deserialize_struct_or_false<'de, T, D>(d: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    // Internal wrapper struct
    struct StructOrFalse<T>(PhantomData<T>);

    impl<'de, T> de::Visitor<'de> for StructOrFalse<T>
    where
        T: Deserialize<'de>,
    {
        type Value = Option<T>;

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            /* false means `None`, true is unexpected */
            if value {
                Err(de::Error::invalid_type(Unexpected::Bool(value), &self))
            } else {
                Ok(None)
            }
        }

        fn visit_map<M>(self, visitor: M) -> Result<Self::Value, M::Error>
        where
            M: de::MapAccess<'de>,
        {
            let mvd = de::value::MapAccessDeserializer::new(visitor);
            Deserialize::deserialize(mvd).map(Some)
        }

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "false or {}", type_name::<T>())
        }
    }

    d.deserialize_any(StructOrFalse(PhantomData))
}

pub fn serialize_struct_or_false<T, S>(v: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    match v {
        None => false.serialize(serializer),
        Some(d) => d.serialize(serializer),
    }
}

pub mod struct_or_false {
    pub use super::deserialize_struct_or_false as deserialize;
    pub use super::serialize_struct_or_false as serialize;
}
