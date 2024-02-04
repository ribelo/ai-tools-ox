pub mod tools;

use std::collections::HashMap;

pub trait Jsonify {
    fn jsonify() -> serde_json::Value;
}

macro_rules! impl_jsonify {
    ( $( $( $t:ty )|+ => $result:expr ),* $(,)? ) => {
        $(
            $(
                impl Jsonify for $t {
                    fn jsonify() -> serde_json::Value {
                        serde_json::Value::String($result.to_string())
                    }
                }
            )*
        )*
    };
}

impl_jsonify!(
    &str | String | char => "string",
    i8 | i16 | i32 | i64 | i128 => "number",
    u8 | u16 | u32 | u64 | u128 => "number",
    f32 | f64 => "number",
    bool => "boolean",
);

impl<T: Jsonify> Jsonify for Vec<T> {
    fn jsonify() -> serde_json::Value {
        serde_json::Value::String(format!("{}[]", <T>::jsonify().as_str().unwrap()))
    }
}

impl<K: Jsonify, V: Jsonify> Jsonify for HashMap<K, V> {
    fn jsonify() -> serde_json::Value {
        serde_json::Value::String(format!(
            "Map<{}, {}>",
            <K>::jsonify().as_str().unwrap(),
            <V>::jsonify().as_str().unwrap()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_tools_ox_derive::Object as JsonifyObject;
    #[test]
    fn test_jsonify() {
        assert_eq!(
            String::jsonify(),
            serde_json::Value::String("string".to_string())
        );
        assert_eq!(
            i32::jsonify(),
            serde_json::Value::String("number".to_string())
        );
        assert_eq!(
            Vec::<i32>::jsonify(),
            serde_json::Value::String("number[]".to_string())
        );
        assert_eq!(
            HashMap::<String, String>::jsonify(),
            serde_json::Value::String("Map<string, string>".to_string())
        );

        #[allow(dead_code)]
        #[derive(JsonifyObject)]
        struct Foo {
            #[description(description = "a is some number")]
            a: i32,
            b: String,
            c: Vec<f32>,
        }
        println!("{}", serde_json::to_string_pretty(&Foo::jsonify()).unwrap())
    }
}
