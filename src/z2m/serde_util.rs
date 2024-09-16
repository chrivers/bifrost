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

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::{from_str, to_string};

    use crate::error::ApiResult;

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
    struct Foo {
        #[serde(with = "super::struct_or_false")]
        foo: Option<Bar>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
    struct Bar {
        bar: u32,
    }

    const FOO_NONE: Foo = Foo { foo: None };
    const FOO_SOME: Foo = Foo {
        foo: Some(Bar { bar: 42 }),
    };

    const FOO_NONE_STR: &str = r#"{"foo":false}"#;
    const FOO_SOME_STR: &str = r#"{"foo":{"bar":42}}"#;
    const FOO_TRUE: &str = r#"{"foo":true}"#;
    const FOO_LIST: &str = r#"{"foo":[42]}"#;

    #[test]
    pub fn serialize_none() -> ApiResult<()> {
        assert_eq!(to_string(&FOO_NONE)?, FOO_NONE_STR);
        Ok(())
    }

    #[test]
    pub fn serialize_some() -> ApiResult<()> {
        assert_eq!(to_string(&FOO_SOME)?, FOO_SOME_STR);

        Ok(())
    }

    #[test]
    pub fn deserialize_false() -> ApiResult<()> {
        assert_eq!(from_str::<Foo>(FOO_NONE_STR)?, FOO_NONE);
        Ok(())
    }

    #[test]
    pub fn deserialize_struct() -> ApiResult<()> {
        assert_eq!(from_str::<Foo>(FOO_SOME_STR)?, FOO_SOME);
        Ok(())
    }

    #[test]
    pub fn deserialize_true() {
        /* must return error */
        from_str::<Foo>(FOO_TRUE).unwrap_err();
    }

    #[test]
    pub fn deserialize_list() {
        /* must return error */
        from_str::<Foo>(FOO_LIST).unwrap_err();
    }
}
