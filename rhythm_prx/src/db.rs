use sled::{Db, Error as SledErr, IVec};
use std::{collections::HashMap, convert::TryInto, path::Path};
use std::error::Error;
use std::convert::From;
use flexbuffers::SerializationError;
use rhythm_proto::{Request, Response};
use hyper::{http::request, http::response, body::Bytes};


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

