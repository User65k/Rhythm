use flexbuffers::SerializationError;
use hyper::{body::Bytes, http::request, http::response};
use rhythm_proto::{Request, Response};
use sled::{Db, Error as SledErr, IVec};
use std::convert::From;
use std::error::Error;
use std::{collections::HashMap, convert::TryInto, path::Path};

#[derive(Debug)]
pub enum DBErr {
    Sled(SledErr),
    //NoProto(NP_Error)
    ArchiveBufferError(SerializationError),
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

#[derive(Clone)]
pub struct DB {
    db: Db,
}

impl DB {
    const KEY_REQUEST_HEADER: &'static [u8] = b"req";
    const KEY_REQUEST_BODY: &'static [u8] = b"reqbod";
    const KEY_RESPONSE_HEADER: &'static [u8] = b"resp";
    const KEY_RESPONSE_BODY: &'static [u8] = b"respbod";

    pub fn new() -> Result<DB, DBErr> {
        DB::open(Path::new("/tmp/rhythm"))
    }
    pub fn open(file_name: &Path) -> Result<DB, DBErr> {
        let db = sled::open(file_name)?;
        Ok(DB { db })
    }
    pub fn save_to_disk(&self, file_name: &str) -> Result<(), DBErr> {
        Ok(self.db.flush().map(|_| ())?)
    }
    pub fn store_req(&self, parts: &request::Parts, body: &Bytes) -> Result<u64, DBErr> {
        let last_key = self.db.generate_id()?;
        let req: Request = parts.into();
        let iv: IVec = req.try_into()?;

        let req_store = self.db.open_tree(Self::KEY_REQUEST_HEADER)?;
        let req_body_store = self.db.open_tree(Self::KEY_REQUEST_BODY)?;

        req_store.insert(last_key.to_be_bytes(), iv)?;
        req_body_store.insert(last_key.to_be_bytes(), body.as_ref())?;
        Ok(last_key)
    }
    pub fn store_resp(&self, req: u64, parts: &response::Parts, body: &Bytes) -> Result<(), DBErr> {
        let resp: Response = parts.into();
        let iv: IVec = resp.try_into()?;

        let resp_store = self.db.open_tree(Self::KEY_RESPONSE_HEADER)?;
        let resp_body_store = self.db.open_tree(Self::KEY_RESPONSE_BODY)?;

        resp_store.insert(req.to_be_bytes(), iv)?;
        resp_body_store.insert(req.to_be_bytes(), body.as_ref())?;
        Ok(())
    }
    /// Get a stored Request and Response (if available)
    pub fn get_req_resp(&self, key: u64) -> Result<Option<(IVec, Option<IVec>)>, DBErr> {
        let key = key.to_be_bytes();
        let req_store = self.db.open_tree(Self::KEY_REQUEST_HEADER)?;
        let req = if let Some(req) = req_store.get(key)? {
            req
        }else{
            return Ok(None);
        };
        let resp_store = self.db.open_tree(Self::KEY_RESPONSE_HEADER)?;
        Ok(Some(if let Some(resp) = resp_store.get(key)? {
            (
                req,
                Some(resp),
            )
        }else{
            (
                req,
                None,
            )
        }))
    }
    pub fn get_req_body(&self, key: u64) -> Result<Option<IVec>, DBErr> {
        let key = key.to_be_bytes();
        Ok(self.db.open_tree(Self::KEY_REQUEST_BODY)?.get(key)?)
    }
    pub fn get_resp_body(&self, key: u64) -> Result<Option<IVec>, DBErr> {
        let key = key.to_be_bytes();
        Ok(self.db.open_tree(Self::KEY_RESPONSE_BODY)?.get(key)?)
    }
    pub fn search_req(&self) -> Result<(), ()> {
        Ok(())
    }
    pub fn search_resp(&self) -> Result<(), ()> {
        Ok(())
    }
}
