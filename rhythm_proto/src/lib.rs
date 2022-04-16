#[cfg(not(target_arch = "wasm32"))]
use flexbuffers::{FlexbufferSerializer, SerializationError};
#[cfg(not(target_arch = "wasm32"))]
use serde::{de::Error as SerdeErr, Deserializer};
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use sled::IVec;
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::convert::{From, TryInto};
#[cfg(not(target_arch = "wasm32"))]
use std::fmt::Display;
#[cfg(not(target_arch = "wasm32"))]
use std::str::FromStr;

/// Data requested by the ui via REST
#[derive(Debug)]
#[cfg(not(target_arch = "wasm32"))]
#[derive(Deserialize)]
#[serde(tag = "op")]
pub enum APICall {
    ///get whole Req-Resp headers
    Headers {
        id: u64,
    },
    ///get Req-Resp for bottom list
    Brief {
        //#[serde(deserialize_with = "from_str")]
        id: u64,
    },
    /// get body of req
    ReqBody {
        id: u64,
    },
    /// get body of resp
    RespBody {
        id: u64,
    },    
       //StartApp{path: PathBuf},
       //Send{req: !},
       //Alter{id: u64, req: !},
}/*
#[cfg(not(target_arch = "wasm32"))]
fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(SerdeErr::custom)
}*/

/// Data sent over the Websocket from prx to ui
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "t")]
pub enum WSNotify {
    NewReq {
        id: u64,
        method: String,
        uri: String,
    },
    NewResp {
        id: u64,
        status: u16,
    },
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

/// HTTP Request Header stored in DB
/// 
/// Without fieldnames, as flexbuffers would include them in each dataset
#[derive(Serialize, Deserialize, Debug)]
pub struct Request (
    Method,
    //HTTP Version
    HttpVersion,
    //URI
    String,
    HashMap<String, Vec<u8>>,
);

/// HTTP Response Header stored in DB
/// 
/// Without fieldnames, as flexbuffers would include them in each dataset
#[derive(Serialize, Deserialize, Debug)]
pub struct Response (
    //Response Code
    u16,
    // The response's version
    HttpVersion,
    // The response's headers
    HashMap<String, Vec<u8>>,
);

/// HTTP Method stored in DB
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Method {
    WellKnown(u8), //one byte less then an empty vec
    Custom(Vec<u8>)
}
/// HTTP Method stored in DB
#[derive(Serialize, Deserialize, Debug)]
pub struct HttpVersion(u8);

#[cfg(not(target_arch = "wasm32"))]
impl From<&hyper::http::request::Parts> for Request {
    #[inline]
    fn from(parts: &hyper::http::request::Parts) -> Self {
        let mut headers: HashMap<String, Vec<u8>> = HashMap::new();
        for (k, v) in parts.headers.iter() {
            headers.insert(k.to_string(), v.as_bytes().into());
        }
        Request(
            (&parts.method).into(),
            parts.version.into(),
            parts.uri.to_string(),
            headers,
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<&hyper::http::response::Parts> for Response {
    #[inline]
    fn from(parts: &hyper::http::response::Parts) -> Self {
        let mut headers: HashMap<String, Vec<u8>> = HashMap::new();
        for (k, v) in parts.headers.iter() {
            headers.insert(k.to_string(), v.as_bytes().into());
        }
        Response (
            parts.status.as_u16(),
            parts.version.into(),
            headers,
        )
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

#[cfg(not(target_arch = "wasm32"))]
impl From<&hyper::Method> for Method {
    #[inline]
    fn from(method: &hyper::Method) -> Self {
        match method {
            &hyper::Method::GET     => Method::WellKnown(0),
            &hyper::Method::HEAD    => Method::WellKnown(1),
            &hyper::Method::POST    => Method::WellKnown(2),
            &hyper::Method::PUT     => Method::WellKnown(3),
            &hyper::Method::DELETE  => Method::WellKnown(4),
            &hyper::Method::CONNECT => Method::WellKnown(5),
            &hyper::Method::OPTIONS => Method::WellKnown(6),
            &hyper::Method::PATCH   => Method::WellKnown(7),
            &hyper::Method::TRACE   => Method::WellKnown(8),
            m => Method::Custom(m.as_str().as_bytes().into())
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl From<hyper::Version> for HttpVersion {
    #[inline]
    fn from(version: hyper::Version) -> Self {
        HttpVersion(match version {
            hyper::Version::HTTP_09 => 9,
            hyper::Version::HTTP_10 => 10,
            hyper::Version::HTTP_11 => 11,
            hyper::Version::HTTP_2 => 2,
            hyper::Version::HTTP_3 => 3,
            _ => 0,
        })
    }
}
