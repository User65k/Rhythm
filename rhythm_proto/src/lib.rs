#[cfg(not(target_arch = "wasm32"))]
use serde::{Deserializer, de::Error as SerdeErr};
#[cfg(not(target_arch = "wasm32"))]
use std::str::FromStr;
#[cfg(not(target_arch = "wasm32"))]
use std::fmt::Display;
#[cfg(not(target_arch = "wasm32"))]
use sled::IVec;
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use hyper::{http::request, http::response, Version};
#[cfg(not(target_arch = "wasm32"))]
use std::convert::{From, TryInto};
#[cfg(not(target_arch = "wasm32"))]
use flexbuffers::{FlexbufferSerializer, SerializationError};
use serde::{Deserialize, Serialize};


/// Data requested by the ui via REST
#[derive(Debug)]
#[cfg(not(target_arch = "wasm32"))]
#[derive(Deserialize)]
#[serde(tag = "op")]
pub enum APICall {
    Details{id: u64}, //get whole Req-Resp
    Brief{#[serde(deserialize_with = "from_str")] id: u64}, //get Req-Resp for bottom list
    //StartApp{path: PathBuf},
    //Send{req: !},
    //Alter{id: u64, req: !},
}
#[cfg(not(target_arch = "wasm32"))]
fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where T: FromStr,
          T::Err: Display,
          D: Deserializer<'de>
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(SerdeErr::custom)
}

/// Data sent over the Websocket from prx to ui
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "t")]
pub enum WSNotify {
    NewReq {id: u64, method: String, uri: String},
    NewResp{id: u64, status: u16},
}

#[cfg(target_arch = "wasm32")]
impl WSNotify {
    pub fn parse(bytes: &[u8]) -> Result<WSNotify, Box<dyn std::error::Error>> {
        let r = flexbuffers::Reader::get_root(bytes)?;
        Ok(WSNotify::deserialize(r)?)
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl WSNotify {
    pub fn as_u8(self) -> Result<Vec<u8>, SerializationError> {
        let mut s = FlexbufferSerializer::new();
        self.serialize(&mut s)?;
        Ok(s.take_buffer())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    pub method: String,
    pub uri: String,
    pub version: u8,
    pub headers: HashMap<String, Vec<u8>>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub status: u16,
    /// The response's version
    pub version: u8,
    /// The response's headers
    pub headers: HashMap<String, Vec<u8>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl From<&request::Parts> for Request {
    #[inline]
    fn from(parts: &request::Parts) -> Self {
        let version = vers_to_index(parts.version);
        let mut headers: HashMap<String, Vec<u8>> = HashMap::new();
        for (k,v) in parts.headers.iter() {
            headers.insert(k.to_string(), v.as_bytes().into());
        }
        Request {
            version, 
            uri: parts.uri.to_string(),
            method: parts.method.to_string(),
            headers
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<&response::Parts> for Response {
    #[inline]
    fn from(parts: &response::Parts) -> Self {
        let version = vers_to_index(parts.version);
        let mut headers: HashMap<String, Vec<u8>> = HashMap::new();
        for (k,v) in parts.headers.iter() {
            headers.insert(k.to_string(), v.as_bytes().into());
        }
        Response {
            status: parts.status.as_u16(),
            version,
            headers
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl TryInto<IVec> for Request {
    fn try_into(self) -> Result<IVec, SerializationError> {
        
        let mut s = FlexbufferSerializer::new();
        self.serialize(&mut s)?;
        
        Ok(s.view().into())
    }

    type Error = SerializationError;
}

#[cfg(not(target_arch = "wasm32"))]
impl TryInto<IVec> for Response {
    fn try_into(self) -> Result<IVec, SerializationError> {
        
        let mut s = FlexbufferSerializer::new();
        self.serialize(&mut s)?;
        
        Ok(s.view().into())
    }
    type Error = SerializationError;
}

#[inline]
#[cfg(not(target_arch = "wasm32"))]
fn vers_to_index(version: Version) -> u8 {
    match version {
        Version::HTTP_09 => 9,
        Version::HTTP_10 => 10,
        Version::HTTP_11 => 11,
        Version::HTTP_2 => 2,
        Version::HTTP_3 => 3,
        _ => 0
    }
}