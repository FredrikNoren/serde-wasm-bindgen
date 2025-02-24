use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{fmt, sync::Mutex};
use wasm_bindgen::JsValue;

/// This type is used to preserve a [`JsValue`] when serializing and deserializing.
///
/// ```rust
/// #[derive(Serialize, Deserialize)]
/// struct MyStruct {
///    // works with objects from wasm-bindgen; they just need to implement From<JsValue> and Into<JsValue>
///    my_value: PreserveJsValue<ObjectFromWasmBindgen>,
///    // and with raw JsValue values
///    my_other_value: PreserveJsValue<JsValue>,
/// }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct PreserveJsValue<T: From<JsValue> + Into<JsValue> + Clone>(pub T);
impl<T: From<JsValue> + Into<JsValue> + Clone> From<T> for PreserveJsValue<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}
impl<T: From<JsValue> + Into<JsValue> + Clone + Default> Default for PreserveJsValue<T> {
    fn default() -> Self {
        Self(T::default())
    }
}

#[derive(Clone)]
pub(crate) struct JsValueKeeper(pub JsValue);
unsafe impl Send for JsValueKeeper {}
unsafe impl Sync for JsValueKeeper {}

pub(crate) static NEXT_PRESERVE: Mutex<Option<JsValueKeeper>> = Mutex::new(None);

impl<T: From<JsValue> + Into<JsValue> + Clone> Serialize for PreserveJsValue<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        NEXT_PRESERVE
            .lock()
            .unwrap()
            .replace(JsValueKeeper(self.0.clone().into()));
        serializer.serialize_i64(0)
    }
}

impl<'de, T: From<JsValue> + Into<JsValue> + Clone> Deserialize<'de> for PreserveJsValue<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct JsValueVisitor<T: From<JsValue> + Into<JsValue> + Clone>(
            std::marker::PhantomData<T>,
        );

        impl<'de, T: From<JsValue> + Into<JsValue> + Clone> Visitor<'de> for JsValueVisitor<T> {
            type Value = PreserveJsValue<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct PreserveJsValue")
            }

            fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(PreserveJsValue(
                    NEXT_PRESERVE.lock().unwrap().take().unwrap().0.into(),
                ))
            }
        }

        deserializer.deserialize_i64(JsValueVisitor(std::marker::PhantomData))
    }
}
