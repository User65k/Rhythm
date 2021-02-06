use sled::{Db, Error as SledErr, IVec};
use std::{collections::HashMap, convert::TryInto, path::Path};
use hyper::{http::request, http::response, body::Bytes, Error as HyperErr};
use std::error::Error;
use hyper::{Method, Version, StatusCode};
use std::convert::From;
use flexbuffers::{FlexbufferSerializer, SerializationError};
use serde::{Deserialize, Serialize};
#[derive(Debug)]
pub enum DBErr {
    Sled(SledErr),
    //NoProto(NP_Error)
    ArchiveBufferError(SerializationError)
}
impl From<SledErr> for DBErr {
    fn from(error: SledErr) -> Self {
        DBErr::Sled(error)
    }
}
impl From<SerializationError> for DBErr {
    fn from(error: SerializationError) -> Self {
        DBErr::ArchiveBufferError(error)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    pub method: String,
    pub uri: String,
    pub version: u8,
    pub headers: HashMap<String, Vec<u8>>,
}
#[derive(Serialize, Deserialize, Debug)]
struct Response {
    pub status: u16,
    /// The response's version
    pub version: u8,
    /// The response's headers
    pub headers: HashMap<String, Vec<u8>>,
}

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
impl TryInto<IVec> for Request {
    fn try_into(self) -> Result<IVec, SerializationError> {
        
        let mut s = flexbuffers::FlexbufferSerializer::new();
        self.serialize(&mut s)?;
        
        Ok(s.view().into())
    }

    type Error = SerializationError;
}
impl TryInto<IVec> for Response {
    fn try_into(self) -> Result<IVec, SerializationError> {
        
        let mut s = flexbuffers::FlexbufferSerializer::new();
        self.serialize(&mut s)?;
        
        Ok(s.view().into())
    }
    type Error = SerializationError;
}

pub struct DB {
    db: Db
}
impl DB {
    pub fn new() -> Result<DB,DBErr> {
        DB::open(Path::new("/tmp/rhythm"))
    }
    pub fn open(file_name: &Path) -> Result<DB,DBErr> {
        let db = sled::open(file_name)?;
        Ok(DB {
            db
        })
    }
    pub fn save_to_disk(&mut self, file_name: &str) -> Result<(),DBErr> {
        Ok(self.db.flush().map(|_|())?)
    }
    pub fn store_req(&mut self, parts: &request::Parts, body: &Bytes) -> Result<u64, DBErr> {
        let last_key = self.db.generate_id()?;
        let req: Request = parts.into();
        let iv: IVec = req.try_into()?;

        let req_store = self.db.open_tree(b"req")?;
        let req_body_store = self.db.open_tree(b"reqbod")?;

        req_store.insert(last_key.to_be_bytes(), iv)?;
        req_body_store.insert(last_key.to_be_bytes(), body.as_ref())?;
        Ok(last_key)
    }
    pub fn store_resp(&mut self, req: u64, parts: &response::Parts, body: &Bytes) -> Result<(), DBErr> {
        let resp: Response = parts.into();
        let iv: IVec = resp.try_into()?;
        
        let resp_store = self.db.open_tree(b"resp")?;
        let resp_body_store = self.db.open_tree(b"respbod")?;

        resp_store.insert(req.to_be_bytes(), iv)?;
        resp_body_store.insert(req.to_be_bytes(), body.as_ref())?;
        Ok(())
    }
    pub fn get_req_resp(&self) -> Result<(),()> {
        Ok(())
    }
    pub fn search_req(&self) -> Result<(),()> {
        Ok(())
    }
    pub fn search_resp(&self) -> Result<(),()> {
        Ok(())
    }
}

#[inline]
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