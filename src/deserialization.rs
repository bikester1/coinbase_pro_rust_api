pub(crate) mod iso_date_time {
    use chrono::NaiveDateTime;
    use serde::{
        self,
        Deserialize,
        Deserializer,
        Serializer,
    };

    // Returned format from coinbase 2022-01-20T18:38:25.055677Z
    const FORMAT: &'static str = "%Y-%m-%dT%H:%M:%S%.6fZ";

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}

pub(crate) mod option_iso_date_time {
    use chrono::NaiveDateTime;
    use serde::{
        self,
        Deserialize,
        Deserializer,
        Serializer,
    };

    // Returned format from coinbase 2022-01-20T18:38:25.055677Z
    const FORMAT: &'static str = "%Y-%m-%dT%H:%M:%S%.fZ";

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &Option<NaiveDateTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match date {
            None => "".to_string(),
            Some(date) => {
                format!("{}", date.format(FORMAT))
            }
        };
        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut s: String = match Option::deserialize(deserializer)? {
            Some(s) => s,
            None => return Ok(None),
        };
        s = s.replace("Z", "");
        let mut my_format: String = FORMAT.to_string();
        my_format.pop();
        Ok(Some(
            NaiveDateTime::parse_from_str(&s, my_format.as_str())
                .map_err(serde::de::Error::custom)?,
        ))
    }
}

pub(crate) mod transfer_date {
    use chrono::NaiveDateTime;
    use serde::{
        self,
        Deserialize,
        Deserializer,
        Serializer,
    };

    // Returned format from coinbase 2022-01-20T18:38:25.055677Z
    const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S%.f%#z";
    const FORMAT_NO_TZ: &'static str = "%Y-%m-%d %H:%M:%S%.f";

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));

        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let format = if s[s.len() - 5..].contains("+") {
            FORMAT
        } else {
            FORMAT_NO_TZ
        };
        NaiveDateTime::parse_from_str(&s, format).map_err(serde::de::Error::custom)
    }
}

pub(crate) mod string_as_float {
    use std::str::FromStr;

    use serde::{
        self,
        Deserialize,
        Deserializer,
        Serializer,
    };

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", value);
        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<f64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        f64::from_str(s.as_str())
            .map_err(|_| serde::de::Error::custom("String -> Float Parsing Error"))
    }
}

pub(crate) mod option_string_as_float {
    use std::str::FromStr;

    use serde::{
        self,
        Deserialize,
        Deserializer,
        Serializer,
    };

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(value: &Option<f64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match value {
            None => "".to_string(),
            Some(value) => {
                format!("{}", value)
            }
        };
        serializer.serialize_str(&s.as_str())
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = match Option::deserialize(deserializer)? {
            Some(s) => s,
            None => return Ok(None),
        };

        f64::from_str(s.as_str())
            .map_err(|_| serde::de::Error::custom("String -> Float Parsing Error"))
            .map(|x| Some(x))
    }
}
